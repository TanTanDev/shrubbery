//! Determinism properties that must hold for every asset in `assets/shrubbery/`.
//!
//! Needs `ron` (asset parsing) and `serde`, both provided by the `bevy` feature.
#![cfg(feature = "bevy")]

mod common;

use common::{discover_assets, load_shrubbery_settings};
use shrubbery_voxel::prelude::*;

/// Generating twice with the same seed must produce identical voxels.
#[test]
fn same_seed_same_voxels() {
    for asset_name in discover_assets() {
        let settings = load_shrubbery_settings(&asset_name);
        assert_eq!(
            sorted_voxels(42, &settings),
            sorted_voxels(42, &settings),
            "same seed produced different voxels for {asset_name}"
        );
    }
}

/// Different seeds must produce different voxels.
#[test]
fn different_seeds_different_voxels() {
    for asset_name in discover_assets() {
        let settings = load_shrubbery_settings(&asset_name);
        assert_ne!(
            sorted_voxels(1, &settings),
            sorted_voxels(2, &settings),
            "different seeds produced identical voxels for {asset_name}"
        );
    }
}

/// Voxelize with `seed`, sorted by position since iteration order isn't guaranteed.
fn sorted_voxels(
    seed: u64,
    settings: &ShrubberySettings,
) -> Vec<(glam::IVec3, shrubbery::voxel::VoxelId)> {
    let mut generator = ShrubberyGenerator::generate(seed, settings);
    let mut voxels = generator.voxelize();
    voxels.sort_by_key(|(pos, _)| (pos.x, pos.y, pos.z));
    voxels
}
