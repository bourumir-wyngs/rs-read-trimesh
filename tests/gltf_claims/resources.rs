use super::{Fixture, Value, json, load, triangle};
use std::fs;
use std::path::{Path, PathBuf};

/// Extra fixture paths are removed in reverse declaration order, including when
/// an assertion fails. Only paths explicitly created by these tests are owned.
struct OwnedPath(PathBuf);

impl Drop for OwnedPath {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0).or_else(|_| fs::remove_dir(&self.0));
    }
}

fn assert_triangle(fixture: &Fixture) {
    let mesh = load(fixture).expect("contained or embedded triangle buffer must load");
    assert_triangle_mesh(&mesh);
}

fn assert_triangle_mesh(mesh: &super::TriMesh) {
    assert_eq!(mesh.vertices().len(), 3);
    assert_eq!(mesh.indices(), &[[0, 1, 2]]);
    for (vertex, expected) in
        mesh.vertices()
            .iter()
            .zip([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]])
    {
        assert_eq!([vertex.x, vertex.y, vertex.z], expected);
    }
}

fn glb_fixture(document: &Value, buffer: Option<&[u8]>) -> (Fixture, OwnedPath) {
    let mut fixture = Fixture::new(document, &[]);
    fixture.path = fixture.directory.join("scene.glb");
    let glb_path = OwnedPath(fixture.path.clone());
    let glb = gltf::binary::Glb {
        header: gltf::binary::Header {
            magic: *b"glTF",
            version: 2,
            length: 0, // The serializer calculates the length, including padding.
        },
        json: gltf::json::serialize::to_vec(document).unwrap().into(),
        bin: buffer.map(Into::into),
    };
    fs::write(&fixture.path, glb.to_vec().unwrap()).unwrap();
    (fixture, glb_path)
}

fn percent_encode(text: &str) -> String {
    text.bytes().map(|byte| format!("%{byte:02X}")).collect()
}

fn rejects_outside_buffer(make_uri: impl FnOnce(&Path) -> String) {
    let (mut document, buffer) = triangle();
    let outside = Fixture::new(&document, &buffer);
    let uri = make_uri(&outside.directory.join("buffer.bin"));
    document["buffers"][0]["uri"] = uri.clone().into();
    let fixture = Fixture::new(&document, &buffer);

    // The outside file contains valid geometry: accepting this URI would produce
    // a mesh, so rejection cannot be caused by missing or malformed data.
    assert!(load(&fixture).is_err(), "outside URI was accepted: {uri}");
}

#[test]
fn raw_absolute_buffer_uri_is_rejected() {
    rejects_outside_buffer(|path| path.to_str().unwrap().to_owned());
}

#[test]
fn encoded_absolute_buffer_uri_is_rejected() {
    rejects_outside_buffer(|path| percent_encode(path.to_str().unwrap()));
}

#[test]
fn parent_traversal_buffer_uri_is_rejected() {
    rejects_outside_buffer(|path| {
        format!(
            "../{}/buffer.bin",
            path.parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
        )
    });
}

#[test]
fn encoded_parent_traversal_buffer_uri_is_rejected() {
    rejects_outside_buffer(|path| {
        percent_encode(&format!(
            "../{}/buffer.bin",
            path.parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
        ))
    });
}

#[test]
fn file_scheme_buffer_uri_is_rejected() {
    rejects_outside_buffer(|path| format!("file:{}", path.display()));
}

#[test]
fn file_slashes_scheme_buffer_uri_is_rejected() {
    rejects_outside_buffer(|path| format!("file://{}", path.display()));
}

#[test]
fn unreferenced_second_buffer_cannot_read_outside_directory() {
    let (mut document, buffer) = triangle();
    let outside = Fixture::new(&document, &buffer);
    let mut extra = json(r#"{"byteLength":42}"#);
    extra["uri"] = outside
        .directory
        .join("buffer.bin")
        .to_str()
        .unwrap()
        .into();
    document["buffers"].as_array_mut().unwrap().push(extra);
    let fixture = Fixture::new(&document, &buffer);

    assert!(
        load(&fixture).is_err(),
        "every declared buffer must be confined"
    );
}

#[test]
fn glb_external_buffer_cannot_read_outside_directory() {
    let (mut document, buffer) = triangle();
    let outside = Fixture::new(&document, &buffer);
    document["buffers"][0]["uri"] = outside
        .directory
        .join("buffer.bin")
        .to_str()
        .unwrap()
        .into();
    let (fixture, _glb_path) = glb_fixture(&document, None);

    assert!(
        load(&fixture).is_err(),
        "GLB external buffers must also be confined"
    );
}

#[cfg(unix)]
#[test]
fn file_symlink_to_outside_buffer_is_rejected() {
    let (mut document, buffer) = triangle();
    let outside = Fixture::new(&document, &buffer);
    document["buffers"][0]["uri"] = "linked.bin".into();
    let fixture = Fixture::new(&document, &buffer);
    let link = OwnedPath(fixture.directory.join("linked.bin"));
    std::os::unix::fs::symlink(outside.directory.join("buffer.bin"), &link.0).unwrap();

    assert!(
        load(&fixture).is_err(),
        "file symlinks must not escape the model directory"
    );
}

#[cfg(unix)]
#[test]
fn relative_symlink_to_outside_buffer_is_rejected() {
    let (mut document, buffer) = triangle();
    let outside = Fixture::new(&document, &buffer);
    document["buffers"][0]["uri"] = "linked.bin".into();
    let fixture = Fixture::new(&document, &buffer);
    let link = OwnedPath(fixture.directory.join("linked.bin"));
    let target = Path::new("..")
        .join(outside.directory.file_name().unwrap())
        .join("buffer.bin");
    std::os::unix::fs::symlink(target, &link.0).unwrap();

    assert!(
        load(&fixture).is_err(),
        "relative symlink targets must not escape the model directory"
    );
}

#[cfg(unix)]
#[test]
fn directory_symlink_to_outside_buffer_is_rejected() {
    let (mut document, buffer) = triangle();
    let outside = Fixture::new(&document, &buffer);
    document["buffers"][0]["uri"] = "linked/buffer.bin".into();
    let fixture = Fixture::new(&document, &buffer);
    let link = OwnedPath(fixture.directory.join("linked"));
    std::os::unix::fs::symlink(&outside.directory, &link.0).unwrap();

    assert!(
        load(&fixture).is_err(),
        "directory symlinks must not escape the model directory"
    );
}

#[test]
fn bare_model_filename_loads_relative_buffer_successfully() {
    const CHILD: &str = "RS_READ_TRIMESH_GLTF_BARE_FILENAME_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let mesh = super::load_trimesh_with_flags("scene.gltf", 1.0, super::TriMeshFlags::empty())
            .expect("a bare model filename must resolve sidecars from the current directory");
        assert_triangle_mesh(&mesh);
        fs::write("ready", b"ready").unwrap();
        return;
    }

    let (document, buffer) = triangle();
    let fixture = Fixture::new(&document, &buffer);
    // The child owns its working directory; parallel tests keep their own cwd.
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "resources::bare_model_filename_loads_relative_buffer_successfully",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .current_dir(&fixture.directory)
        .output()
        .expect("start bare model filename subprocess");
    assert!(
        output.status.success(),
        "bare model filename subprocess failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        fixture.directory.join("ready").exists(),
        "child test must run"
    );
}

#[test]
fn nested_relative_buffer_loads_successfully() {
    let (mut document, buffer) = triangle();
    document["buffers"][0]["uri"] = "nested/mesh.bin".into();
    let fixture = Fixture::new(&document, &buffer);
    let nested = OwnedPath(fixture.directory.join("nested"));
    fs::create_dir(&nested.0).unwrap();
    let sidecar = OwnedPath(nested.0.join("mesh.bin"));
    fs::write(&sidecar.0, &buffer).unwrap();

    assert_triangle(&fixture);
}

#[test]
fn encoded_space_in_buffer_filename_loads_successfully() {
    let (mut document, buffer) = triangle();
    document["buffers"][0]["uri"] = "mesh%20data.bin".into();
    let fixture = Fixture::new(&document, &buffer);
    let sidecar = OwnedPath(fixture.directory.join("mesh data.bin"));
    fs::write(&sidecar.0, &buffer).unwrap();

    assert_triangle(&fixture);
}

#[test]
fn buffer_uri_is_percent_decoded_exactly_once() {
    let (mut document, buffer) = triangle();
    document["buffers"][0]["uri"] = "%2562uffer.bin".into();
    let fixture = Fixture::new(&document, &buffer);
    let sidecar = OwnedPath(fixture.directory.join("%62uffer.bin"));
    fs::write(&sidecar.0, &buffer).unwrap();
    // A second decoding would select buffer.bin. Make that alternative invalid
    // so successful loading identifies the file that was actually opened.
    fs::write(fixture.directory.join("buffer.bin"), []).unwrap();

    assert_triangle(&fixture);
}

#[test]
fn dot_and_contained_parent_components_load_successfully() {
    let (mut document, buffer) = triangle();
    document["buffers"][0]["uri"] = "./nested/../buffer.bin".into();
    let fixture = Fixture::new(&document, &buffer);
    let nested = OwnedPath(fixture.directory.join("nested"));
    fs::create_dir(&nested.0).unwrap();

    assert_triangle(&fixture);
}

#[cfg(unix)]
#[test]
fn contained_relative_symlink_loads_successfully() {
    let (mut document, buffer) = triangle();
    document["buffers"][0]["uri"] = "nested/linked.bin".into();
    let fixture = Fixture::new(&document, &buffer);
    let nested = OwnedPath(fixture.directory.join("nested"));
    fs::create_dir(&nested.0).unwrap();
    let link = OwnedPath(nested.0.join("linked.bin"));
    std::os::unix::fs::symlink("../buffer.bin", &link.0).unwrap();

    assert_triangle(&fixture);
}

#[test]
fn embedded_base64_buffer_loads_successfully() {
    let (mut document, _) = triangle();
    document["buffers"][0]["uri"] = concat!(
        "data:application/octet-stream;base64,",
        "AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAAAAABAAIA"
    )
    .into();
    let fixture = Fixture::new(&document, &[]);
    fs::remove_file(fixture.directory.join("buffer.bin")).unwrap();

    assert_triangle(&fixture);
}

#[test]
fn embedded_glb_bin_loads_successfully() {
    let (mut document, buffer) = triangle();
    document["buffers"][0]
        .as_object_mut()
        .unwrap()
        .remove("uri");
    let (fixture, _glb_path) = glb_fixture(&document, Some(&buffer));
    fs::remove_file(fixture.directory.join("buffer.bin")).unwrap();

    assert_triangle(&fixture);
}

#[test]
fn unsupported_schemes_and_windows_paths_are_rejected() {
    for uri in [
        "https://example.invalid/buffer.bin",
        "ftp://example.invalid/buffer.bin",
        "C:/buffer.bin",
        "C:buffer.bin",
        r"C:\buffer.bin",
        r"\\server\share\buffer.bin",
        "C%3A%2Fbuffer.bin",
        "%5C%5Cserver%5Cshare%5Cbuffer.bin",
    ] {
        let (mut document, buffer) = triangle();
        document["buffers"][0]["uri"] = uri.into();
        let fixture = Fixture::new(&document, &buffer);
        assert!(
            load(&fixture).is_err(),
            "unsupported URI was accepted: {uri}"
        );
    }
}

#[cfg(unix)]
#[test]
fn backslashes_are_rejected_even_when_the_literal_filename_exists() {
    for uri in [r"nested\buffer.bin", "nested%5Cbuffer.bin"] {
        let (mut document, buffer) = triangle();
        document["buffers"][0]["uri"] = uri.into();
        let fixture = Fixture::new(&document, &buffer);
        let sidecar = OwnedPath(fixture.directory.join(r"nested\buffer.bin"));
        fs::write(&sidecar.0, &buffer).unwrap();
        assert!(load(&fixture).is_err(), "backslash URI was accepted: {uri}");
    }
}

#[cfg(unix)]
#[test]
fn encoded_windows_drive_prefix_is_rejected_when_literal_filename_exists() {
    let (mut document, buffer) = triangle();
    document["buffers"][0]["uri"] = "C%3Abuffer.bin".into();
    let fixture = Fixture::new(&document, &buffer);
    let sidecar = OwnedPath(fixture.directory.join("C:buffer.bin"));
    fs::write(&sidecar.0, &buffer).unwrap();

    assert!(
        load(&fixture).is_err(),
        "decoded Windows drive prefixes must be rejected"
    );
}
