use super::{TriMesh, TriMeshFlags, load_trimesh, load_trimesh_with_flags};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

// A matching material used between these shared-vertex faces must not split
// geometry or make the result depend on an unused sidecar's contents.
const OBJ: &str = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3\nusemtl guessed\nf 1 3 4\n";
const MATCHING_MTL: &str = "newmtl guessed\nKd 1 0 0\n";

struct Fixture {
    directory: PathBuf,
    path: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let directory = loop {
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let candidate = std::env::temp_dir()
                .join(format!("rs-read-trimesh-obj-{}-{id}", std::process::id()));
            match fs::create_dir(&candidate) {
                Ok(()) => break candidate,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("could not create fixture directory: {error}"),
            }
        };
        let fixture = Self {
            path: directory.join("model/scene.obj"),
            directory,
        };
        fs::create_dir(fixture.path.parent().unwrap()).unwrap();
        fixture
    }

    fn write_obj(&self, libraries: &str) {
        fs::write(&self.path, format!("{libraries}{OBJ}")).unwrap();
    }

    fn path_str(&self) -> &str {
        self.path.to_str().expect("UTF-8 fixture path")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // All files and symlinks belong to this uniquely created fixture root.
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn assert_quad(fixture: &Fixture) {
    for mesh in [
        load_trimesh_with_flags(fixture.path_str(), 1.0, TriMeshFlags::empty()),
        load_trimesh(fixture.path_str(), 1.0),
    ] {
        let mesh: TriMesh = mesh.expect("unused material libraries must not prevent OBJ loading");
        let vertices: Vec<_> = mesh.vertices().iter().map(|v| [v.x, v.y, v.z]).collect();
        assert_eq!(
            vertices,
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0]
            ]
        );
        assert_eq!(mesh.indices(), &[[0, 1, 2], [0, 2, 3]]);
    }
}

#[test]
fn plain_obj_geometry_is_preserved() {
    let fixture = Fixture::new();
    fs::write(&fixture.path, OBJ.replace("usemtl guessed\n", "")).unwrap();
    assert_quad(&fixture);
}

#[test]
fn adjacent_material_library_contents_do_not_affect_geometry() {
    for material in [
        None,
        Some("newmtl other\nKd 0 1 0\n"),
        Some("newmtl guessed\nKd invalid\n"),
        Some(MATCHING_MTL),
    ] {
        let fixture = Fixture::new();
        fixture.write_obj("mtllib adjacent.mtl\nmtllib adjacent.mtl\n");
        if let Some(contents) = material {
            fs::write(fixture.path.with_file_name("adjacent.mtl"), contents).unwrap();
        }
        assert_quad(&fixture);
    }
}

#[test]
fn parent_traversal_material_library_is_ignored() {
    let fixture = Fixture::new();
    fs::write(fixture.directory.join("outside.mtl"), MATCHING_MTL).unwrap();
    fixture.write_obj("mtllib ../outside.mtl\n");
    assert_quad(&fixture);
}

#[test]
fn absolute_material_library_is_ignored() {
    let fixture = Fixture::new();
    let material = fixture.directory.join("outside.mtl");
    fs::write(&material, MATCHING_MTL).unwrap();
    fixture.write_obj(&format!("mtllib {}\n", material.display()));
    assert_quad(&fixture);
}

#[cfg(unix)]
#[test]
fn symlink_to_outside_material_library_is_ignored() {
    let fixture = Fixture::new();
    fs::write(fixture.directory.join("outside.mtl"), MATCHING_MTL).unwrap();
    std::os::unix::fs::symlink("../outside.mtl", fixture.path.with_file_name("linked.mtl"))
        .unwrap();
    fixture.write_obj("mtllib linked.mtl\n");
    assert_quad(&fixture);
}

#[test]
fn missing_or_invalid_obj_still_returns_an_error() {
    let fixture = Fixture::new();
    for contents in [None, Some("v invalid 0 0\nf 1 2 3\n"), Some("usemtl\n")] {
        if let Some(contents) = contents {
            fs::write(&fixture.path, contents).unwrap();
        }
        assert!(load_trimesh(fixture.path_str(), 1.0).is_err());
        assert!(load_trimesh_with_flags(fixture.path_str(), 1.0, TriMeshFlags::empty()).is_err());
    }
}
