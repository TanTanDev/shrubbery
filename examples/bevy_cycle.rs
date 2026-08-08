//! This example allows you to cycle through and preview
//! all "shrubbery.ron" Assets
#[path = "common/scene_setup.rs"]
mod scene_setup;

use bevy::{asset::LoadedFolder, color::palettes::css::BROWN, prelude::*};
use rand::{RngExt, SeedableRng};
use shrubbery::{bevy_plugin::ShrubberyAsset, prelude::*};

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
        .add_plugins(ShrubberyDebugDrawPlugin)
        .add_plugins(scene_setup::SceneSetupPlugin) // camera+lights+circular base is setup here
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
        // .add_systems(Update, move_light)
        .insert_resource(TreeSeed(0))
        .run();
}

/// handle to keep the tree asset always loaded in memory
#[derive(Resource)]
struct TreeAssetHandles(Vec<Handle<ShrubberyAsset>>);

#[derive(Resource)]
struct TreeSeed(pub u64);

fn setup(mut commands: Commands) {
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
}

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
        Text::new(
            "tap: R     to reset\ntap: N     to open next tree\ntap: G     toggle gizmos\ntap: H     toggle attractors\ntap: B     toggle branches",
        ),
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
        Tree {
            tree_handle: tree_handle.clone(),
        },
        ShrubberyDebugDraw {
            asset: tree_handle,
            seed: 0,
        },
    ));
}

#[allow(clippy::too_many_arguments)]
fn rebuild_tree_on_update(
    mut events: MessageReader<AssetEvent<ShrubberyAsset>>,
    tree_assets: Res<Assets<ShrubberyAsset>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    mut tree_entities: Query<(Entity, &Tree, &mut ShrubberyDebugDraw)>,
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
    for (i, (tree_entity, tree_handle, tree_asset, mut debug_draw)) in tree_entities
        .iter_mut()
        .filter_map(|(entity, tree, debug_draw)| {
            tree_assets
                .get(&tree.tree_handle)
                .map(|asset| (entity, tree.tree_handle.clone(), asset, debug_draw))
        })
        .enumerate()
    {
        // destroy child voxel entities
        commands.entity(tree_entity).despawn_children();

        let unique_tree_seed = tree_seed.0 + i as u64;
        // keep the debug overlay in sync with the regenerated tree
        if debug_draw.seed != unique_tree_seed || debug_draw.asset != tree_handle {
            debug_draw.seed = unique_tree_seed;
            debug_draw.asset = tree_handle;
        }
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
