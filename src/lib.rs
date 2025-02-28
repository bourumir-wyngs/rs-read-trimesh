use parry3d::math::Point;
use parry3d::shape::{TriMesh, TriMeshFlags};
use ply_rs_bw::parser::Parser;
use ply_rs_bw::ply::{DefaultElement, Property};
use std::fs::File;
use std::io::BufReader;
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
/// This function applies flags FIX_INTERNAL_EDGES and MERGE_DUPLICATE_VERTICES. For precise control
/// over flags, use `load_trimesh_with_flags`
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
/// * If the file extension is not supported (not `.stl`, `.ply`,`.obj` or `.dae`).
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
/// }///
/// ```
pub fn load_trimesh(file_path: &str, scale: f32) -> Result<TriMesh, String> {
    load_trimesh_with_flags(
        file_path,
        scale,
        TriMeshFlags::FIX_INTERNAL_EDGES | TriMeshFlags::MERGE_DUPLICATE_VERTICES,
    )
}

/// Loads a 3D triangular mesh (TriMesh) from a given file. Allows specifying flags
/// (that is important if default flags make unwanted changes of the mesh content).
/// See `load_trimesh,` for example, and a more detailed description.
pub fn load_trimesh_with_flags(
    file_path: &str,
    scale: f32,
    flags: TriMeshFlags,
) -> Result<TriMesh, String> {
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
        Some("dae") => load_trimesh_from_dae(file_path)?,
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
    Ok(TriMesh::with_flags(vertices, indices, flags))
}

/// Function to load a TriMesh from a PLY file
fn load_trimesh_from_ply(ply_file_path: &str) -> Result<(Vec<Point<f32>>, Vec<[u32; 3]>), String> {
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
fn load_trimesh_from_stl(stl_file_path: &str) -> Result<(Vec<Point<f32>>, Vec<[u32; 3]>), String> {
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
fn load_trimesh_from_obj(obj_file_path: &str) -> Result<(Vec<Point<f32>>, Vec<[u32; 3]>), String> {
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
                .map(|chunk| [chunk[0], chunk[1], chunk[2]]),
        );
    }

    Ok((vertices, indices))
}

use dae_parser::{
    ArrayElement, Document, GeometryElement, LibraryElement, Primitive, Semantic,
};
use parry3d::na::Point3;

pub fn load_trimesh_from_dae(
    dae_file_path: &str,
) -> Result<(Vec<Point3<f32>>, Vec<[u32; 3]>), String> {
    // Open the file
    let file = File::open(Path::new(dae_file_path))
        .map_err(|e| format!("Failed to open .dae file: {}", e))?;
    let reader = BufReader::new(file);

    // Parse the Collada document
    let document =
        Document::from_reader(reader).map_err(|e| format!("Failed to parse .dae file {:?}", e))?;

    let mut meshes = Vec::new();

    // Iterate through geometries in the document
    for geometry in document.library.iter() {
        if let LibraryElement::Geometries(geometry) = geometry {
            for item in geometry.items.iter() {
                if let GeometryElement::Mesh(mesh) = &item.element {

                    let mut mesh_vertices = Vec::new();
                    let mut mesh_indices = Vec::new();

                    if let Some(vertices) = &mesh.vertices {
                        for input in vertices.inputs.iter() {
                            if input.semantic == Semantic::Position {
                                let source_uri = input.source.to_string();
                                let source_id =
                                    source_uri.strip_prefix('#').unwrap_or(&*source_uri);

                                for source in mesh.sources.iter() {
                                    if let Some(id) = &source.id {
                                        if id == &source_id {
                                            if let Some(positions) = &source.array {
                                                if let ArrayElement::Float(positions) = positions {
                                                    mesh_vertices.reserve(positions.len() / 3);
                                                    for pos in positions.chunks_exact(3) {
                                                        mesh_vertices.push(Point3::new(
                                                            pos[0], pos[1], pos[2],
                                                        ));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        for primitive in mesh.elements.iter() {
                            if let Primitive::Triangles(triangles) = primitive {
                                if let Some(prim) = &triangles.data.prim {
                                    for pos in prim.chunks_exact(3) {
                                        // It is already 3 member vectors of u32
                                        mesh_indices.push([pos[0], pos[1], pos[2]]);
                                    }
                                }
                            }
                        }
                        meshes.push((mesh_vertices, mesh_indices));
                    }
                }
            }
        }
    }

    if meshes.is_empty() {
        Err("The file contains no mesh".to_string())
    } else {
        Ok(merge_meshes(meshes))
    }
}

fn merge_meshes(
    meshes: Vec<(Vec<Point3<f32>>, Vec<[u32; 3]>)>,
) -> (Vec<Point3<f32>>, Vec<[u32; 3]>) {
    if meshes.len() == 1 {
        println!("Found single mesh:");
        return meshes.into_iter().next().unwrap();
    }

    let mut merged_vertices = Vec::new();
    let mut merged_indices = Vec::new();
    let mut vertex_offset = 0u32;

    for (vertices, indices) in meshes {
        // Add vertices
        merged_vertices.extend(vertices.iter().cloned());

        // Adjust indices and add them
        merged_indices.extend(
            indices
                .into_iter()
                .map(|[i0, i1, i2]| [i0 + vertex_offset, i1 + vertex_offset, i2 + vertex_offset]),
        );

        // Update vertex offset for next mesh
        vertex_offset += vertices.len() as u32;
    }

    (merged_vertices, merged_indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_meshes() {
        let mesh1 = (
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2]],
        );

        let mesh2 = (
            vec![
                Point3::new(1.0, 1.0, 0.0),
                Point3::new(2.0, 1.0, 0.0),
                Point3::new(1.0, 2.0, 0.0),
            ],
            vec![[0, 1, 2]],
        );

        let (merged_vertices, merged_indices) = merge_meshes(vec![mesh1, mesh2]);

        let expected_vertices = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(2.0, 1.0, 0.0),
            Point3::new(1.0, 2.0, 0.0),
        ];

        let expected_indices = vec![[0, 1, 2], [3, 4, 5]];

        assert_eq!(merged_vertices, expected_vertices);
        assert_eq!(merged_indices, expected_indices);
    }
}
