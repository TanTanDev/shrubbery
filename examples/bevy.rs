///! the smallest implementation showcase
#[path = "common/scene_setup.rs"]
mod scene_setup;

use bevy::{color::palettes::css::BROWN, prelude::*};
use rand::{RngExt, SeedableRng};
use shrubbery::{bevy_plugin::ShrubberyAsset, prelude::*};
use std::time::Duration;

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum GameDefinedVoxelType {
    Bark,
    LeafBright,
    LeafDark,
}

#[derive(Clone, Default, Asset, TypePath, Debug)]
pub struct VoxelMaterial {
    voxel_name: String,
    material: Color,
}

#[derive(Resource, Debug)]
pub struct VoxelMaterials(Vec<VoxelMaterial>);

impl VoxelMaterials {
    pub fn color(&self, i: usize) -> Color {
        let Some(material) = self.0.get(i) else {
            error!("no color at i: {:?}", i);
            return Color::default();
        };
        material.material
    }
}

impl From<(&str, Srgba)> for VoxelMaterial {
    fn from(value: (&str, Srgba)) -> Self {
        Self {
            voxel_name: value.0.to_string(),
            material: Color::from(value.1),
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ShrubberyPlugin)
        .add_plugins(scene_setup::SceneSetupPlugin) // camera+lights+circular base is setup here
        .add_systems(Startup, setup)
        .add_systems(Update, spawn_on_asset_change)
        .add_systems(Update, cycle_seed)
        .add_systems(Update, update_on_press)
        .insert_resource(TreeSeed(0))
        .insert_resource(TreeSeedTimer(Timer::new(
            Duration::from_millis(3500),
            TimerMode::Repeating,
        )))
        .run();
}

/// handle to keep the tree asset always loaded in memory
#[derive(Resource)]
#[allow(dead_code)]
struct TreeAssetHandle(Handle<ShrubberyAsset>);

/// if present, swap the tree seed
#[derive(Resource)]
struct TreeSeedTimer(Timer);
#[derive(Resource)]
struct TreeSeed(pub u64);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let voxel_materials: Vec<VoxelMaterial> = [
        ("bark_bright", BROWN),
        ("bark", BROWN.with_red(0.4)),
        ("bark_dark", BROWN.with_red(0.3)),
        // ("leaf_bright", GREEN),
        ("leaf_bright", Srgba::BLACK.with_green(0.7)),
        ("leaf_mid", Srgba::BLACK.with_green(0.5)),
        ("leaf_dark", Srgba::BLACK.with_green(0.3)),
    ]
    .into_iter()
    .map(Into::<VoxelMaterial>::into)
    .collect();
    let voxel_materials = VoxelMaterials(voxel_materials);
    let mut map = ahash::HashMap::<String, VoxelId>::default();

    for (i, name) in voxel_materials
        .0
        .iter()
        .map(|v| v.voxel_name.clone())
        .enumerate()
    {
        map.insert(name, VoxelId(i as u32));
    }

    commands.insert_resource(VoxelDefinitions(map));
    commands.insert_resource(voxel_materials);

    commands.insert_resource(TreeAssetHandle(
        asset_server.load("shrubbery/oak.shrubbery.ron"),
    ));
}

/// root entity holding tree voxels
#[derive(Component)]
pub struct PreviewTreeTag;

fn cycle_seed(
    mut tree_seed: ResMut<TreeSeed>,
    time: Res<Time>,
    cycle_timer: Option<ResMut<TreeSeedTimer>>,
) {
    let Some(mut cycle_timer) = cycle_timer else {
        return;
    };
    if cycle_timer.0.tick(time.delta()).just_finished() {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(time.elapsed().as_millis() as u64);
        tree_seed.0 = rng.random();
    }
}

fn update_on_press(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut tree_seed: ResMut<TreeSeed>,
    time: Res<Time>,
) {
    if !keyboard.just_pressed(KeyCode::KeyR) {
        return;
    }
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(time.elapsed().as_millis() as u64);
    tree_seed.0 = rng.random();
}

#[allow(clippy::too_many_arguments)]
fn spawn_on_asset_change(
    mut events: MessageReader<AssetEvent<ShrubberyAsset>>,
    tree_assets: Res<Assets<ShrubberyAsset>>,
    tree_handle: Res<TreeAssetHandle>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    preview_trees: Query<Entity, With<PreviewTreeTag>>,
    voxel_materials: Res<VoxelMaterials>,
    tree_seed: Res<TreeSeed>,
) {
    let mut rebuild_tree = false;
    for event in events.read() {
        let (AssetEvent::Added { .. } | AssetEvent::Modified { .. }) = event else {
            continue;
        };
        rebuild_tree = true;
        break;
    }
    if tree_seed.is_changed() {
        rebuild_tree = true;
    }
    if rebuild_tree {
        for preview_tree_entity in preview_trees.iter() {
            commands.entity(preview_tree_entity).despawn();
        }
        let Some(tree_asset) = tree_assets.get(&tree_handle.0) else {
            return;
        };
        let mut generator = ShrubberyGenerator::generate(tree_seed.0, tree_asset);
        let voxels = generator.voxelize();

        let root_entity_id = commands
            .spawn((Transform::default(), Visibility::Visible, PreviewTreeTag))
            .id();
        for (pos, voxel_id) in voxels.into_iter() {
            let color = voxel_materials.color(voxel_id.0 as usize);

            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(1., 1., 1.))),
                MeshMaterial3d(materials.add(color)),
                Transform::from_translation(IVec3::from(pos.to_array()).as_vec3()),
                ChildOf(root_entity_id),
            ));
        }
    }
}
