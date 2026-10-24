use cap_std::fs::Dir;
use gltf::buffer::{Data, Source};
use std::path::Path;

/// Imports every buffer without granting document URIs ambient filesystem access.
pub(super) fn import_buffers(
    document: &gltf::Document,
    base: &Dir,
    mut blob: Option<Vec<u8>>,
) -> Result<Vec<Data>, String> {
    let mut buffers = Vec::new();
    for buffer in document.buffers() {
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

                // Dir resolves and opens beneath the held directory handle,
                // including when following symlinks. A canonicalize/check/read
                // sequence would leave a filesystem race between check and use.
                let mut bytes = base.read(path).map_err(|e| {
                    format!("Cannot read buffer URI {uri:?} within the glTF directory: {e}")
                })?;
                // Preserve gltf::import_buffers' padding and length semantics.
                while bytes.len() % 4 != 0 {
                    bytes.push(0);
                }
                Data(bytes)
            }
            source => {
                // With no filesystem base, the dependency can only decode
                // embedded data or consume the GLB BIN chunk.
                Data::from_source_and_blob(source, None, &mut blob).map_err(|e| e.to_string())?
            }
        };
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
