use super::{Fixture, json, load, triangle};

#[test]
fn missing_position_accessor_returns_error_without_panicking() {
    let fixture = Fixture::new(
        &json(
            r#"{
                "asset": {"version": "2.0"},
                "meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}]
            }"#,
        ),
        &[],
    );

    // gltf 1.4.1 panics during opening, before loader geometry checks run.
    // Either safe prevalidation or a corrected dependency may reject this input.
    assert!(
        rs_read_trimesh::load_trimesh(fixture.path_str(), 1.0).is_err(),
        "missing POSITION accessor must return an error without panicking"
    );
}

#[test]
fn invalid_percent_encoded_uri_returns_error_without_panicking() {
    let fixture = Fixture::new(
        &json(r#"{"asset":{"version":"2.0"},"buffers":[{"byteLength":1,"uri":"%FF"}]}"#),
        &[],
    );

    // gltf 1.4.1 unwraps failed UTF-8 decoding during buffer import. The public
    // API must handle this complete input with an error instead of unwinding.
    assert!(
        rs_read_trimesh::load_trimesh(fixture.path_str(), 1.0).is_err(),
        "invalid percent-encoded URI must return an error without panicking"
    );
}

#[test]
fn valid_percent_encoded_buffer_uri_loads_successfully() {
    let (mut document, buffer) = triangle();
    document["buffers"][0]["uri"] = "%62uffer.bin".into();
    let fixture = Fixture::new(&document, &buffer);

    let mesh =
        load(&fixture).expect("valid percent-encoded URI resolves to the fixture's buffer.bin");
    assert_eq!(mesh.vertices().len(), 3);
    assert_eq!(mesh.indices(), &[[0, 1, 2]]);
}
