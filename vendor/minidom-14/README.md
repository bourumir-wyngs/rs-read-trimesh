minidom-14
==========

This directory vendors the crates.io release of `minidom-14` 0.17.0
(upstream commit `58269cf2f130aa133e0b061199d1d94d8b9346c1`). The original
MPL-2.0 license and source notices are retained.

Local changes upgrade `quick-xml` from 0.36 to 0.41 to resolve
RUSTSEC-2026-0194 and RUSTSEC-2026-0195. The parser combines the new separate
entity-reference events and preserves text nodes, XML whitespace trimming,
CDATA, and invalid-reference errors. Regression tests cover these behaviors
and large attribute lists with late duplicates.

Run the dependency tests with:

```sh
cargo test --manifest-path vendor/minidom-14/Cargo.toml
```

The root `[patch.crates-io]` override applies when building this repository.
Cargo ignores dependency-level patches when `rs-read-trimesh` is used by
another project. Downstream users need the override in their own root
manifest, pointing to this directory, until an upstream release or published
dependency forks carry the fix.

Upstream README
---------------

This is a fork of [`minidom 0.14`](https://lib.rs/crates/minidom), which is
exclusively receiving dependency upgrades and bugfixes. This crate exists
because minidom 0.15 changed its API significantly and dropped support for
[`quick-xml`](https://lib.rs/crates/quick-xml).
