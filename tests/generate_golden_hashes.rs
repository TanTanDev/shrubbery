//! Regenerate golden hashes for every asset in `assets/shrubbery/`.
//!
//! Run with: cargo test --test generate_golden_hashes -- --ignored --nocapture
//!
//! Needs `ron` (asset parsing) and `serde`, both provided by the `bevy` feature.
#![cfg(feature = "bevy")]

mod common;

use common::{
    GOLDEN_DIR, GoldenHashEntry, GoldenHashFile, discover_assets, load_shrubbery_settings,
};
use shrubbery::prelude::*;

use crate::common::VoxelHasher;

const SEEDS: [u64; 3] = [42, 12345, 999999];

#[test]
#[ignore]
fn generate_all_golden_hashes() {
    for asset_name in discover_assets() {
        let settings = load_shrubbery_settings(&asset_name);

        let seeds = SEEDS
            .into_iter()
            .map(|seed| {
                let mut generator = ShrubberyGenerator::generate(seed, &settings);
                let hash = VoxelHasher::hash_voxels(seed, &generator.voxelize());
                println!("  {asset_name} @ seed={seed}: {hash}");
                GoldenHashEntry {
                    seed,
                    name: format!("seed_{seed}"),
                    hash,
                }
            })
            .collect();

        let golden = GoldenHashFile {
            asset: format!("{asset_name}.shrubbery.ron"),
            seeds,
        };
        golden.save(&asset_name).expect("write golden hashes");
        println!("  → {}", GoldenHashFile::path(&asset_name));
    }

    println!("\n✓ All golden hashes generated in {GOLDEN_DIR}/");
}
