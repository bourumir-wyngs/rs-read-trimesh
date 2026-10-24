//! Regression tests for the safe behavior required by the glTF PR review.
//! The bug cases must fail until the loader rejects malformed input without
//! panicking or hanging, and preserves winding or rejects mirrored transforms.
//! Run with `cargo test --test gltf_claims -- --nocapture`.
//! Also run with `--release` to detect unchecked index addition wrapping.
//! The `usize`-to-`u32` vertex-count cast is not exercised at its limit: doing so
//! through the public loader would require allocating over four billion vertices.
#![cfg(feature = "gltf")]

#[cfg(feature = "parry_19")]
use parry_19 as parry;
#[cfg(feature = "parry_26")]
use parry_26 as parry;
#[cfg(feature = "parry_27")]
use parry_27 as parry;
#[cfg(feature = "parry_28")]
use parry_28 as parry;
#[cfg(feature = "parry13")]
use parry13 as parry;
#[cfg(feature = "parry17")]
use parry17 as parry;

use gltf::json::Value;
use parry::shape::{TriMesh, TriMeshFlags};
use rs_read_trimesh::load_trimesh_with_flags;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

#[path = "gltf_claims/dependency.rs"]
mod dependency;
#[path = "gltf_claims/geometry.rs"]
mod geometry;
#[path = "gltf_claims/hierarchy.rs"]
mod hierarchy;

fn json(text: &str) -> Value {
    gltf::json::deserialize::from_str(text).expect("valid fixture JSON")
}

/// A valid indexed, counterclockwise triangle in the XY plane, facing +Z.
fn triangle() -> (Value, Vec<u8>) {
    let document = json(
        r#"{
            "asset": {"version": "2.0"},
            "scene": 0,
            "scenes": [{"nodes": [0]}],
            "nodes": [{"mesh": 0}],
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
            "buffers": [{"byteLength": 42, "uri": "buffer.bin"}],
            "bufferViews": [
                {"buffer": 0, "byteOffset": 0, "byteLength": 36},
                {"buffer": 0, "byteOffset": 36, "byteLength": 6}
            ],
            "accessors": [
                {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
                 "min": [0, 0, 0], "max": [1, 1, 0]},
                {"bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR"}
            ]
        }"#,
    );
    let mut buffer: Vec<u8> = [0.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .collect();
    buffer.extend([0_u16, 1, 2].into_iter().flat_map(u16::to_le_bytes));
    (document, buffer)
}

struct Fixture {
    directory: PathBuf,
    path: PathBuf,
}

impl Fixture {
    fn new(document: &Value, buffer: &[u8]) -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let directory = loop {
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let candidate = std::env::temp_dir()
                .join(format!("rs-read-trimesh-gltf-{}-{id}", std::process::id()));
            match fs::create_dir(&candidate) {
                Ok(()) => break candidate,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("could not create fixture directory: {error}"),
            }
        };
        let fixture = Self {
            path: directory.join("scene.gltf"),
            directory,
        };
        fs::write(
            &fixture.path,
            gltf::json::serialize::to_vec(document).unwrap(),
        )
        .unwrap();
        fs::write(fixture.directory.join("buffer.bin"), buffer).unwrap();
        fixture
    }

    fn path_str(&self) -> &str {
        self.path.to_str().expect("UTF-8 fixture path")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Remove only files owned by this fixture, including the subprocess marker.
        for name in ["scene.gltf", "buffer.bin", "ready"] {
            let _ = fs::remove_file(self.directory.join(name));
        }
        let _ = fs::remove_dir(&self.directory);
    }
}

fn load(fixture: &Fixture) -> Result<TriMesh, String> {
    load_trimesh_with_flags(fixture.path_str(), 1.0, TriMeshFlags::empty())
}
