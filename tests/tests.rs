use rs_read_trimesh::{load_trimesh};
use std::path::Path;

#[test]
fn test_doubles_ints_ply() {
    let file_path = "tests/sample_files/doubles_ints.ply";
    run_trimesh_test(file_path);
}

#[test]
fn test_doubles_shorts_ply() {
    let file_path = "tests/sample_files/doubles_shorts.ply";
    run_trimesh_test(file_path);
}

#[test]
fn test_floats_ints_ply() {
    let file_path = "tests/sample_files/floats_ints.ply";
    run_trimesh_test(file_path);
}

#[test]
fn test_floats_shorts_ply() {
    let file_path = "tests/sample_files/floats_shorts.ply";
    run_trimesh_test(file_path);
}

#[test]
fn test_object_obj() {
    let file_path = "tests/sample_files/object.obj";
    run_trimesh_test(file_path);
}

#[test]
fn test_stl() {
    let file_path = "tests/sample_files/stl.stl";
    run_trimesh_test(file_path);
}

/// Helper function for running each test
fn run_trimesh_test(file_path: &str) {
    assert!(
        Path::new(file_path).exists(),
        "File {} does not exist. Make sure all test files are present.",
        file_path
    );

    // Attempt to load the TriMesh from the file
    match load_trimesh(file_path, 1.0) {
        Ok(mesh) => {
            // Verify the contents of the TriMesh match the expectations
            assert!(
                verify_trimesh_content(&mesh),
                "TriMesh loaded from {} does not match the expected content.",
                file_path
            );
        }
        Err(e) => panic!("Failed to load TriMesh from {}: {}", file_path, e),
    }
}

use parry3d::math::Point;
use parry3d::shape::TriMesh;

/// Helper to round floats to two decimal places
fn round_to_two_decimals(value: f32) -> f32 {
    (value * 100.0).round() / 100.0
}

/// Compare two floating-point numbers up to two decimal places
fn floats_match(a: f32, b: f32) -> bool {
    round_to_two_decimals(a) == round_to_two_decimals(b)
}

/// Verify the content of a TriMesh
pub fn verify_trimesh_content(mesh: &TriMesh) -> bool {
    // Define the expected vertices and face
    let expected_vertices = vec![
        Point::new(-0.7, 2.1, 0.0),
        Point::new(1.4, 4.2, 0.0),
        Point::new(-3.5, 4.9, 0.0),
    ];
    let expected_face = [0, 1, 2]; // The single triangle face

    // Check vertices
    let vertices = mesh.vertices();
    if vertices.len() != expected_vertices.len() {
        println!("Vertex count mismatch: expected {}, got {}", expected_vertices.len(), vertices.len());
        return false;
    }

    for (i, vertex) in vertices.iter().enumerate() {
        let expected_vertex = &expected_vertices[i];
        if !floats_match(vertex.x, expected_vertex.x)
            || !floats_match(vertex.y, expected_vertex.y)
            || !floats_match(vertex.z, expected_vertex.z)
        {
            println!(
                "Vertex mismatch at index {}: expected {:?}, got {:?}",
                i, expected_vertex, vertex
            );
            return false;
        }
    }

    // Check face indices
    let indices = mesh.indices();
    if indices.len() != 1 {
        println!("Face count mismatch: expected 1, got {}", indices.len());
        return false;
    }

    if indices[0] != expected_face {
        println!(
            "Face mismatch: expected {:?}, got {:?}",
            expected_face, indices[0]
        );
        return false;
    }
    // "TriMesh content matches expected values.";
    true
}