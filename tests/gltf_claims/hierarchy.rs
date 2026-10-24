use super::{Fixture, json, load, triangle};
use std::fs;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const CHILD_PATH: &str = "RS_READ_TRIMESH_GLTF_PROBE_PATH";

struct ReapChild(Child);

impl Drop for ReapChild {
    fn drop(&mut self) {
        // Also clean up if an assertion or an I/O operation fails in the parent.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Re-run just the calling test in a child. The marker separates process startup
/// from the load timeout, and killing/reaping the child bounds cyclic traversal.
fn assert_traversal_finishes(document: &str, test_name: &str, buffer: &[u8], expect_error: bool) {
    if let Some(path) = std::env::var_os(CHILD_PATH) {
        let path = std::path::PathBuf::from(path);
        fs::write(path.with_file_name("ready"), b"ready").unwrap();
        let result = rs_read_trimesh::load_trimesh_with_flags(
            path.to_str().unwrap(),
            1.0,
            super::TriMeshFlags::empty(),
        );
        assert_eq!(
            result.is_err(),
            expect_error,
            "cyclic input must return Err; a valid hierarchy must load successfully"
        );
        std::process::exit(0);
    }

    let fixture = Fixture::new(&json(document), buffer);
    let ready = fixture.directory.join("ready");
    let mut child = ReapChild(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test_name, "--nocapture"])
            .env(CHILD_PATH, &fixture.path)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start traversal subprocess"),
    );

    let startup = Instant::now();
    while !ready.exists() {
        if let Some(status) = child.0.try_wait().unwrap() {
            // A tiny valid fixture may finish between checking the marker and
            // polling the process. That is successful completion, not startup failure.
            assert!(
                ready.exists(),
                "traversal child exited before loading: {status}"
            );
            assert!(status.success(), "traversal child failed: {status}");
            return;
        }
        assert!(
            startup.elapsed() < Duration::from_secs(10),
            "traversal child did not start within ten seconds"
        );
        thread::sleep(Duration::from_millis(10));
    }

    let started = Instant::now();
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success(), "traversal child failed: {status}");
            return;
        }
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "loader did not terminate within one second; cyclic input must return Err"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn self_cycle_returns_error_without_hanging() {
    // Exact review input: the traversal pops node 0 and pushes node 0 forever.
    assert_traversal_finishes(
        r#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"children":[0]}]}"#,
        "hierarchy::self_cycle_returns_error_without_hanging",
        &[],
        true,
    );
}

#[test]
fn two_node_cycle_returns_error_without_hanging() {
    assert_traversal_finishes(
        r#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"children":[1]},{"children":[0]}]}"#,
        "hierarchy::two_node_cycle_returns_error_without_hanging",
        &[],
        true,
    );
}

#[test]
fn acyclic_hierarchy_terminates() {
    let (mut document, buffer) = triangle();
    document["nodes"] = json(r#"[{"children":[1]},{"children":[2]},{"mesh":0}]"#);
    assert_traversal_finishes(
        &document.to_string(),
        "hierarchy::acyclic_hierarchy_terminates",
        &buffer,
        false,
    );
}

#[test]
fn repeated_parentage_returns_error() {
    let (mut document, buffer) = triangle();
    document["nodes"] = json(
        r#"[{"children":[1,2]},
            {"children":[3]},
            {"children":[3],"translation":[0,0,2]},
            {"mesh":0}]"#,
    );
    let fixture = Fixture::new(&document, &buffer);
    assert!(
        load(&fixture).is_err(),
        "a node with two parents must be rejected"
    );
}

#[test]
fn distinct_nodes_can_legitimately_share_a_mesh() {
    let (mut document, buffer) = triangle();
    document["scenes"][0]["nodes"] = json("[0,1]");
    document["nodes"] = json(r#"[{"mesh":0},{"mesh":0,"translation":[0,0,2]}]"#);
    let fixture = Fixture::new(&document, &buffer);
    let mesh = load(&fixture).expect("sharing a mesh is valid; sharing a child node is not");
    assert_eq!(mesh.vertices().len(), 6);
    assert_eq!(mesh.indices().len(), 2);
}
