use std::ops::Deref;

use ahash::HashSet;
use bevy::{
    asset::{Asset, AssetLoader},
    prelude::*,
    reflect::TypePath,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{shrubbery::ShrubberySettings, voxel::VoxelDefinitions};

pub struct ShrubberyPlugin;

impl Plugin for ShrubberyPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.init_asset::<ShrubberyAsset>();
        app.init_asset_loader::<ShrubberyAssetLoader>();
        app.insert_resource(TreeAssetsAwaitingSync(HashSet::default()));
        app.add_systems(
            PostUpdate,
            (
                begin_sync_voxel_ids_with_tree_assets,
                sync_voxel_ids_with_tree_assets,
            ),
        );
    }
}

#[derive(Debug, Error)]
pub enum RonLoaderError {
    #[error("could not load asset: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not parse RON: {0}")]
    Ron(#[from] ron::error::SpannedError),
}

/// A loaded `*.shrubbery.ron` file, wrapping a [`ShrubberySettings`] recipe.
#[derive(Clone, Default, Debug, Asset, TypePath, Serialize, Deserialize)]
#[cfg_attr(feature = "serde", serde(transparent))] // collapse the inner newtype
pub struct ShrubberyAsset(pub ShrubberySettings);

impl Deref for ShrubberyAsset {
    type Target = ShrubberySettings;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Default, TypePath)]
pub struct ShrubberyAssetLoader;

impl AssetLoader for ShrubberyAssetLoader {
    type Asset = ShrubberyAsset;
    type Settings = ();
    type Error = RonLoaderError;

    async fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _settings: &Self::Settings,
        _load_context: &mut bevy::asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let asset: Self::Asset = ron::de::from_bytes(&bytes)?;
        Ok(asset)
    }

    fn extensions(&self) -> &[&str] {
        &["shrubbery.ron"]
    }
}

/// Tree assets waiting for a `VoxelDefinitions` resource to resolve their
/// voxel names into ids.
#[derive(Resource)]
pub struct TreeAssetsAwaitingSync(HashSet<AssetId<ShrubberyAsset>>);

fn begin_sync_voxel_ids_with_tree_assets(
    mut tree_assets: ResMut<Assets<ShrubberyAsset>>,
    mut events: MessageReader<AssetEvent<ShrubberyAsset>>,
    voxel_definitions: Option<Res<VoxelDefinitions>>,
    mut trees_awaiting_sync: ResMut<TreeAssetsAwaitingSync>,
) {
    for event in events.read() {
        let (AssetEvent::LoadedWithDependencies { id } | AssetEvent::Modified { id }) = event
        else {
            continue;
        };
        // Resolve now if a registry is already present, otherwise defer until
        // `sync_voxel_ids_with_tree_assets` sees one.
        match &voxel_definitions {
            Some(voxel_definitions) => {
                if let Some(tree_asset) = tree_assets.get_mut_untracked(*id) {
                    tree_asset.0.resolve_voxel_definitions(voxel_definitions);
                };
            }
            None => {
                trees_awaiting_sync.0.insert(*id);
            }
        }
    }
}

fn sync_voxel_ids_with_tree_assets(
    mut awaiting: ResMut<TreeAssetsAwaitingSync>,
    mut tree_assets: ResMut<Assets<ShrubberyAsset>>,
    voxel_definitions: Option<Res<VoxelDefinitions>>,
) {
    awaiting.0.retain(|id| {
        let Some(asset) = tree_assets.get_mut_untracked(*id) else {
            return false; // id no longer valid
        };
        let Some(voxel_definitions) = &voxel_definitions else {
            return true; // still waiting on a registry
        };
        asset.0.resolve_voxel_definitions(voxel_definitions);
        false
    });
}
