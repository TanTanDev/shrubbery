use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use bevy::{
    color::palettes::css::{BROWN, DARK_GREEN, GREEN},
    prelude::*,
};
use rand::{RngExt, SeedableRng};
use shrubbery::{
    bevy_fly_cam::{FlyCam, KeyBindings, MovementSettings, NoCameraPlayerPlugin, PlayerPlugin},
    bevy_plugin::TreeSpaceColonizationAsset,
    prelude::SpaceColonizationPlugin,
    voxel::{VoxelDefinitions, VoxelId, voxelize},
};

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
        material.material.clone()
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
        .add_plugins(SpaceColonizationPlugin)
        .add_plugins(NoCameraPlayerPlugin)
        .insert_resource(MovementSettings {
            sensitivity: 0.00015, // default: 0.00012
            speed: 12.0,          // default: 12.0
        })
        .add_systems(Startup, setup)
        .add_systems(Update, spawn_on_asset_change)
        // .add_systems(Update, cycle_seed)
        .add_systems(Update, update_on_press)
        .insert_resource(TreeSeed(0))
        .insert_resource(TreeSeedTimer(Timer::new(
            Duration::from_millis(500),
            TimerMode::Repeating,
        )))
        .run();
}

/// handle to keep the tree asset always loaded in memory
#[derive(Resource)]
#[allow(dead_code)]
struct TreeAssetHandle(Handle<TreeSpaceColonizationAsset>);

/// if present, swap the tree seed
#[derive(Resource)]
struct TreeSeedTimer(Timer);
#[derive(Resource)]
struct TreeSeed(pub u64);

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let voxel_materials: Vec<VoxelMaterial> = [
        ("bark", BROWN),
        ("leaf_bright", GREEN),
        ("leaf_dark", DARK_GREEN),
    ]
    .into_iter()
    .map(Into::<VoxelMaterial>::into)
    .collect();
    let voxel_materials = VoxelMaterials(voxel_materials);
    let mut map = HashMap::<String, VoxelId>::default();

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
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-11.5, 54.5, 29.0).looking_at(Vec3::ZERO, Vec3::Y),
        FlyCam,
        // Transform::from_xyz(-11.5, 34.5, 39.0).looking_at(Vec3::ZERO, Vec3::Y),
        // Transform::from_xyz(-11.5, 164.5, 39.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    commands.insert_resource(TreeAssetHandle(
        asset_server.load("space_colonizers/fir.tree.space_colonizer.ron"),
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

fn spawn_on_asset_change(
    mut events: MessageReader<AssetEvent<TreeSpaceColonizationAsset>>,
    tree_assets: Res<Assets<TreeSpaceColonizationAsset>>,
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
        info!("rebuilding tree!");
        let now = Instant::now();
        // tree_asset.0.error_if_voxel_ids_air();
        let mut generator = tree_asset.0.make_generator(tree_seed.0);
        // let mut rng = ChaCha8Rng::seed_from_u64(0u64);
        generator.execute_all_step(&tree_asset.0);

        info!("time to build: {:?}", now.elapsed());
        let now = Instant::now();
        let voxels = voxelize(&mut generator, &tree_asset.0);
        info!("time to voxelize: {:?}", now.elapsed());

        let root_entity_id = commands
            .spawn((Transform::default(), Visibility::Visible, PreviewTreeTag))
            .id();
        for (pos, voxel_id) in voxels.into_iter() {
            let color = voxel_materials.color(voxel_id.0 as usize);
            // info!("voxel id: {:?} ", voxel_id);

            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(1., 1., 1.))),
                MeshMaterial3d(materials.add(color)),
                Transform::from_translation(IVec3::from(pos.to_array()).as_vec3()),
                ChildOf(root_entity_id),
            ));
        }
    }
}
