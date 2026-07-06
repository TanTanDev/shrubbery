use ahash::HashSet;
use bevy::{
    asset::{Asset, AssetLoader},
    prelude::*,
    reflect::TypePath,
};
use serde::{Deserialize, Serialize};

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

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RonLoaderError {
    #[error("could not load asset: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not parse RON: {0}")]
    RonSpannedError(#[from] ron::error::SpannedError),
}

// todo: rename to shrubbery asset
#[derive(Clone, Default, Debug, Asset, TypePath, Serialize, Deserialize)]
#[serde(transparent)] // collapse the inner tuple
pub struct ShrubberyAsset(pub ShrubberySettings);

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

/// these TreeAssets are awaiting to sync voxel names into VoxelIds, when VoxelDictionary are present
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
        // if voxel definitions are set, immediately resolve voxel ids' otherwise pass it to
        // 'sync_voxel_ids_with_tree_assets'
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
            return false; // id is no longer valid, don't retain
        };
        let Some(voxel_definitions) = &voxel_definitions else {
            return true;
        };
        asset.0.resolve_voxel_definitions(voxel_definitions);
        false // don't retain it's resolved, 
    });
}
