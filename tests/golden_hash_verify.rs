//! Verify every asset reproduces the golden hashes recorded in `tests/golden_hashes/`.
//!
//! Needs `ron` (asset parsing) and `serde`, both provided by the `bevy` feature.
#![cfg(feature = "bevy")]

mod common;

use common::{ASSETS_DIR, GoldenHashFile, discover_assets, load_shrubbery_settings};
use shrubbery_voxel::prelude::*;

use crate::common::VoxelHasher;

#[test]
fn verify_all_deterministic_hashes() {
    let assets = discover_assets();
    assert!(!assets.is_empty(), "no assets found in {ASSETS_DIR}");

    for asset_name in &assets {
        let settings = load_shrubbery_settings(asset_name);
        let golden = GoldenHashFile::load(asset_name)
            .unwrap_or_else(|e| panic!("missing golden hashes for {asset_name}: {e}"));
        assert!(
            !golden.seeds.is_empty(),
            "no seeds defined for {asset_name}"
        );

        for entry in &golden.seeds {
            let mut generator = ShrubberyGenerator::generate(entry.seed, &settings);
            let voxels = generator.voxelize();
            let computed = VoxelHasher::hash_voxels(entry.seed, &voxels);

            assert_eq!(
                computed, entry.hash,
                "hash mismatch for asset={asset_name}, seed={} ({}): expected {}, got {computed}",
                entry.seed, entry.name, entry.hash
            );
        }
    }
}
