use std::{collections::HashMap, f32::consts::PI};

use bevy::{
    asset::LoadedFolder,
    color::palettes::css::{BROWN, WHITE, YELLOW},
    light::CascadeShadowConfigBuilder,
    prelude::*,
};
use rand::{RngExt, SeedableRng};
use shrubbery::{
    bevy_fly_cam::{FlyCam, MovementSettings, NoCameraPlayerPlugin},
    bevy_plugin::ShrubberyAsset,
    prelude::ShrubberyPlugin,
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
        .add_plugins(ShrubberyPlugin)
        .add_plugins(NoCameraPlayerPlugin)
        .insert_resource(MovementSettings {
            sensitivity: 0.00015,
            speed: 64.0,
        })
        .add_systems(Startup, setup)
        .add_systems(Startup, setup_assets)
        .add_systems(Startup, spawn_tree)
        .add_systems(Startup, spawn_ui)
        .add_systems(Update, next_tree_on_press)
        .add_systems(Update, handle_tree_folder_loaded)
        .add_systems(Update, sync_ui_text)
        .add_systems(Update, setup_tree_on_assets_loaded)
        .add_systems(Update, rebuild_tree_on_update)
        .add_systems(Update, update_seed_on_press)
        .add_systems(Update, move_light)
        .insert_resource(TreeSeed(0))
        .run();
}

/// handle to keep the tree asset always loaded in memory
#[derive(Resource)]
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
        ("leaf_bright", Srgba::BLACK.with_green(0.7)),
        ("leaf_mid", Srgba::BLACK.with_green(0.5)),
        ("leaf_dark", Srgba::BLACK.with_green(0.3)),
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
        Transform::from_xyz(0.0, 90.5, 90.0).looking_at(Vec3::ZERO, Vec3::Y),
        FlyCam,
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

#[derive(Component)]
struct UiTextTag;

fn spawn_ui(mut commands: Commands) {
    let container = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    commands.spawn((
        Text::new("tap: R     to reset\ntap: N     to open next tree"),
        Node { ..default() },
        ChildOf(container),
    ));
    commands.spawn((
        UiTextTag,
        Text::new("NO TREE ASSIGNED"),
        Node { ..default() },
        ChildOf(container),
    ));
}

// if a tree spawns pre knowing what tree assets exist, set it once it loads
fn setup_tree_on_assets_loaded(
    tree_handles: Option<Res<TreeAssetHandles>>,
    mut is_loaded: Local<bool>,
    mut query: Query<&mut Tree>,
) {
    if *is_loaded {
        return;
    }
    let Some(tree_handles) = tree_handles else {
        return;
    };
    for mut tree in query.iter_mut() {
        tree.tree_handle = tree_handles.0.first().expect("any tree asset").clone();
        info!("overwrite existing trree");
    }
    info!("forced tree handles");
    *is_loaded = true;
}

fn sync_ui_text(
    query: Query<&Tree, Changed<Tree>>,
    mut texts: Query<&mut Text, With<UiTextTag>>,
    asset_server: Res<AssetServer>,
) {
    for tree in query {
        let Some(asset_path) = asset_server.get_path(&tree.tree_handle) else {
            // the first tree, might have a null tree handle
            continue;
        };
        let asset_name = asset_path.path().file_name().unwrap().to_string_lossy();
        for mut text in texts.iter_mut() {
            text.0 = asset_name.to_string();
        }
    }
}

fn next_tree_on_press(
    keyboard: Res<ButtonInput<KeyCode>>,
    tree_handles: Option<Res<TreeAssetHandles>>,
    query: Query<&mut Tree>,
) {
    if !keyboard.just_pressed(KeyCode::KeyN) {
        return;
    }
    let Some(tree_handles) = tree_handles else {
        error!("TreeAssetHandles not inserted");
        return;
    };
    for mut tree in query {
        let asset_index: usize = tree_handles
            .0
            .iter()
            .position(|handle| handle.id() == tree.tree_handle.id())
            .unwrap_or(0);
        let mut next_index = asset_index + 1;
        if next_index >= tree_handles.0.len() {
            next_index = 0;
        }
        tree.tree_handle = tree_handles.0.get(next_index).expect("wtf").clone();
    }
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

fn spawn_tree(mut commands: Commands, tree_handles: Option<Res<TreeAssetHandles>>) {
    let tree_handle = tree_handles.map_or(Handle::default(), |handles| {
        handles.0.first().expect("1 single asset").clone()
    });
    commands.spawn((
        Transform::default(),
        Visibility::Visible,
        Tree { tree_handle },
    ));
}

fn rebuild_tree_on_update(
    mut events: MessageReader<AssetEvent<ShrubberyAsset>>,
    tree_assets: Res<Assets<ShrubberyAsset>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    tree_entities: Query<(Entity, &Tree)>,
    changed_tree_entities: Query<&Tree, Changed<Tree>>,
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
    if !changed_tree_entities.is_empty() {
        rebuild_all = true;
    }
    if !rebuild_all {
        return;
    }
    for (i, (tree_entity, tree_asset)) in tree_entities
        .iter()
        .filter_map(|(entity, tree)| {
            tree_assets
                .get(&tree.tree_handle)
                .and_then(|asset| Some((entity, asset)))
        })
        .enumerate()
    {
        // destroy child voxel entities
        commands.entity(tree_entity).despawn_children();

        let unique_tree_seed = tree_seed.0 + i as u64;
        let mut generator = tree_asset.0.make_generator(unique_tree_seed);
        generator.execute_all_step(&tree_asset.0);
        let voxels = voxelize(&mut generator, unique_tree_seed);

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
