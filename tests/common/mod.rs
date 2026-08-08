//! Shared helpers for integration tests.
//!
//! Each test binary includes this module and uses a different subset of it,
//! so unused-code warnings are expected and allowed.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use shrubbery_voxel::{prelude::ShrubberySettings, voxel::VoxelId};

pub const ASSETS_DIR: &str = "assets/shrubbery";
pub const GOLDEN_DIR: &str = "tests/golden_hashes";

/// Names of every asset in `assets/shrubbery/`, sorted, without the `.shrubbery.ron` suffix.
pub fn discover_assets() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(ASSETS_DIR)
        .unwrap_or_else(|e| panic!("cannot read {ASSETS_DIR}: {e}"))
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().into_string().ok()?;
            name.strip_suffix(".shrubbery.ron").map(str::to_owned)
        })
        .collect();
    names.sort();
    names
}

/// Load and parse `assets/shrubbery/{asset_name}.shrubbery.ron`.
pub fn load_shrubbery_settings(asset_name: &str) -> ShrubberySettings {
    let path = format!("{ASSETS_DIR}/{asset_name}.shrubbery.ron");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
    ron::de::from_bytes(&bytes).unwrap_or_else(|e| panic!("cannot parse {path}: {e}"))
}

/// A single golden hash entry for a specific seed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenHashEntry {
    pub seed: u64,
    #[serde(default)]
    pub name: String,
    pub hash: String,
}

/// Collection of golden hashes for a single asset with multiple seeds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenHashFile {
    pub asset: String,
    pub seeds: Vec<GoldenHashEntry>,
}

impl GoldenHashFile {
    /// Path of the golden hash file for an asset, with or without the `.shrubbery.ron` suffix.
    pub fn path(asset_name: &str) -> String {
        let clean = asset_name
            .strip_suffix(".shrubbery.ron")
            .unwrap_or(asset_name);
        format!("{GOLDEN_DIR}/{clean}.golden.ron")
    }

    pub fn load(asset_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(ron::from_str(&std::fs::read_to_string(Self::path(
            asset_name,
        ))?)?)
    }

    pub fn save(&self, asset_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(GOLDEN_DIR)?;
        Ok(std::fs::write(
            Self::path(asset_name),
            ron::to_string(self)?,
        )?)
    }
}

use glam::IVec3;
use sha2::{Digest, Sha256};

/// Deterministic hasher for voxel grids using SHA256.
///
/// Produces consistent hash values regardless of voxel iteration order
/// by sorting coordinates before hashing.
pub struct VoxelHasher;

impl VoxelHasher {
    /// Hash a voxel grid deterministically using SHA256.
    ///
    /// Data layout: [seed_u64 | voxel_count_u32 | sorted_voxels]
    /// Returns lowercase hex string (first 16 chars of SHA256).
    pub fn hash_voxels(seed: u64, voxels: &[(IVec3, VoxelId)]) -> String {
        // Sort voxels lexicographically by coordinates (x, y, z)
        let mut sorted = voxels.to_vec();
        sorted.sort_by_key(|(pos, _)| (pos.x, pos.y, pos.z));

        // Build deterministic byte buffer: [seed || count || voxels]
        let mut data = Vec::new();
        data.extend_from_slice(&seed.to_le_bytes());
        data.extend_from_slice(&(sorted.len() as u32).to_le_bytes());

        for (pos, vid) in &sorted {
            data.extend_from_slice(&pos.x.to_le_bytes());
            data.extend_from_slice(&pos.y.to_le_bytes());
            data.extend_from_slice(&pos.z.to_le_bytes());
            data.extend_from_slice(&vid.0.to_le_bytes());
        }

        // Hash with SHA256
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let hash_bytes = hasher.finalize();

        // Return first 16 hex chars (64-bit equivalent length)
        format!(
            "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            hash_bytes[0],
            hash_bytes[1],
            hash_bytes[2],
            hash_bytes[3],
            hash_bytes[4],
            hash_bytes[5],
            hash_bytes[6],
            hash_bytes[7]
        )
    }

    /// Verify that voxels hash to the expected value.
    pub fn verify(
        seed: u64,
        voxels: &[(IVec3, VoxelId)],
        expected_hash: &str,
    ) -> Result<(), String> {
        let computed = Self::hash_voxels(seed, voxels);
        if computed == expected_hash {
            Ok(())
        } else {
            Err(format!(
                "Hash mismatch: expected {}, got {}",
                expected_hash, computed
            ))
        }
    }
}
