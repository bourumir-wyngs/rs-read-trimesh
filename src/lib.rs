use dae_parser::{ArrayElement, Document, GeometryElement, LibraryElement, Primitive, Semantic};
use ply_rs_bw::parser::Parser;
use ply_rs_bw::ply::{DefaultElement, Property};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use stl_io::read_stl;
use tobj;

#[cfg(not(any(
    feature = "parry13",
    feature = "parry17",
    feature = "parry_19",
    feature = "parry_26",
    feature = "parry_27",
    feature = "parry_28"
)))]
compile_error!(
    "Enable exactly one Parry backend feature: use-parry-9_13, use-parry-14_17, use-parry-18_19, use-parry-26, use-parry-27, or use-parry-28."
);

#[cfg(any(
    all(
        feature = "parry13",
        any(
            feature = "parry17",
            feature = "parry_19",
            feature = "parry_26",
            feature = "parry_27",
            feature = "parry_28"
        )
    ),
    all(
        feature = "parry17",
        any(
            feature = "parry_19",
            feature = "parry_26",
            feature = "parry_27",
            feature = "parry_28"
        )
    ),
    all(
        feature = "parry_19",
        any(feature = "parry_26", feature = "parry_27", feature = "parry_28")
    ),
    all(feature = "parry_26", any(feature = "parry_27", feature = "parry_28")),
    all(feature = "parry_27", feature = "parry_28")
))]
compile_error!(
    "Enable only one Parry backend feature. Disable default features when selecting a non-default Parry version."
);

#[cfg(feature = "parry13")]
use parry13::shape::{TriMesh, TriMeshFlags};

#[cfg(all(not(feature = "parry13"), feature = "parry17"))]
use parry17::shape::{TriMesh, TriMeshFlags};

#[cfg(all(
    not(any(feature = "parry13", feature = "parry17")),
    feature = "parry_19"
))]
use parry_19::shape::{TriMesh, TriMeshFlags};

#[cfg(all(
    not(any(feature = "parry13", feature = "parry17", feature = "parry_19")),
    feature = "parry_26"
))]
use parry_26::shape::{TriMesh, TriMeshFlags};

#[cfg(all(
    not(any(
        feature = "parry13",
        feature = "parry17",
        feature = "parry_19",
        feature = "parry_26"
    )),
    feature = "parry_27"
))]
use parry_27::shape::{TriMesh, TriMeshFlags};

#[cfg(all(
    not(any(
        feature = "parry13",
        feature = "parry17",
        feature = "parry_19",
        feature = "parry_26",
        feature = "parry_27"
    )),
    feature = "parry_28"
))]
use parry_28::shape::{TriMesh, TriMeshFlags};

/// Loads a 3D triangular mesh (TriMesh) from a given file, applies optional scaling
/// and returns the constructed mesh. This function supports multiple formats.
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
/// * `.dae` - Collada files
///
/// # Errors
///
/// Returns an error in the following cases:
/// * If the file extension is not supported.
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
    #[cfg(feature = "parry13")]
    return load_trimesh_with_flags(file_path, scale, TriMeshFlags::MERGE_DUPLICATE_VERTICES);

    #[cfg(any(
        feature = "parry_19",
        feature = "parry17",
        feature = "parry_26",
        feature = "parry_27",
        feature = "parry_28"
    ))]
    return load_trimesh_with_flags(
        file_path,
        scale,
        TriMeshFlags::FIX_INTERNAL_EDGES | TriMeshFlags::MERGE_DUPLICATE_VERTICES,
    );
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
        #[cfg(feature = "gltf")]
        Some("gltf") | Some("glb") => load_trimesh_from_gltf(file_path)?,
        _ => {
            return Err(format!(
                "Unsupported file extension for '{}', only .stl, .ply and .obj are supported.",
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
    #[cfg(any(
        feature = "parry_19",
        feature = "parry_26",
        feature = "parry_27",
        feature = "parry_28"
    ))]
    {
        return TriMesh::with_flags(vertices, indices, flags).map_err(|e| e.to_string());
    }

    #[cfg(any(feature = "parry13", feature = "parry17"))]
    {
        return Ok(TriMesh::with_flags(vertices, indices, flags));
    }
}

/// Function to load a TriMesh from a PLY file
fn load_trimesh_from_ply<V>(ply_file_path: &str) -> Result<(Vec<V>, Vec<[u32; 3]>), String>
where
    V: From<[f32; 3]>,
{
    // Open the file
    let file = File::open(ply_file_path)
        .map_err(|err| format!("Could not open .ply file '{}': {}", ply_file_path, err))?;
    let mut reader = BufReader::new(file);

    // Create a PLY parser and parse the header
    let parser = Parser::<DefaultElement>::new();
    let ply = parser
        .read_ply(&mut reader)
        .map_err(|err| format!("Could not parse .ply file '{}': {}", ply_file_path, err))?;

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

            vertices.push([x, y, z].into());
        }
    } else {
        return Err("No 'vertex' payload found in the .ply file".to_string());
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
fn load_trimesh_from_stl<V>(stl_file_path: &str) -> Result<(Vec<V>, Vec<[u32; 3]>), String>
where
    V: From<[f32; 3]>,
{
    // Open the STL file
    let file = File::open(stl_file_path)
        .map_err(|err| format!("Could not open STL file {}: {}", stl_file_path, err))?;
    let mut reader = BufReader::new(file);

    // Read the STL file into IndexedMesh
    let stl = read_stl(&mut reader)
        .map_err(|err| format!("Could not parse STL file {}: {}", stl_file_path, err))?;

    // Extract vertices and let the active Parry version infer its vertex type.
    let vertices: Vec<V> = stl
        .vertices
        .into_iter()
        .map(|vertex| [vertex[0], vertex[1], vertex[2]].into())
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

/// Function to load a TriMesh from a GLTF file
#[cfg(feature = "gltf")]
fn load_trimesh_from_gltf<V>(gltf_file_path: &str) -> Result<(Vec<V>, Vec<[u32; 3]>), String>
where
    V: From<[f32; 3]>,
{
    // Load the GLTF file using the `gltf` library
    let contents = catch_gltf_panic(|| gltf::Gltf::open(gltf_file_path))
        .and_then(|opened| opened.map_err(|e| e.to_string()))
        .map_err(|e| format!("Failed to load file '{}': {}", gltf_file_path, e))?;

    let base = std::path::Path::new(gltf_file_path).parent();
    let buffers =
        catch_gltf_panic(|| gltf::import_buffers(&contents.document, base, contents.blob))
            .and_then(|imported| imported.map_err(|e| e.to_string()))
            .map_err(|e| {
                format!(
                    "Failed to load buffers for file '{}': {}",
                    gltf_file_path, e
                )
            })?;

    // Collect vertices and indices
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // GLTF has 1 scene typically
    let scene = contents
        .document
        .default_scene()
        .or_else(|| contents.document.scenes().next())
        .ok_or_else(|| format!("No scenes found in glTF file: {gltf_file_path}!"))?;

    const IDENTITY: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    // Traverse scene nodes. Could be using recursion too
    let mut pending: Vec<(gltf::Node, [[f32; 4]; 4])> =
        scene.nodes().map(|node| (node, IDENTITY)).collect();

    let mut visited = std::collections::HashSet::new();

    while let Some((node, parent_mat)) = pending.pop() {
        if !visited.insert(node.index()) {
            return Err(format!(
                "Invalid node hierarchy in glTF file '{}': node {} is reachable more than once",
                gltf_file_path,
                node.index()
            ));
        }

        let mat = mul_mat4(parent_mat, node.transform().matrix());
        let mirrored = det_mat3(mat) < 0.0;

        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                // Only Triangles, we can add TriangleStrip and TriangleFan support but it's rarely used.
                if primitive.mode() != gltf::mesh::Mode::Triangles {
                    continue;
                }

                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                let Some(positions) = reader.read_positions() else {
                    // Positions should exist but can't be read (glTF file is broken)
                    if primitive.get(&gltf::Semantic::Positions).is_some() {
                        return Err(format!(
                            "Unreadable vertex positions in glTF file '{}'",
                            gltf_file_path
                        ));
                    }
                    // In practice the position attribute exists, but the spec doesn't require it so skip if missing
                    continue;
                };

                let index_offset = vertices.len() as u32;
                let count = positions.len() as u32;

                // Bake world transform into vertex positions
                vertices.extend(positions.map(|p| transform_vec3(mat, p).into()));

                let flat_indices: Vec<u32> = match reader.read_indices() {
                    Some(read_indices) => read_indices.into_u32().collect(),
                    // Indices should exist but can't be read (glTF file is broken)
                    None if primitive.indices().is_some() => {
                        return Err(format!(
                            "Unreadable indices in glTF file '{}'",
                            gltf_file_path
                        ));
                    }
                    // Create the indices if the geometry has no index buffer
                    None => (0..count).collect(),
                };

                // Indices must refer to this primitive's own vertices
                if let Some(invalid) = flat_indices.iter().find(|&&index| index >= count) {
                    return Err(format!(
                        "Invalid index {} in glTF file '{}': the primitive has only {} vertices",
                        invalid, gltf_file_path, count
                    ));
                }

                // Write indices by walking the indices in groups of three.
                // Negative scaling (det(mat) < 0): flip the faces so they stay CCW.
                indices.extend(flat_indices.chunks_exact(3).map(|face| {
                    let (b, c) = if mirrored { (2, 1) } else { (1, 2) };
                    [
                        face[0] + index_offset,
                        face[b] + index_offset,
                        face[c] + index_offset,
                    ]
                }));
            }
        }

        for child in node.children() {
            pending.push((child, mat));
        }
    }

    Ok((vertices, indices))
}

/// Runs a call into the `gltf` crate and turns a panic inside it into an error.
/// gltf 1.4.1 panics on some malformed files instead of returning an error.
#[cfg(feature = "gltf")]
fn catch_gltf_panic<T>(call: impl FnOnce() -> T) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(call))
        .map_err(|_| "the glTF parser panicked on malformed input".to_string())
}

/// Multiplies two column-major 4x4 matrices, returning `a * b`
#[cfg(feature = "gltf")]
fn mul_mat4(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    for c in 0..4 {
        for r in 0..4 {
            for k in 0..4 {
                out[c][r] += a[k][r] * b[c][k];
            }
        }
    }
    out
}

/// Transforms a point with a column-major 4x4 matrix.
#[cfg(feature = "gltf")]
fn transform_vec3(m: [[f32; 4]; 4], p: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * p[0] + m[1][0] * p[1] + m[2][0] * p[2] + m[3][0],
        m[0][1] * p[0] + m[1][1] * p[1] + m[2][1] * p[2] + m[3][1],
        m[0][2] * p[0] + m[1][2] * p[1] + m[2][2] * p[2] + m[3][2],
    ]
}

#[cfg(feature = "gltf")]
fn det_mat3(m: [[f32; 4]; 4]) -> f32 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        + m[0][1] * (m[1][2] * m[2][0] - m[1][0] * m[2][2])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// Function to load a TriMesh from an OBJ file
fn load_trimesh_from_obj<V>(obj_file_path: &str) -> Result<(Vec<V>, Vec<[u32; 3]>), String>
where
    V: From<[f32; 3]>,
{
    // Load the OBJ file using the `tobj` library
    let (models, _) = tobj::load_obj(obj_file_path, &tobj::LoadOptions::default())
        .map_err(|e| format!("Failed to load .obj file '{}': {}", obj_file_path, e))?;

    // Collect vertices and indices
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for model in models {
        let mesh = model.mesh;

        // Extract vertices
        vertices.extend(
            mesh.positions
                .chunks_exact(3)
                .map(|chunk| [chunk[0], chunk[1], chunk[2]].into()),
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

fn load_trimesh_from_dae<V>(dae_file_path: &str) -> Result<(Vec<V>, Vec<[u32; 3]>), String>
where
    V: Clone + From<[f32; 3]>,
{
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
                                                        mesh_vertices
                                                            .push([pos[0], pos[1], pos[2]].into());
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
                                // only add indices if the input semantic is a vertex
                                if let Some(vertex_input) = triangles
                                    .inputs
                                    .inputs
                                    .iter()
                                    .find(|input| input.semantic == Semantic::Vertex)
                                {
                                    let offset = vertex_input.offset as usize;
                                    let num_semantics = triangles.inputs.inputs.len();
                                    mesh_indices.reserve(triangles.count * 3);

                                    if let Some(prim) = &triangles.data.prim {
                                        for pos in prim.chunks_exact(num_semantics * 3) {
                                            mesh_indices.push([
                                                pos[offset],
                                                pos[num_semantics + offset],
                                                pos[(2 * num_semantics) + offset],
                                            ])
                                        }
                                    }
                                }
                            }
                        }

                        if !mesh_vertices.is_empty() {
                            meshes.push((mesh_vertices, mesh_indices));
                        }
                    }
                }
            }
        }
    }

    if meshes.is_empty() {
        Err("The .dae file contains no mesh".to_string())
    } else {
        Ok(merge_meshes(meshes))
    }
}

fn merge_meshes<V: Clone>(meshes: Vec<(Vec<V>, Vec<[u32; 3]>)>) -> (Vec<V>, Vec<[u32; 3]>) {
    if meshes.len() == 1 {
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
        fn point(x: f32, y: f32, z: f32) -> [f32; 3] {
            [x, y, z]
        }

        let mesh1 = (
            vec![
                point(0.0, 0.0, 0.0),
                point(1.0, 0.0, 0.0),
                point(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2]],
        );

        let mesh2 = (
            vec![
                point(1.0, 1.0, 0.0),
                point(2.0, 1.0, 0.0),
                point(1.0, 2.0, 0.0),
            ],
            vec![[0, 1, 2]],
        );

        let (merged_vertices, merged_indices) = merge_meshes(vec![mesh1, mesh2]);

        let expected_vertices = vec![
            point(0.0, 0.0, 0.0),
            point(1.0, 0.0, 0.0),
            point(0.0, 1.0, 0.0),
            point(1.0, 1.0, 0.0),
            point(2.0, 1.0, 0.0),
            point(1.0, 2.0, 0.0),
        ];

        let expected_indices = vec![[0, 1, 2], [3, 4, 5]];

        assert_eq!(merged_vertices, expected_vertices);
        assert_eq!(merged_indices, expected_indices);
    }
}
