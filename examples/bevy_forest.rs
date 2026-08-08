use std::f32::consts::PI;

use bevy::{
    asset::LoadedFolder,
    color::palettes::css::{BROWN, WHITE, YELLOW},
    light::CascadeShadowConfigBuilder,
    prelude::*,
};
use rand::{RngExt, SeedableRng, seq::IndexedRandom};
use shrubbery::{
    bevy_fly_cam::{FlyCamera, FlyCameraPlugin},
    bevy_plugin::ShrubberyAsset,
    prelude::*,
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
        .add_plugins(FlyCameraPlugin)
        .add_systems(Startup, setup)
        .add_systems(Startup, setup_assets)
        .add_systems(Update, setup_forest)
        .add_systems(Update, handle_tree_folder_loaded)
        .add_systems(Update, rebuild_tree_on_update)
        .add_systems(Update, update_seed_on_press)
        .add_systems(Update, move_light)
        .insert_resource(TreeSeed(0))
        .run();
}

/// handle to keep the tree asset always loaded in memory
#[derive(Resource)]
#[allow(dead_code)]
struct TreeAssetHandles(Vec<Handle<ShrubberyAsset>>);

#[derive(Resource)]
struct TreeSeed(pub u64);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
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
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 90.5, 90.0).looking_at(Vec3::ZERO, Vec3::Y),
        FlyCamera::default(),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Circle::new(100.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    commands.insert_resource(AmbientLight {
        color: WHITE.into(),
        brightness: 600.0,
        affects_lightmapped_meshes: true,
    });

    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 4.0,
            maximum_distance: 10.0,
            ..default()
        }
        .build(),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        PointLight {
            shadows_enabled: true,
            color: YELLOW.into(),
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
        RotatingLightTag,
    ));
}

fn move_light(mut query: Query<&mut Transform, With<RotatingLightTag>>, time: Res<Time>) {
    let speed = 5.0;
    let rotation = time.elapsed_secs() * speed;
    let quat = Quat::from_rotation_y(rotation);
    let distance = 14.0;
    let boom = Vec3::X * distance;
    for mut transform in query.iter_mut() {
        transform.translation = quat * boom;
        transform.translation.y = 8.0;
    }
}

#[derive(Component)]
pub struct RotatingLightTag;

/// root entity holding tree voxels
#[derive(Component)]
pub struct Tree {
    pub tree_handle: Handle<ShrubberyAsset>,
}

fn update_seed_on_press(
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

#[derive(Resource)]
pub struct TreeFolderHandle(Handle<LoadedFolder>);

fn setup_assets(asset_server: Res<AssetServer>, mut commands: Commands) {
    let tree_folder_handle = asset_server.load_folder("shrubbery");
    commands.insert_resource(TreeFolderHandle(tree_folder_handle));
}

fn handle_tree_folder_loaded(
    tree_folder_handle: Option<Res<TreeFolderHandle>>,
    loaded_folders: Res<Assets<LoadedFolder>>,
    mut commands: Commands,
) {
    let Some(tree_folder_handle) = tree_folder_handle else {
        return;
    };
    let Some(loaded_folder) = loaded_folders.get(&tree_folder_handle.0) else {
        return;
    };

    let mut trees = vec![];
    for handle in loaded_folder.handles.iter() {
        match handle.clone().try_typed::<ShrubberyAsset>() {
            Ok(handle) => {
                trees.push(handle);
            }
            Err(_) => {
                panic!("asset type not ShrubberyAsset");
            }
        }
    }
    commands.insert_resource(TreeAssetHandles(trees));
    commands.remove_resource::<TreeFolderHandle>();
}

fn setup_forest(
    mut commands: Commands,
    tree_handles: Option<Res<TreeAssetHandles>>,
    mut is_trees_spawned: Local<bool>,
) {
    if *is_trees_spawned {
        return;
    }
    let Some(tree_handles) = tree_handles else {
        return;
    };
    let tree_count = 20;
    let mut rng = rand::rng();
    let area = 50.0;
    for _i in 0..tree_count {
        let x = rng.random_range(-area..=area);
        let z = rng.random_range(-area..=area);

        // find real tree handle
        let tree_handle = tree_handles
            .0
            .choose(&mut rng)
            .expect("non-empty tree assets")
            .clone();
        commands.spawn((
            Transform::from_translation(vec3(x, 0.0, z)),
            Visibility::Visible,
            Tree { tree_handle },
        ));
    }
    *is_trees_spawned = true;
}

#[allow(clippy::too_many_arguments)]
fn rebuild_tree_on_update(
    mut events: MessageReader<AssetEvent<ShrubberyAsset>>,
    tree_assets: Res<Assets<ShrubberyAsset>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    tree_entities: Query<(Entity, &Tree)>,
    voxel_materials: Res<VoxelMaterials>,
    tree_seed: Res<TreeSeed>,
) {
    let mut rebuild_all = false;
    for event in events.read() {
        if let AssetEvent::Added { .. } | AssetEvent::Modified { .. } = event {
            rebuild_all = true;
            break;
        }
    }
    if tree_seed.is_changed() {
        rebuild_all = true;
    }
    if !rebuild_all {
        return;
    }
    info!("building all voxel trees");
    for (i, (tree_entity, tree_asset)) in tree_entities
        .iter()
        .filter_map(|(entity, tree)| {
            tree_assets
                .get(&tree.tree_handle)
                .map(|asset| (entity, asset))
        })
        .enumerate()
    {
        // destroy child voxel entities
        commands.entity(tree_entity).despawn_children();

        let unique_tree_seed = tree_seed.0 + i as u64;
        let mut generator = ShrubberyGenerator::generate(unique_tree_seed, tree_asset);
        let voxels = generator.voxelize();

        for (pos, voxel_id) in voxels.into_iter() {
            let color = voxel_materials.color(voxel_id.0 as usize);
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(1., 1., 1.))),
                MeshMaterial3d(materials.add(color)),
                Transform::from_translation(IVec3::from(pos.to_array()).as_vec3()),
                ChildOf(tree_entity),
            ));
        }
    }
}
