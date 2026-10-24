use crate::file_read::{MAX_DOCUMENT_BYTES, TIMEOUT, read_regular_file, run_with_deadline, time_left};
use cap_std::fs::Dir;
use gltf::buffer::{Data, Source};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

// Limits include the four-byte padding added by the glTF importer. The main
// document shares the common document limit; buffers get their own budget.
const LIMITS: BufferLimits = BufferLimits {
    per_buffer: 256 * 1024 * 1024,
    total: 512 * 1024 * 1024,
    timeout: TIMEOUT,
};

#[derive(Clone, Copy)]
struct BufferLimits {
    per_buffer: usize,
    total: usize,
    timeout: Duration,
}

/// Reads the document without trusting a named pipe, device, or file size.
pub(super) fn read_document(path: &Path) -> Result<Vec<u8>, String> {
    crate::file_read::read_document(path, MAX_DOCUMENT_BYTES)
}

/// Directory lookup can itself block on network or userspace filesystems.
pub(super) fn open_directory(path: &Path) -> Result<Dir, String> {
    let path = path.to_owned();
    let deadline = Instant::now() + TIMEOUT;
    run_with_deadline(deadline, move || {
        Dir::open_ambient_dir(path, cap_std::ambient_authority()).map_err(|e| e.to_string())
    })
}

fn padded_length(length: usize) -> Result<usize, String> {
    length
        .checked_add(3)
        .map(|length| length & !3)
        .ok_or_else(|| "glTF buffer length overflows".to_owned())
}

fn check_length(length: usize, limit: usize) -> Result<usize, String> {
    let padded = padded_length(length)?;
    if padded > limit {
        return Err(format!("glTF buffer exceeds the {limit}-byte limit"));
    }
    Ok(padded)
}

fn check_data_uri(uri: &str, limit: usize) -> Result<(), String> {
    // Match gltf's supported data URI forms, including its bare base64 form.
    // Check the decoded size before asking the dependency to allocate it.
    let rest = &uri["data:".len()..];
    let encoded = rest.split(";base64,").nth(1).unwrap_or(rest);
    let padding = encoded
        .bytes()
        .rev()
        .take(2)
        .take_while(|b| *b == b'=')
        .count();
    let decoded = (encoded.len() / 4 * 3 + encoded.len() % 4 * 3 / 4).saturating_sub(padding);
    check_length(decoded, limit)?;
    Ok(())
}

/// Imports every buffer without granting document URIs ambient filesystem access.
pub(super) fn import_buffers(
    document: &gltf::Document,
    base: &Dir,
    blob: Option<Vec<u8>>,
) -> Result<Vec<Data>, String> {
    import_buffers_with_limits(document, base, blob, LIMITS)
}

fn import_buffers_with_limits(
    document: &gltf::Document,
    base: &Dir,
    mut blob: Option<Vec<u8>>,
    limits: BufferLimits,
) -> Result<Vec<Data>, String> {
    let deadline = Instant::now() + limits.timeout;
    let mut declared = 0;
    for buffer in document.buffers() {
        time_left(deadline)?;
        declared += check_length(
            buffer.length(),
            limits.per_buffer.min(limits.total - declared),
        )?;
    }
    // Cloning the capability does not re-resolve its path. Timed-out workers
    // retain their own handle while the caller is free to return.
    let base = Arc::new(base.try_clone().map_err(|e| e.to_string())?);
    let mut buffers = Vec::new();
    let mut total = 0;
    for buffer in document.buffers() {
        time_left(deadline)?;
        let limit = limits.per_buffer.min(limits.total - total);
        let data = match buffer.source() {
            Source::Uri(uri) if !uri.starts_with("data:") => {
                // Decode once and read this exact path. In particular, do not
                // hand the original URI back to gltf's unrestricted importer.
                let decoded =
                    urlencoding::decode(uri).map_err(|e| format!("Invalid buffer URI: {e}"))?;
                let path = Path::new(decoded.as_ref());
                // Check Windows path syntax on every host as well as native
                // absolute paths. Colons also exclude schemes and NTFS streams.
                if decoded.is_empty() || decoded.contains([':', '\\', '\0']) || path.is_absolute() {
                    return Err(format!(
                        "Invalid buffer URI {uri:?}: expected a relative path within the glTF directory"
                    ));
                }

                let base = Arc::clone(&base);
                let path = path.to_owned();
                let mut bytes = run_with_deadline(deadline, move || {
                    let mut options = cap_std::fs::OpenOptions::new();
                    options.read(true);
                    #[cfg(unix)]
                    {
                        use cap_std::fs::OpenOptionsExt;
                        options.custom_flags(libc::O_NONBLOCK);
                    }
                    // Dir resolves beneath the held directory handle, including
                    // symlinks. Nonblocking opens prevent waiting for FIFO peers.
                    let file = base.open_with(path, &options).map_err(|e| e.to_string())?;
                    read_regular_file(file.into_std(), limit, deadline)
                })
                .map_err(|e| {
                    format!("Cannot read buffer URI {uri:?} within the glTF directory: {e}")
                })?;
                // Preserve gltf::import_buffers' padding and length semantics.
                let padded = check_length(bytes.len(), limit)?;
                bytes.resize(padded, 0);
                Data(bytes)
            }
            source => {
                match source {
                    Source::Uri(uri) => check_data_uri(uri, limit)?,
                    Source::Bin => {
                        if let Some(bytes) = &blob {
                            check_length(bytes.len(), limit)?;
                        }
                    }
                }
                // With no filesystem base, the dependency can only decode
                // embedded data or consume the GLB BIN chunk.
                Data::from_source_and_blob(source, None, &mut blob).map_err(|e| e.to_string())?
            }
        };
        time_left(deadline)?;
        total += check_length(data.len(), limit)?;
        if data.len() < buffer.length() {
            return Err(gltf::Error::BufferLength {
                buffer: buffer.index(),
                expected: buffer.length(),
                actual: data.len(),
            }
            .to_string());
        }
        buffers.push(data);
    }
    Ok(buffers)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn import_small(json: &str, blob: Option<Vec<u8>>) -> Result<Vec<Data>, String> {
        let document = gltf::Gltf::from_slice(json.as_bytes()).unwrap().document;
        let base = Dir::open_ambient_dir(".", cap_std::ambient_authority()).unwrap();
        import_buffers_with_limits(
            &document,
            &base,
            blob,
            BufferLimits {
                per_buffer: 8,
                total: 12,
                timeout: Duration::from_secs(5),
            },
        )
    }

    #[test]
    fn embedded_decoded_size_is_limited_before_import() {
        let error = import_small(
            r#"{"asset":{"version":"2.0"},"buffers":[{"byteLength":1,"uri":"data:application/octet-stream;base64,AAAAAAAAAAAA"}]}"#,
            None,
        ).unwrap_err();
        assert!(error.contains("8-byte limit"), "{error}");
    }

    #[test]
    fn aggregate_limit_counts_actual_decoded_lengths_and_padding() {
        let error = import_small(
            r#"{"asset":{"version":"2.0"},"buffers":[{"byteLength":1,"uri":"data:AAAAAAAA"},{"byteLength":1,"uri":"data:AAAAAAAA"}]}"#,
            None,
        ).unwrap_err();
        assert!(error.contains("4-byte limit"), "{error}");
    }

    #[test]
    fn glb_bin_actual_size_is_limited() {
        let error = import_small(
            r#"{"asset":{"version":"2.0"},"buffers":[{"byteLength":1}]}"#,
            Some(vec![0; 9]),
        )
        .unwrap_err();
        assert!(error.contains("8-byte limit"), "{error}");
    }

    #[test]
    fn embedded_buffers_keep_padding_and_supported_uri_forms() {
        for uri in [
            "data:AAAAAAA=",
            "data:application/octet-stream;base64,AAAAAAA=",
        ] {
            let json = format!(
                r#"{{"asset":{{"version":"2.0"}},"buffers":[{{"byteLength":5,"uri":"{uri}"}}]}}"#
            );
            let buffers = import_small(&json, None).unwrap();
            assert_eq!(buffers[0].len(), 8);
        }
    }

}
