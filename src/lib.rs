use parry3d::math::Point;
use parry3d::shape::{TriMesh, TriMeshFlags};
use ply_rs_bw::parser::Parser;
use ply_rs_bw::ply::{DefaultElement, Property};
use std::fs::File;
use std::io::{BufReader};
use std::path::Path;
use stl_io::read_stl;
use tobj;

/// Loads a 3D triangular mesh (TriMesh) from a given file, applies optional scaling
/// and returns the constructed mesh. This function supports multiple formats such as `.stl`, `.ply`,
/// and `.obj`.
///
/// # Arguments
///
/// * `file_path` - A string slice that represents the path to the input file containing the 3D mesh.
/// * `scale` - A floating-point value used to apply scaling to the vertex data.
///             If `scale` is 1.0, no scaling is applied. For ply files scaling is more part of
///             the format, as they are unit-agnostic and may come in meters, millimeters or inches.
///
/// # Returns
///
/// * `Ok(TriMesh)` - The loaded and scaled `TriMesh` object containing the vertices and indices
/// * `Err(String)` - If the file format is unsupported, or an error occurs during the loading process.
///
/// # Supported Formats
///
/// This function determines the file type based on the file extension:
/// * `.stl` - Standard Tessellation Language files.
/// * `.ply` - Polygon files that can represent geometric 3D data.
/// * `.obj` - Wavefront OBJ files.
///
/// # Errors
///
/// Returns an error in the following cases:
/// * If the file extension is not supported (not `.stl`, `.ply`, or `.obj`).
/// * If the file cannot be read or parsed by the respective loader.
///
/// # Example
///
/// ```rust
/// use rs_read_trimesh::load_trimesh;
///
/// let file_path = "example.ply";
/// let scale = 0.001; // Let's assume ply is in mm and we want in meters
///
/// match load_trimesh(file_path, scale) {
///     Ok(mesh) => {
///         println!("Successfully loaded and scaled mesh with {} vertices.", mesh.vertices().len());
///     }
///     Err(e) => {
///         eprintln!("Failed to load mesh: {}", e);
///     }
/// }
/// ```
pub fn load_trimesh(file_path: &str, scale: f32) -> Result<TriMesh, String> {
    let path = Path::new(file_path);
    let mut vertices;
    let indices;

    // Determine the file extension and call the appropriate loader
    (vertices, indices) = match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .as_deref() // Convert Option<String> to Option<&str> for matching
    {
        Some("stl") => load_trimesh_from_stl(file_path)?,
        Some("ply") => load_trimesh_from_ply(file_path)?,
        Some("obj") => load_trimesh_from_obj(file_path)?,
        _ => {
            return Err(format!(
                "Unsupported file extension for '{}', only .stl, .ply, and .obj are supported.",
                file_path
            ));
        }
    };

    // Apply scaling in place to all vertices
    if (scale - 1.0).abs() > f32::EPSILON {
        for vertex in &mut vertices {
            *vertex *= scale; // Scale the vertex in place
        }
    }

    // Create and return the TriMesh
    Ok(TriMesh::with_flags(
        vertices,
        indices,
        TriMeshFlags::FIX_INTERNAL_EDGES | TriMeshFlags::MERGE_DUPLICATE_VERTICES,
    ))
}

/// Function to load a TriMesh from a PLY file
fn load_trimesh_from_ply(
    ply_file_path: &str,
) -> Result<(Vec<Point<f32>>, Vec<[u32; 3]>), String> {
    // Open the file
    let file = File::open(ply_file_path)
        .map_err(|err| format!("Could not open PLY file '{}': {}", ply_file_path, err))?;
    let mut reader = BufReader::new(file);

    // Create a PLY parser and parse the header
    let parser = Parser::<DefaultElement>::new();
    let ply = parser
        .read_ply(&mut reader)
        .map_err(|err| format!("Could not parse PLY file '{}': {}", ply_file_path, err))?;

    // Initialize containers for vertices and indices
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Extract vertices
    if let Some(vertices_elem) = ply.payload.get("vertex") {
        for vertex in vertices_elem {
            let x = vertex
                .get("x")
                .ok_or_else(|| "Missing 'x' coordinate in vertex".to_string())
                .and_then(|prop| match prop {
                    Property::Float(val) => Ok(*val),
                    Property::Double(val) => Ok(*val as f32),
                    _ => Err("Unexpected type for vertex 'x' coordinate".to_string()),
                })?;

            let y = vertex
                .get("y")
                .ok_or_else(|| "Missing 'y' coordinate in vertex".to_string())
                .and_then(|prop| match prop {
                    Property::Float(val) => Ok(*val),
                    Property::Double(val) => Ok(*val as f32),
                    _ => Err("Unexpected type for vertex 'y' coordinate".to_string()),
                })?;

            let z = vertex
                .get("z")
                .ok_or_else(|| "Missing 'z' coordinate in vertex".to_string())
                .and_then(|prop| match prop {
                    Property::Float(val) => Ok(*val),
                    Property::Double(val) => Ok(*val as f32),
                    _ => Err("Unexpected type for vertex 'z' coordinate".to_string()),
                })?;

            vertices.push(Point::new(x, y, z));
        }
    } else {
        return Err("No 'vertex' payload found in the PLY file".to_string());
    }

    // Extract faces (indices)
    if let Some(faces_elem) = ply.payload.get("face") {
        for (i, face) in faces_elem.iter().enumerate() {
            match face.get("vertex_indices") {
                Some(Property::ListUInt(indices_list)) => {
                    indices.push(extract_indices(indices_list, i)?);
                }
                Some(Property::ListInt(indices_list)) => {
                    indices.push(extract_indices(indices_list, i)?);
                }
                Some(Property::ListUShort(indices_list)) => {
                    indices.push(extract_indices(indices_list, i)?);
                }
                Some(Property::ListShort(indices_list)) => {
                    indices.push(extract_indices(indices_list, i)?);
                }

                Some(_) => {
                    return Err(format!(
                        "Unexpected property type for 'vertex_indices' in face {}",
                        i
                    ));
                }
                None => {
                    return Err(format!("Missing 'vertex_indices' property for face {}", i));
                }
            }
        }
    } else {
        return Err("No 'face' payload found in the PLY file".to_string());
    }

    Ok((vertices, indices))
}

// Helper function to handle index extraction
fn extract_indices<T>(indices_list: &[T], i: usize) -> Result<[u32; 3], String>
where
    T: TryInto<u32> + Copy,
    <T as TryInto<u32>>::Error: std::fmt::Debug,
{
    if indices_list.len() < 3 {
        return Err(format!("Insufficient indices for a triangle in face {}", i));
    }

    let a = indices_list[0]
        .try_into()
        .map_err(|_| format!("Failed to convert index 0 in face {} to u32", i))?;
    let b = indices_list[1]
        .try_into()
        .map_err(|_| format!("Failed to convert index 1 in face {} to u32", i))?;
    let c = indices_list[2]
        .try_into()
        .map_err(|_| format!("Failed to convert index 2 in face {} to u32", i))?;

    Ok([a, b, c])
}

/// Function to load a TriMesh from an STL file
fn load_trimesh_from_stl(
    stl_file_path: &str,
) -> Result<(Vec<Point<f32>>, Vec<[u32; 3]>), String> {
    // Open the STL file
    let file = File::open(stl_file_path)
        .map_err(|err| format!("Could not open STL file {}: {}", stl_file_path, err))?;
    let mut reader = BufReader::new(file);

    // Read the STL file into IndexedMesh
    let stl = read_stl(&mut reader)
        .map_err(|err| format!("Could not parse STL file {}: {}", stl_file_path, err))?;

    // Extract vertices and convert them to Point3<f32>
    let vertices: Vec<Point<f32>> = stl
        .vertices
        .into_iter()
        .map(|vertex| Point::new(vertex[0], vertex[1], vertex[2]))
        .collect();

    // Convert face indices from `usize` to `u32` and handle any potential issues
    let indices: Vec<[u32; 3]> = stl
        .faces
        .into_iter()
        .map(|face| {
            let mut converted_face = [0u32; 3];
            for (i, &vertex_index) in face.vertices.iter().enumerate() {
                converted_face[i] = vertex_index.try_into().map_err(|_| {
                    format!(
                        "Could not convert vertex index {} in face {:?} to u32",
                        vertex_index, face.vertices
                    )
                })?;
            }
            Ok(converted_face)
        })
        .collect::<Result<Vec<[u32; 3]>, String>>()?; // Collect and propagate errors

    Ok((vertices, indices))
}

/// Function to load a TriMesh from an OBJ file
fn load_trimesh_from_obj(
    obj_file_path: &str,
) -> Result<(Vec<Point<f32>>, Vec<[u32; 3]>), String> {
    // Load the OBJ file using the `tobj` library
    let (models, _) = tobj::load_obj(obj_file_path, &tobj::LoadOptions::default())
        .map_err(|e| format!("Failed to load OBJ file '{}': {}", obj_file_path, e))?;

    // Collect vertices and indices
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for model in models {
        let mesh = model.mesh;

        // Extract vertices
        vertices.extend(
            mesh.positions
                .chunks_exact(3)
                .map(|chunk| Point::new(chunk[0], chunk[1], chunk[2])),
        );

        // Extract indices (assume triangulated mesh)
        indices.extend(
            mesh.indices
                .chunks_exact(3)
                .map(|chunk| [chunk[0] as u32, chunk[1] as u32, chunk[2] as u32]),
        );
    }

    Ok((vertices, indices))
}

