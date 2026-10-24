use super::{TriMeshFlags, load_trimesh, load_trimesh_with_flags, verify_trimesh_content};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

const COLLADA: &str = include_str!("../sample_files/collada.dae");
const VERTEX_INPUT: &str =
    "<input semantic=\"VERTEX\" source=\"#triangle_vertices\" offset=\"0\"/>";

struct Fixture {
    directory: PathBuf,
    path: PathBuf,
}

impl Fixture {
    fn new(extension: &str, contents: &str) -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let directory = loop {
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let candidate = std::env::temp_dir().join(format!(
                "rs-read-trimesh-validation-{}-{id}",
                std::process::id()
            ));
            match fs::create_dir(&candidate) {
                Ok(()) => break candidate,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("could not create fixture directory: {error}"),
            }
        };
        let fixture = Self {
            path: directory.join(format!("mesh.{extension}")),
            directory,
        };
        fs::write(&fixture.path, contents).unwrap();
        fixture
    }

    fn path_str(&self) -> &str {
        self.path.to_str().expect("UTF-8 fixture path")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn assert_rejected(extension: &str, contents: &str) {
    let fixture = Fixture::new(extension, contents);
    assert_fixture_rejected(&fixture);
}

fn assert_fixture_rejected(fixture: &Fixture) {
    assert!(
        load_trimesh(fixture.path_str(), 1.0).is_err(),
        "default flags must reject invalid geometry"
    );
    assert!(
        load_trimesh_with_flags(fixture.path_str(), 1.0, TriMeshFlags::empty()).is_err(),
        "empty flags must reject invalid geometry"
    );
}

fn assert_triangle(contents: &str) {
    let fixture = Fixture::new("dae", contents);
    for result in [
        load_trimesh(fixture.path_str(), 1.0),
        load_trimesh_with_flags(fixture.path_str(), 1.0, TriMeshFlags::empty()),
    ] {
        let mesh = result.expect("valid COLLADA input layout must load");
        assert!(verify_trimesh_content(&mesh));
    }
}

fn collada(inputs: &str, count: &str, indices: Option<&str>) -> String {
    let start = COLLADA.find("<triangles ").unwrap();
    let end = COLLADA.find("</triangles>").unwrap() + "</triangles>".len();
    let data = indices.map_or_else(String::new, |indices| format!("<p>{indices}</p>"));
    format!(
        "{}<triangles count=\"{count}\">{inputs}{data}</triangles>{}",
        &COLLADA[..start],
        &COLLADA[end..]
    )
}

fn ply(indices: Option<&str>) -> String {
    let face_count = usize::from(indices.is_some());
    let face = indices.map_or_else(String::new, |indices| format!("3 {indices}\n"));
    format!(
        "ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\n\
         property float z\nelement face {face_count}\n\
         property list uchar uint vertex_indices\nend_header\n\
         0 0 0\n1 0 0\n0 1 0\n{face}"
    )
}

#[test]
fn ply_out_of_range_indices_return_an_error() {
    for invalid in [3, u32::MAX] {
        assert_rejected("ply", &ply(Some(&format!("0 1 {invalid}"))));
    }
}

#[test]
fn collada_out_of_range_indices_return_an_error() {
    for invalid in [3, u32::MAX] {
        assert_rejected(
            "dae",
            &collada(VERTEX_INPUT, "1", Some(&format!("0 1 {invalid}"))),
        );
    }
}

#[test]
fn collada_indices_cannot_reference_vertices_in_another_geometry() {
    let start = COLLADA.find("<geometry ").unwrap();
    let end = COLLADA.find("</geometry>").unwrap() + "</geometry>".len();
    let second_geometry = COLLADA[start..end].replace("triangle_", "second_");
    // Index 3 is outside the first mesh, but inside the merged six-vertex mesh.
    let first_geometry = collada(VERTEX_INPUT, "1", Some("0 1 3"));
    assert_rejected(
        "dae",
        &first_geometry.replace(
            "</library_geometries>",
            &format!("{second_geometry}</library_geometries>"),
        ),
    );
}

#[test]
fn collada_invalid_second_geometry_indices_do_not_overflow_when_merged() {
    let invalid = collada(VERTEX_INPUT, "1", Some("0 1 4294967295"));
    let start = invalid.find("<geometry ").unwrap();
    let end = invalid.find("</geometry>").unwrap() + "</geometry>".len();
    let second_geometry = invalid[start..end].replace("triangle_", "second_");
    assert_rejected(
        "dae",
        &COLLADA.replace(
            "</library_geometries>",
            &format!("{second_geometry}</library_geometries>"),
        ),
    );
}

#[test]
fn documents_without_triangles_return_an_error() {
    assert_rejected("ply", &ply(None));
    assert_rejected("obj", "v 0 0 0\nv 1 0 0\nv 0 1 0\n");
    assert_rejected("dae", &collada(VERTEX_INPUT, "0", None));
}

#[test]
fn collada_sparse_input_offsets_preserve_geometry() {
    let inputs = format!(
        "{}<input semantic=\"NORMAL\" source=\"#triangle_positions\" offset=\"0\"/>",
        VERTEX_INPUT.replace("offset=\"0\"", "offset=\"2\"")
    );
    assert_triangle(&collada(&inputs, "1", Some("0 0 0 1 0 1 2 0 2")));
}

#[test]
fn collada_shared_input_offsets_preserve_geometry() {
    let inputs = format!(
        "{VERTEX_INPUT}<input semantic=\"NORMAL\" source=\"#triangle_positions\" offset=\"0\"/>"
    );
    assert_triangle(&collada(&inputs, "1", Some("0 1 2")));
}

#[test]
fn collada_extreme_input_offsets_return_an_error() {
    let inputs = VERTEX_INPUT.replace("offset=\"0\"", "offset=\"4294967295\"");
    assert_rejected("dae", &collada(&inputs, "1", Some("0 1 2")));
}

#[test]
fn collada_custom_technique_payloads_remain_opaque() {
    let extension = r#"<extra><technique profile="custom"><input offset="center"/><triangles count="many"/><float_array count="custom"/></technique></extra>"#;
    let document = COLLADA.replace("</COLLADA>", &format!("{extension}</COLLADA>"));
    assert!(dae_parser::Document::from_reader(document.as_bytes()).is_ok());
    assert_triangle(&document);
}

#[test]
fn collada_source_extensions_do_not_hide_following_numeric_errors() {
    let document = COLLADA.replace(
        "</source>",
        r#"<technique profile="custom"><input offset="center"/></technique></source>"#,
    );
    assert_triangle(&document);
    assert_rejected(
        "dae",
        &document.replace(
            VERTEX_INPUT,
            &VERTEX_INPUT.replace("offset=\"0\"", "offset=\"4294967295\""),
        ),
    );
}

#[test]
fn collada_huge_triangle_counts_without_indices_return_an_error() {
    for count in ["1000000000".to_string(), usize::MAX.to_string()] {
        assert_rejected("dae", &collada(VERTEX_INPUT, &count, None));
    }
}

#[test]
fn collada_huge_triangle_counts_with_indices_return_an_error() {
    for count in ["1000000000".to_string(), usize::MAX.to_string()] {
        assert_rejected("dae", &collada(VERTEX_INPUT, &count, Some("0 1 2")));
    }
}

#[test]
fn collada_triangle_count_mismatch_returns_an_error() {
    for count in ["0", "2"] {
        assert_rejected("dae", &collada(VERTEX_INPUT, count, Some("0 1 2")));
    }
}

#[test]
fn collada_truncated_input_tuple_returns_an_error() {
    let inputs = format!(
        "{VERTEX_INPUT}<input semantic=\"NORMAL\" source=\"#triangle_positions\" offset=\"1\"/>"
    );
    assert_rejected("dae", &collada(&inputs, "1", Some("0 0 1 1 2")));
}

#[test]
fn collada_attribute_limit_accepts_boundary_and_rejects_excess() {
    let attributes: String = (0..127).map(|i| format!(" a{i}=\"value\"")).collect();
    // The count attribute plus 127 extra attributes reaches the 128 limit.
    let at_limit = COLLADA.replace(
        "<triangles count=\"1\">",
        &format!("<triangles count=\"1\"{attributes}>"),
    );
    assert_triangle(&at_limit);
    assert_rejected(
        "dae",
        &at_limit.replace("a126=\"value\"", "a126=\"value\" a127=\"value\""),
    );
}

#[test]
fn collada_late_duplicate_attribute_returns_an_error() {
    let attributes: String = (0..120).map(|i| format!(" a{i}=\"value\"")).collect();
    // Stay below the attribute cap so that the duplicate itself is rejected.
    assert_rejected(
        "dae",
        &COLLADA.replace(
            "<triangles count=\"1\">",
            &format!("<triangles count=\"1\"{attributes} a119=\"duplicate\">"),
        ),
    );
}

#[test]
fn collada_depth_limit_accepts_boundary_and_rejects_excess() {
    for (nested_elements, accepted) in [(125, true), (126, false)] {
        // COLLADA, extra, and technique occupy the first three depth levels.
        let extra = format!(
            "<extra><technique profile=\"depth-test\">{}{}</technique></extra>",
            "<nested>".repeat(nested_elements),
            "</nested>".repeat(nested_elements)
        );
        let contents = COLLADA.replace("</COLLADA>", &format!("{extra}</COLLADA>"));
        if accepted {
            assert_triangle(&contents);
        } else {
            assert_rejected("dae", &contents);
        }
    }
}

#[test]
fn collada_numeric_entities_in_count_and_offset_preserve_geometry() {
    for (count, offset) in [("&#49;", "&#48;"), ("&#x31;", "&#x30;")] {
        let inputs = VERTEX_INPUT.replace("offset=\"0\"", &format!("offset=\"{offset}\""));
        assert_triangle(&collada(&inputs, count, Some("0 1 2")));
    }
}

#[test]
fn collada_extreme_offset_encoded_as_numeric_entities_returns_an_error() {
    let offset: String = u32::MAX
        .to_string()
        .bytes()
        .map(|digit| format!("&#{digit};"))
        .collect();
    let inputs = VERTEX_INPUT.replace("offset=\"0\"", &format!("offset=\"{offset}\""));
    assert_rejected("dae", &collada(&inputs, "1", Some("0 1 2")));
}

#[test]
fn collada_huge_float_array_count_returns_an_error() {
    assert_rejected(
        "dae",
        &COLLADA.replace(
            "id=\"triangle_positions_array\" count=\"9\"",
            &format!("id=\"triangle_positions_array\" count=\"{}\"", usize::MAX),
        ),
    );
}

#[test]
fn collada_overflowing_accessor_size_returns_an_error() {
    assert_rejected(
        "dae",
        &COLLADA.replace(
            "count=\"3\" stride=\"3\"",
            &format!("count=\"{}\" stride=\"3\"", usize::MAX),
        ),
    );
}

#[test]
fn collada_sparse_file_over_document_size_limit_returns_an_error() {
    let fixture = Fixture::new("dae", COLLADA);
    fs::OpenOptions::new()
        .write(true)
        .open(&fixture.path)
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    assert_fixture_rejected(&fixture);
}

#[test]
fn collada_multiple_geometries_preserve_local_triangle_indices() {
    let start = COLLADA.find("<geometry ").unwrap();
    let end = COLLADA.find("</geometry>").unwrap() + "</geometry>".len();
    let second_geometry = COLLADA[start..end]
        .replace("triangle_", "second_")
        .replace("-0.7 2.1 0", "-0.7 2.1 1")
        .replace("1.4 4.2 0", "1.4 4.2 1")
        .replace("-3.5 4.9 0", "-3.5 4.9 1");
    let fixture = Fixture::new(
        "dae",
        &COLLADA.replace(
            "</library_geometries>",
            &format!("{second_geometry}</library_geometries>"),
        ),
    );
    for result in [
        load_trimesh(fixture.path_str(), 1.0),
        load_trimesh_with_flags(fixture.path_str(), 1.0, TriMeshFlags::empty()),
    ] {
        let mesh = result.expect("valid COLLADA geometries must merge");
        assert_eq!(mesh.vertices().len(), 6);
        assert_eq!(mesh.indices(), &[[0, 1, 2], [3, 4, 5]]);
        assert!(mesh.vertices()[..3].iter().all(|v| v.z == 0.0));
        assert!(mesh.vertices()[3..].iter().all(|v| v.z == 1.0));
    }
}

/// Every format the crate loads, including the optional glTF ones.
fn extensions() -> Vec<&'static str> {
    let mut extensions = vec!["dae", "ply", "stl", "obj"];
    if cfg!(feature = "gltf") {
        extensions.extend(["gltf", "glb"]);
    }
    extensions
}

/// Fails instead of hanging the suite if a loader blocks on the path again.
fn load_within(path: &str, seconds: u64) -> Result<(), String> {
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    let owned = path.to_string();
    // The thread is abandoned on timeout: a blocked open cannot be cancelled,
    // and the assertion below has already failed by then.
    std::thread::spawn(move || {
        let _ = send.send(load_trimesh(&owned, 1.0).map(|_| ()));
    });
    receive
        .recv_timeout(std::time::Duration::from_secs(seconds))
        .unwrap_or_else(|_| panic!("loading {path} did not return promptly"))
}

#[cfg(unix)]
fn make_fifo(path: &std::path::Path) {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    // SAFETY: path is a live NUL-terminated string, and mkfifo retains no pointer.
    assert_eq!(
        unsafe { libc::mkfifo(path.as_ptr(), 0o600) },
        0,
        "{}",
        std::io::Error::last_os_error()
    );
}

#[cfg(unix)]
#[test]
fn every_format_rejects_a_fifo_without_waiting_for_a_writer() {
    for extension in extensions() {
        let fixture = Fixture::new(extension, "");
        fs::remove_file(&fixture.path).unwrap();
        make_fifo(&fixture.path);

        let error = load_within(fixture.path_str(), 10).unwrap_err();
        assert!(error.contains("regular files"), "{extension}: {error}");
    }
}

#[cfg(unix)]
#[test]
fn every_format_rejects_an_endless_character_device() {
    for extension in extensions() {
        let fixture = Fixture::new(extension, "");
        fs::remove_file(&fixture.path).unwrap();
        std::os::unix::fs::symlink("/dev/zero", &fixture.path).unwrap();

        let error = load_within(fixture.path_str(), 10).unwrap_err();
        assert!(error.contains("regular files"), "{extension}: {error}");
    }
}

#[test]
fn every_format_rejects_an_oversized_document() {
    for extension in extensions() {
        let fixture = Fixture::new(extension, "");
        // Sparse, so the test does not write a quarter of a gigabyte.
        fs::OpenOptions::new()
            .write(true)
            .open(&fixture.path)
            .unwrap()
            .set_len(256 * 1024 * 1024 + 1)
            .unwrap();

        let error = load_within(fixture.path_str(), 30).unwrap_err();
        assert!(error.contains("byte limit"), "{extension}: {error}");
    }
}
