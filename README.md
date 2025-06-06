# rs-read-trimesh
[![GitHub](https://img.shields.io/badge/GitHub-777777)](https://github.com/bourumir-wyngs/rs-read-trimesh)
[![crates.io](https://img.shields.io/crates/v/rs-read-trimesh.svg)](https://crates.io/crates/rs-read-trimesh)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/bourumir-wyngs/rs-read-trimesh/rust.yml)](https://github.com/bourumir-wyngs/rs-read-trimesh/actions)
[![crates.io](https://img.shields.io/crates/l/rs-read-trimesh.svg)](https://crates.io/crates/rs-read-trimesh)
[![crates.io](https://img.shields.io/crates/d/rs-read-trimesh.svg)](https://crates.io/crates/rs-read-trimesh)
[![docs.rs](https://docs.rs/rs-read-trimesh/badge.svg)](https://docs.rs/rs-read-trimesh)

`rs-read-trimesh` is a Rust library for loading 3D triangular meshes from files in various 3D formats. The main motivation behind this library is that existing readers do not directly output the format we work with (Parry's `TriMesh`) and require additional boilerplate code, which would be better implemented as a separate dependency.

The version 2.0.0 supports Parry versions from 0.9 through 0.19 inclusive, but you may need to specify which one
to be used (see examples below)

## Features
The library provides a single function that reads a file into a `TriMesh` given its file path. It supports `.ply`, `.stl`, `.obj` and `.dae` (Collada) formats, with built-in robustness to handle the diverse data structures found in `.ply` files, which may use different data types.

Unlike in 1.x versions, using the newest versions is the default setting. The Parry 0.18 through 0.21 inclusive can be used simply as

```toml
   [dependencies]
   rs-read-trimesh = "2.0.1"
```

If you want to use Parry 0.14 through 0.17, you need to disable default features and turn the feature `use-parry-14_17` on:

```toml
   [dependencies]
   rs-read-trimesh = { version = "2.0.1", default-features = false, features = ["use-parry-14_17"] }
```

Finally, you can also use Parry 0.9 through 0.13 if you work with legacy versions
```toml
   [dependencies]
   rs-read-trimesh = { version = "2.0.1", default-features = false, features = ["use-parry-9_13"] }
```

## Usage

Here’s an example using the `load_trimesh` function to load a 3D model and apply a scaling factor:

```rust
use rs_read_trimesh::load_trimesh;

fn main() {
    let file_path = "example.ply"; 
    let scale = 0.001; // Suppose the mesh is in mm; we want it in meters.

    match load_trimesh(file_path, scale) {
        Ok(mesh) => {
            println!("Successfully loaded and scaled mesh with {} vertices.", mesh.vertices.len());
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

## Dependencies

The following crates are used to power the functionality of this library:

- [`ply-rs-bw`](https://crates.io/crates/ply-rs-bw): A library for reading and writing PLY files.
- [`stl_io`](https://crates.io/crates/stl_io): A library for reading and writing STL files.
- [`tobj`](https://crates.io/crates/tobj): A library for loading OBJ files.
- [`dae-parser`](https://crates.io/crates/dae-parser): A library for loading Collada (DAE) files.
- [`parry3d`](https://crates.io/crates/parry3d): Provides 3D geometry processing for physical simulations. 

Parry is only used as much here as its mesh data structure is involved. As mentioned, versions from as old as 0.9
to the recent 0.18 are supported.

You **do not need to add these dependencies manually** to your `Cargo.toml`. They are automatically resolved by Cargo when you include `rs-read-trimesh` or any of the mentioned libraries.

## License

This project is licensed under the MIT License (see the [LICENSE](LICENSE) file for details). Some testing material is under Apache v 2.0.
