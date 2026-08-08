# shrubbery
<img src="shrubbery_logo.png" width="200" />

rust library: Space colonization implementation, for generating trees / shrubbery, with built in voxelization utility.

### example: "editor"
![editor preview](editor_preview.png)

## Example code
```rs
use shrubbery::prelude::*;

// Settings describe the build steps; load from RON (see assets/shrubbery/)
// or construct them in code.
let settings: ShrubberySettings =
    ron::de::from_str(&std::fs::read_to_string("oak.shrubbery.ron")?)?;

// Generation is deterministic per seed.
let mut generator = ShrubberyGenerator::generate(42, &settings);

// Voxelize into (IVec3 grid position, VoxelId) pairs.
let voxels = generator.voxelize();
```

Runnable demos live in `examples/` (e.g. `cargo run --example bevy`).

## Feature flags

| Feature | Default | Description |
|---------|---------|-------------|
| `serde` | ✓ | Serialization for settings and voxel definitions (RON, etc.) |
| `bevy` | ✓ | Bevy integration: `.shrubbery.ron` asset loader, plugin, debug draw. Implies `serde` and pulls in `ron` |

Minimal build: `cargo add shrubbery --no-default-features`.

## Testing

```sh
cargo test
```

Determinism is verified with golden hashes: `tests/golden_hashes/*.golden.ron`
record the expected voxel-output hash for every asset in `assets/shrubbery/`
at a fixed set of seeds. The tests discover assets automatically — there is no
hard-coded list.

If you **intentionally change generation behavior** or **add a new asset**,
the golden hash test will fail until you regenerate and commit the goldens:

```sh
cargo test --test generate_golden_hashes -- --ignored --nocapture
```

Review the resulting diff carefully — a changed golden hash means the voxel
output for that asset and seed changed.

## License

Shrubbery is free and open source! All code in this repository is dual-licensed under either:

* MIT License ([LICENSE-MIT](docs/LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](docs/LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
