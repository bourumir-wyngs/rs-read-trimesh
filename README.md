# rs-read-trimesh
[![GitHub](https://img.shields.io/badge/GitHub-777777)](https://github.com/bourumir-wyngs/rs-read-trimesh)
[![crates.io](https://img.shields.io/crates/v/rs-read-trimesh.svg)](https://crates.io/crates/rs-read-trimesh)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/bourumir-wyngs/rs-read-trimesh/rust.yml)](https://github.com/bourumir-wyngs/rs-read-trimesh/actions)
[![crates.io](https://img.shields.io/crates/l/rs-read-trimesh.svg)](https://crates.io/crates/rs-read-trimesh)
[![crates.io](https://img.shields.io/crates/d/rs-read-trimesh.svg)](https://crates.io/crates/rs-read-trimesh)
[![docs.rs](https://docs.rs/rs-read-trimesh/badge.svg)](https://docs.rs/rs-read-trimesh)

`rs-read-trimesh` is a Rust library for loading 3D triangular meshes from files in various 3D formats. The main motivation behind this library is that existing readers do not directly output the format we work with (Parry's `TriMesh`) and require additional boilerplate code, which would be better implemented as a separate dependency.

## Features
The library provides a single function that reads a file into a `TriMesh` given its file path. It supports `.ply`, `.stl`, `.obj`, `.dae` (Collada), `.gltf` and `.glb` formats, with built-in robustness to handle the diverse data structures found in `.ply` files, which may use different data types.

### Choosing a Parry version

Parry 0.28 and glTF support remain enabled by default. No `use-parry-NN` feature is
needed when using this default backend:

```toml
[dependencies]
rs-read-trimesh = "2.1.0"
```

Parry 0.29, 0.30 and 0.31 are also supported. To select a different backend, disable
default features and enable exactly one selector from this table:

| Parry version | Feature |
| --- | --- |
| 0.9–0.13 | `use-parry-9_13` |
| 0.14–0.17 | `use-parry-14_17` |
| 0.18–0.25 | `use-parry-18_19` (historical name) |
| 0.26 | `use-parry-26` |
| 0.27 | `use-parry-27` |
| 0.28 (default) | `use-parry-28` |
| 0.29 | `use-parry-29` |
| 0.30 | `use-parry-30` |
| 0.31 | `use-parry-31` |

For example, to use Parry 0.31 with glTF/GLB support:

```toml
[dependencies]
rs-read-trimesh = { version = "2.1.0", default-features = false, features = ["use-parry-31", "gltf"] }
# Add this only if your code refers to Parry types or flags directly:
parry3d = "0.31"
```

Omit `gltf` from the feature list if you only need PLY, STL, OBJ and DAE.
Disabling default features disables both the default Parry backend and glTF support.
To keep Parry 0.28 while disabling glTF, use `default-features = false` with
`features = ["use-parry-28"]`.

### When is a selector needed?

`load_trimesh` returns the selected backend's concrete `TriMesh` type, and
`load_trimesh_with_flags` accepts that backend's `TriMeshFlags`. If you only use the
loaded mesh through this library's return value, the default backend is sufficient;
you do not need to add `parry3d` yourself.

When sharing meshes or flags with another dependency, select its Parry version.
For example, a function expecting `parry3d` 0.30's `TriMesh` needs `use-parry-30`.
Parry 0.28's and 0.30's types are distinct even where their APIs look identical.
For Rapier, match the **Parry version Rapier depends on**, rather than Rapier's own
version number; flags can be imported through `rapier3d::parry`.
Adding a direct Parry or Rapier dependency does not automatically select this
library's backend.

Exactly one backend must be enabled. Use `default-features = false` when selecting
a backend other than 0.28. Cargo [combines features across dependency paths](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification),
so every dependency on this crate must agree on the backend and avoid re-enabling
the default when another backend is selected. `--all-features` is therefore
unsupported.

The three legacy selectors cover version ranges; they do not pin a particular
minor release. Cargo can resolve a different Parry release for another dependency,
as explained in its [public dependency guidance](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#version-metadata).
If you encounter incompatible `TriMesh` or `TriMeshFlags` types, inspect
`cargo tree -d` and `cargo tree -e features`. For a legacy range, you may need to
align the application's lockfile with
`cargo update -p parry3d@<resolved-version> --precise <required-version>`.
For the individual 0.26–0.31 selectors, each selects its matching minor line and
allows compatible patch updates.

### Parry 0.29–0.31 compatibility

These releases retain the mesh construction API used here:
`TriMesh::with_flags(vertices, indices, flags)`. Their vertices still support
conversion from `[f32; 3]` and scaling, so all supported file formats use the
existing loaders. Upstream changes to polylines in 0.29, SIMD/contact APIs in 0.30,
and query results and additional mesh flags in 0.31 do not require loader changes.
Parry 0.31.0 was yanked; the `0.31` dependency currently resolves to 0.31.1.
See the [upstream changelog](https://github.com/dimforge/parry/blob/master/CHANGELOG.md)
and [Parry 0.31 mesh API](https://docs.rs/parry3d/0.31.1/parry3d/shape/struct.TriMesh.html).

## Usage

Here’s an example using the `load_trimesh` function to load a 3D model and apply a scaling factor:

```rust
use rs_read_trimesh::load_trimesh;

fn main() {
    let file_path = "example.ply"; 
    let scale = 0.001; // Suppose the mesh is in mm; we want it in meters.

    match load_trimesh(file_path, scale) {
        Ok(mesh) => {
            println!("Successfully loaded and scaled mesh with {} vertices.", mesh.vertices().len());
        }
        Err(e) => {
            eprintln!("Error loading mesh: {}", e);
        }
    }
}
```

### Scaling

The `scale` parameter allows you to scale all the vertices of the mesh. Setting `scale = 1.0` will result in no scaling. Scaling ply files seems quite a frequent case as they are unit-agnostic.

### Limitations
For .dae, only triangle meshes are supported (this format may contain lots of other stuff). If the .dae file contains multiple meshes, they are merged.

OBJ material libraries (`mtllib`) are ignored and never opened. Material names
do not affect the returned geometry.

The `gltf` feature, enabled by default, adds `.gltf` and `.glb` support. When disabling
default features to select a Parry backend, enable `gltf` explicitly if needed.
The loader selects the default scene, or the first scene when none is designated,
and merges its triangle-list primitives with node transforms applied. Files without
scenes are rejected. Triangle strips, triangle fans, points, and lines are skipped.
Skinning, morph weights, and animation are not applied; the loader extracts the
unskinned, unmorphed base geometry. Textures and materials are not loaded.

glTF buffer files must resolve within the model file's parent directory.
Relative paths (including percent-encoded names), nested directories, embedded
data URIs, and GLB buffers are supported. Absolute paths, `file:` or other
external URI schemes, Windows drive/backslash paths, and paths or symlinks
escaping that directory are rejected. Keep an untrusted model and its buffers
in a dedicated directory containing only resources it may read.

## Dependencies

The following crates are used to power the functionality of this library:

- [`ply-rs-bw`](https://crates.io/crates/ply-rs-bw): A library for reading and writing PLY files.
- [`stl_io`](https://crates.io/crates/stl_io): A library for reading and writing STL files.
- [`tobj`](https://crates.io/crates/tobj): A library for loading OBJ files.
- [`dae-parser`](https://crates.io/crates/dae-parser): A library for loading Collada (DAE) files.
- [`parry3d`](https://crates.io/crates/parry3d): Provides 3D geometry processing for physical simulations. 

Parry is only used as much here as its mesh data structure is involved. As mentioned, versions from as old as 0.9
through 0.31 are supported.

You **do not need to add these dependencies manually** to your `Cargo.toml`. They are automatically resolved by Cargo when you include `rs-read-trimesh` or any of the mentioned libraries.

### Local XML security patch

This checkout patches `minidom-14`, a dependency of `dae-parser`, to use
`quick-xml` 0.41 and resolve RUSTSEC-2026-0194 and RUSTSEC-2026-0195.
See [the vendored dependency notes](vendor/minidom-14/README.md) for the
source, changes, and tests. Cargo does not propagate this root patch to
projects consuming `rs-read-trimesh`; those projects need their own patch
until a fixed dependency release is available.

## License

This project is licensed under the MIT License (see the [LICENSE](LICENSE) file for details). Some testing material is under Apache v 2.0.
