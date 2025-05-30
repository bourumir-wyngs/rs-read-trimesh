use rs_read_trimesh::{load_trimesh, load_trimesh_with_flags};
use std::path::Path;

#[cfg(feature = "parry13")]
use {parry13::math::Point,
     parry13::shape::{TriMesh, TriMeshFlags}
};

#[cfg(feature = "parry17")]
use {parry17::math::Point,
     parry17::shape::{TriMesh, TriMeshFlags}
};

#[cfg(feature = "parry_19")]
use {parry_19::math::Point,
     parry_19::shape::{TriMesh, TriMeshFlags}
};

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

#[test]
fn test_collada() {
    let file_path = "tests/sample_files/collada.dae";
    run_trimesh_test(file_path);
}

#[test]
fn test_collada_robot() {
    let expected_vertices = [
        0.0217462, 0.0295452, 0.210809, 0.0217462, 0.0525, 0.210809, -0.0217462, 0.0295452,
        0.210809, -0.0217462, 0.0525, 0.210809, 0.0525, -0.0217462, 0.03, 0.0525, 0.0217462, 0.03,
        0.0525, -0.0217462, 0.13, 0.0525, 0.0217462, 0.13, -0.0217462, -0.0525, 0.03, 0.0217462,
        -0.0525, 0.03, -0.0217462, -0.0525, 0.13, 0.0217462, -0.0525, 0.13, 0.0217462, -0.0525,
        0.03, 0.0525, -0.0217462, 0.03, 0.0217462, -0.0525, 0.13, 0.0525, -0.0217462, 0.13,
        0.0217462, 0.0525, 0.03, -0.0217462, 0.0525, 0.03, 0.0217462, 0.0525, 0.210809, -0.0217462,
        0.0525, 0.210809, -0.0525, -0.0217462, 0.03, -0.0217462, -0.0525, 0.03, -0.0525,
        -0.0217462, 0.13, -0.0217462, -0.0525, 0.13, 0.0482419, 0.0295452, 0.13, -0.0217462,
        0.0295452, 0.210809, -0.0482419, 0.0295452, 0.171109, 0.0482419, 0.0295452, 0.171109,
        0.0217462, 0.0295452, 0.210809, -0.0482419, 0.0295452, 0.13, 0.0482419, 0.0295452,
        0.171109, 0.0217462, 0.0525, 0.210809, 0.0217462, 0.0295452, 0.210809, -0.0217462, 0.0525,
        0.210809, -0.0482419, 0.0295452, 0.171109, -0.0217462, 0.0295452, 0.210809, 0.0525,
        -0.0217462, 0.13, 0.0525, 0.0217462, 0.13, 0.0482419, 0.0295452, 0.13, -0.0525, 0.0217462,
        0.13, -0.0525, -0.0217462, 0.13, -0.0482419, 0.0295452, 0.13, -0.0217462, -0.0525, 0.13,
        0.0217462, -0.0525, 0.13, 0.0525, 0.0217462, 0.03, 0.0525, -0.0217462, 0.03, -0.0217462,
        -0.0525, 0.03, -0.0217462, 0.0525, 0.03, 0.0217462, 0.0525, 0.03, 0.0217462, -0.0525, 0.03,
        -0.0525, -0.0217462, 0.03, -0.0525, 0.0217462, 0.03, -0.0482419, 0.0295452, 0.13,
        -0.0482419, 0.0295452, 0.171109, -0.0525, 0.0217462, 0.13, -0.0525, -0.0217462, 0.13,
        -0.0217462, 0.0525, 0.03, -0.0525, 0.0217462, 0.03, -0.0525, -0.0217462, 0.03, -0.0217462,
        0.0525, 0.210809, 0.0165685, -0.027939, 0.13, 0.0165685, 0.0295452, 0.13, 0.04, -0.027939,
        0.153431, 0.04, 0.0295452, 0.153431, -0.0165685, -0.027939, 0.13, -0.0165685, 0.0295452,
        0.13, 0.0165685, -0.027939, 0.13, 0.0165685, 0.0295452, 0.13, -0.0165685, 0.0295452, 0.13,
        -0.0165685, -0.027939, 0.13, -0.04, 0.0295452, 0.153431, -0.04, -0.027939, 0.153431, -0.04,
        0.0295452, 0.153431, -0.04, -0.027939, 0.153431, -0.04, 0.0295452, 0.186569, -0.04,
        -0.027939, 0.186569, -0.04, 0.0295452, 0.186569, -0.04, -0.027939, 0.186569, -0.0167685,
        0.0295452, 0.2098, -0.0167685, -0.027939, 0.2098, 0.0167685, -0.027939, 0.2098, 0.0167685,
        0.0295452, 0.2098, -0.0167685, -0.027939, 0.2098, -0.0167685, 0.0295452, 0.2098, 0.04,
        -0.027939, 0.186569, 0.04, 0.0295452, 0.186569, 0.0167685, -0.027939, 0.2098, 0.0167685,
        0.0295452, 0.2098, 0.04, -0.027939, 0.153431, 0.04, 0.0295452, 0.153431, 0.04, -0.027939,
        0.186569, 0.04, 0.0295452, 0.186569, 0.0167685, -0.027939, 0.2098, -0.0167685, -0.027939,
        0.2098, -0.0165685, -0.027939, 0.13, 0.0165685, -0.027939, 0.13, 0.04, -0.027939, 0.153431,
        -0.04, -0.027939, 0.186569, -0.04, -0.027939, 0.153431, 0.04, -0.027939, 0.186569,
        -0.0167685, 0.0295452, 0.2098, 0.0167685, 0.0295452, 0.2098, 0.0165685, 0.0295452, 0.13,
        -0.0165685, 0.0295452, 0.13, -0.04, 0.0295452, 0.153431, 0.04, 0.0295452, 0.186569, 0.04,
        0.0295452, 0.153431, -0.04, 0.0295452, 0.186569,
    ];
    let expected_indices = [
        0, 1, 2, 1, 3, 2, 4, 5, 6, 5, 7, 6, 8, 9, 10, 9, 11, 10, 12, 13, 14, 13, 15, 14, 16, 17, 18,
        17, 19, 18, 20, 21, 22, 21, 23, 22, 29, 24, 26, 25, 26, 27, 28, 25, 27, 27, 26, 24, 31, 32,
        30, 33, 34, 35, 36, 38, 43, 42, 43, 41, 40, 42, 41, 39, 40, 41, 43, 38, 41, 36, 37, 38, 44,
        46, 48, 47, 48, 46, 50, 51, 47, 46, 50, 47, 49, 46, 45, 44, 45, 46, 54, 52, 57, 57, 55, 54,
        57, 58, 55, 57, 52, 56, 56, 52, 59, 52, 53, 59, 60, 61, 62, 61, 63, 62, 64, 65, 66, 65, 67,
        66, 68, 69, 70, 69, 71, 70, 72, 73, 74, 73, 75, 74, 76, 77, 78, 77, 79, 78, 80, 81, 82, 81,
        83, 82, 84, 85, 86, 85, 87, 86, 88, 89, 90, 89, 91, 90, 99, 92, 96, 95, 96, 92, 94, 95, 92,
        93, 98, 94, 97, 98, 93, 92, 93, 94, 107, 100, 104, 103, 104, 100, 102, 103, 100, 101, 106,
        102, 105, 106, 101, 100, 101, 102,
    ];

    let file_path = "tests/sample_files/robot.dae";

    // We need to suppress the flags otherwise TriMesh constructor modifies the content.
    match load_trimesh_with_flags(file_path, 1.0, TriMeshFlags::empty()) {
        Ok(mesh) => {
            // Retrieve vertices and indices from the loaded mesh
            let actual_vertices = mesh.vertices();
            let actual_indices = mesh.indices();

            // Ensure counts match after conversion
            assert_eq!(
                actual_vertices.len(),
                expected_vertices.len() / 3, // Ensure we're comparing points
                "Vertex count does not match the expected value"
            );
            assert_eq!(
                actual_indices.len(),
                expected_indices.len() / 3, // Ensure we're comparing triangles
                "Index count does not match the expected value"
            );

            // Verify vertex content using chunks of 3
            for (i, chunk) in expected_vertices.chunks(3).enumerate() {
                let actual = &actual_vertices[i];
                assert!(
                    (actual.x - chunk[0]).abs() < f32::EPSILON,
                    "Vertex X-coordinate mismatch at index {}: expected {}, found {}",
                    i,
                    chunk[0],
                    actual.x
                );
                assert!(
                    (actual.y - chunk[1]).abs() < f32::EPSILON,
                    "Vertex Y-coordinate mismatch at index {}: expected {}, found {}",
                    i,
                    chunk[1],
                    actual.y
                );
                assert!(
                    (actual.z - chunk[2]).abs() < f32::EPSILON,
                    "Vertex Z-coordinate mismatch at index {}: expected {}, found {}",
                    i,
                    chunk[2],
                    actual.z
                );
            }

            // Verify index content using chunks of 3
            for (i, chunk) in expected_indices.chunks(3).enumerate() {
                let actual = &actual_indices[i];
                assert_eq!(
                    actual[0], chunk[0],
                    "Index mismatch at position {}: expected {}, found {}",
                    i, chunk[0], actual[0]
                );
                assert_eq!(
                    actual[1], chunk[1],
                    "Index mismatch at position {}: expected {}, found {}",
                    i, chunk[1], actual[1]
                );
                assert_eq!(
                    actual[2], chunk[2],
                    "Index mismatch at position {}: expected {}, found {}",
                    i, chunk[2], actual[2]
                );
            }
            println!(
                "Successfully validated TriMesh with {} vertices and {} faces.",
                actual_vertices.len() / 3,
                actual_indices.len() / 3
            );
        }
        Err(e) => {
            panic!("Failed to load TriMesh from {}: {}", file_path, e);
        }
    }

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
        println!(
            "Vertex count mismatch: expected {}, got {}",
            expected_vertices.len(),
            vertices.len()
        );
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
