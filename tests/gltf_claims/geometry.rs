use super::{Fixture, TriMesh, json, load, triangle};

fn signed_normal_z(mesh: &TriMesh) -> f32 {
    let [a, b, c] = mesh.indices()[0].map(|index| mesh.vertices()[index as usize]);
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

#[test]
fn valid_indexed_triangle_control() {
    let (document, buffer) = triangle();
    let mesh = load(&Fixture::new(&document, &buffer)).unwrap();
    assert_eq!(mesh.vertices().len(), 3);
    assert_eq!(mesh.indices(), &[[0, 1, 2]]);
    assert_eq!(signed_normal_z(&mesh), 1.0);
}

#[test]
fn absent_indices_generate_sequential_triangle_control() {
    let (mut document, buffer) = triangle();
    document["meshes"][0]["primitives"][0]
        .as_object_mut()
        .unwrap()
        .remove("indices");
    let fixture = Fixture::new(&document, &buffer);
    let mesh = load(&fixture).unwrap();
    assert_eq!(mesh.vertices().len(), 3);
    assert_eq!(mesh.indices(), &[[0, 1, 2]]);
    assert_eq!(signed_normal_z(&mesh), 1.0);
}

#[test]
fn out_of_bounds_index_returns_an_error_without_panicking() {
    let (document, mut buffer) = triangle();
    buffer[40..42].copy_from_slice(&3_u16.to_le_bytes());
    let fixture = Fixture::new(&document, &buffer);
    assert!(
        load(&fixture).is_err(),
        "index 3 exceeds the three positions"
    );
}

#[test]
fn invalid_local_index_cannot_reference_another_primitive() {
    let (mut document, mut buffer) = triangle();
    buffer[40..42].copy_from_slice(&3_u16.to_le_bytes());
    buffer.resize(44, 0); // Align the second POSITION view to a float boundary.
    buffer.extend(
        [2.0_f32, 2.0, 0.0, 3.0, 2.0, 0.0, 2.0, 3.0, 0.0]
            .into_iter()
            .flat_map(f32::to_le_bytes),
    );
    document["buffers"][0]["byteLength"] = buffer.len().into();
    document["bufferViews"]
        .as_array_mut()
        .unwrap()
        .push(json(r#"{"buffer":0,"byteOffset":44,"byteLength":36}"#));
    document["accessors"].as_array_mut().unwrap().push(json(
        r#"{"bufferView":2,"componentType":5126,"count":3,"type":"VEC3",
            "min":[2,2,0],"max":[3,3,0]}"#,
    ));
    document["meshes"][0]["primitives"]
        .as_array_mut()
        .unwrap()
        .push(json(r#"{"attributes":{"POSITION":2}}"#));

    assert!(
        load(&Fixture::new(&document, &buffer)).is_err(),
        "index 3 is outside its primitive even though the combined mesh has six vertices"
    );
}

#[test]
fn unreadable_existing_indices_return_an_error() {
    let (mut document, buffer) = triangle();
    // The accessor starts after the end of its six-byte index buffer view.
    document["accessors"][1]["byteOffset"] = 6.into();
    let fixture = Fixture::new(&document, &buffer);
    assert!(
        load(&fixture).is_err(),
        "unreadable indices must not generate a triangle"
    );
}

#[test]
fn unreadable_position_accessor_returns_an_error() {
    let (mut document, buffer) = triangle();
    let mut unreadable = document["accessors"][0].clone();
    unreadable["byteOffset"] = 36.into();
    document["accessors"]
        .as_array_mut()
        .unwrap()
        .push(unreadable);
    document["meshes"][0]["primitives"]
        .as_array_mut()
        .unwrap()
        .push(json(r#"{"attributes":{"POSITION":2}}"#));
    let fixture = Fixture::new(&document, &buffer);
    assert!(
        load(&fixture).is_err(),
        "an unreadable POSITION accessor must not silently skip a primitive"
    );
}

#[test]
fn mirrored_node_preserves_face_orientation() {
    // glTF selects winding from the determinant of the accumulated transform:
    // https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#instantiation
    // Baking a reflection into Parry vertices must correct the winding.
    let (mut document, buffer) = triangle();
    document["nodes"][0]["scale"] = json("[-1,1,1]");
    let mesh = load(&Fixture::new(&document, &buffer)).expect("valid mirrored node must load");
    assert_eq!(mesh.vertices().len(), 3);
    assert_eq!(mesh.indices().len(), 1);
    assert_eq!(mesh.vertices()[1].x, -1.0);
    assert_eq!(signed_normal_z(&mesh), 1.0);
}

#[test]
fn mirrored_ancestor_preserves_face_orientation() {
    let (mut document, buffer) = triangle();
    document["nodes"] = json(r#"[{"scale":[-1,1,1],"children":[1]},{"mesh":0}]"#);
    let mesh = load(&Fixture::new(&document, &buffer)).expect("valid mirrored ancestor must load");
    assert_eq!(mesh.vertices().len(), 3);
    assert_eq!(mesh.indices().len(), 1);
    assert_eq!(mesh.vertices()[1].x, -1.0);
    assert_eq!(signed_normal_z(&mesh), 1.0);
}

#[test]
fn two_accumulated_mirrors_preserve_orientation_control() {
    let (mut document, buffer) = triangle();
    document["nodes"] = json(r#"[{"scale":[-1,1,1],"children":[1]},{"scale":[-1,1,1],"mesh":0}]"#);
    let mesh = load(&Fixture::new(&document, &buffer)).unwrap();
    assert_eq!(mesh.vertices()[1].x, 1.0);
    assert_eq!(mesh.indices(), &[[0, 1, 2]]);
    assert_eq!(signed_normal_z(&mesh), 1.0);
}

#[test]
fn overflowing_primitive_index_returns_an_error_without_panicking() {
    let (mut document, mut buffer) = triangle();
    buffer.resize(44, 0);
    buffer.extend([0_u32, 1, u32::MAX].into_iter().flat_map(u32::to_le_bytes));
    document["buffers"][0]["byteLength"] = buffer.len().into();
    document["bufferViews"]
        .as_array_mut()
        .unwrap()
        .push(json(r#"{"buffer":0,"byteOffset":44,"byteLength":12}"#));
    document["accessors"].as_array_mut().unwrap().push(json(
        r#"{"bufferView":2,"componentType":5125,"count":3,"type":"SCALAR"}"#,
    ));
    document["meshes"][0]["primitives"]
        .as_array_mut()
        .unwrap()
        .push(json(r#"{"attributes":{"POSITION":0},"indices":2}"#));
    let fixture = Fixture::new(&document, &buffer);
    assert!(
        load(&fixture).is_err(),
        "u32::MAX must be rejected before adding the primitive offset"
    );
}

#[test]
fn float_index_accessor_returns_an_error_without_panicking() {
    let (mut document, buffer) = triangle();
    // The referenced accessor exists, but FLOAT data cannot represent indices.
    document["meshes"][0]["primitives"][0]["indices"] = 0.into();
    load(&Fixture::new(&document, &buffer))
        .map(|_| ())
        .expect_err("a FLOAT POSITION accessor cannot be used as an index accessor");
}

#[test]
fn position_stride_smaller_than_one_element_returns_an_error() {
    let (mut document, buffer) = triangle();
    document["bufferViews"][0]["byteStride"] = 4.into();
    load(&Fixture::new(&document, &buffer))
        .map(|_| ())
        .expect_err("a twelve-byte POSITION element cannot have a four-byte stride");
}

#[test]
fn zero_position_count_returns_an_error_without_panicking() {
    let (mut document, buffer) = triangle();
    document["accessors"][0]["count"] = 0.into();
    load(&Fixture::new(&document, &buffer))
        .map(|_| ())
        .expect_err("zero accessor count must be rejected before calculating count - 1");
}

#[test]
fn overflowing_buffer_view_range_returns_an_error_without_panicking() {
    let (mut document, buffer) = triangle();
    document["bufferViews"][0]["byteOffset"] = u64::MAX.into();
    load(&Fixture::new(&document, &buffer))
        .map(|_| ())
        .expect_err("the buffer view end must not wrap when its length is added");
}

#[test]
fn overflowing_accessor_range_returns_an_error_without_panicking() {
    let (mut document, buffer) = triangle();
    document["accessors"][0]["byteOffset"] = u64::MAX.into();
    load(&Fixture::new(&document, &buffer))
        .map(|_| ())
        .expect_err("the accessor end must not wrap when its data size is added");
}

#[test]
fn overflowing_accessor_count_returns_an_error_without_panicking() {
    let (mut document, buffer) = triangle();
    document["accessors"][0]["count"] = u64::MAX.into();
    load(&Fixture::new(&document, &buffer))
        .map(|_| ())
        .expect_err("the accessor byte count must not wrap or cause an allocation");
}

#[test]
fn incomplete_triangle_returns_an_error_instead_of_dropping_an_index() {
    let (mut document, mut buffer) = triangle();
    buffer.extend(0_u16.to_le_bytes());
    document["buffers"][0]["byteLength"] = buffer.len().into();
    document["bufferViews"][1]["byteLength"] = 8.into();
    document["accessors"][1]["count"] = 4.into();
    load(&Fixture::new(&document, &buffer))
        .map(|_| ())
        .expect_err("four indices cannot form a complete TRIANGLES primitive");
}

/// A triangle whose POSITION accessor starts at zero and replaces vertices 1 and 2.
fn sparse_triangle() -> (gltf::json::Value, Vec<u8>) {
    let (mut document, mut buffer) = triangle();
    buffer.resize(44, 0);
    buffer.extend([1_u8, 2]);
    buffer.resize(48, 0);
    buffer.extend(
        [1.0_f32, 0.0, 0.0, 0.0, 1.0, 0.0]
            .into_iter()
            .flat_map(f32::to_le_bytes),
    );
    document["buffers"][0]["byteLength"] = buffer.len().into();
    document["bufferViews"].as_array_mut().unwrap().extend([
        json(r#"{"buffer":0,"byteOffset":44,"byteLength":2}"#),
        json(r#"{"buffer":0,"byteOffset":48,"byteLength":24}"#),
    ]);
    document["accessors"][0]
        .as_object_mut()
        .unwrap()
        .remove("bufferView");
    document["accessors"][0]["sparse"] = json(
        r#"{"count":2,"indices":{"bufferView":2,"componentType":5121},
            "values":{"bufferView":3}}"#,
    );
    (document, buffer)
}

#[test]
fn sparse_positions_without_a_base_buffer_load_successfully() {
    let (document, buffer) = sparse_triangle();
    let mesh = load(&Fixture::new(&document, &buffer)).expect("valid sparse triangle must load");
    assert_eq!(mesh.vertices().len(), 3);
    assert_eq!(
        mesh.vertices()
            .iter()
            .map(|vertex| [vertex.x, vertex.y, vertex.z])
            .collect::<Vec<_>>(),
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    );
    assert_eq!(mesh.indices(), &[[0, 1, 2]]);
}

#[test]
fn out_of_range_sparse_index_returns_an_error() {
    let (document, mut buffer) = sparse_triangle();
    buffer[45] = 3;
    load(&Fixture::new(&document, &buffer))
        .map(|_| ())
        .expect_err("sparse index 3 is outside the three-element POSITION accessor");
}

#[test]
fn non_increasing_sparse_indices_return_an_error() {
    let (document, mut buffer) = sparse_triangle();
    buffer[44..46].copy_from_slice(&[2, 1]);
    load(&Fixture::new(&document, &buffer))
        .map(|_| ())
        .expect_err("sparse indices must be strictly increasing");
}

#[test]
fn truncated_sparse_values_return_an_error_without_panicking() {
    let (mut document, buffer) = sparse_triangle();
    document["bufferViews"][3]["byteLength"] = 12.into();
    load(&Fixture::new(&document, &buffer))
        .map(|_| ())
        .expect_err("two sparse positions require twenty-four bytes of values");
}
