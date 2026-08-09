//! Shrubbery file previewer / hot-reload editor.
//!
//! Tap the `file picker` button to open any `*.shrubbery.ron` file from disk
//! (outside `assets/`, so bevy's asset watcher cannot see it). The file is
//! loaded manually via `std::fs` and re-polled for changes so edits hot-reload.
//!
//! Voxel colors are owned by the host app, so unknown voxel names found in the
//! loaded file are assigned a random color and persisted to
//! `assets/editor_voxel_color_map.ron`. That file *is* under `assets/` and is
//! loaded as a real bevy asset, so manual edits to it hot-reload through
//! bevy's file watcher. Voxel names that needed a random preset are listed in
//! the warning window at the bottom left.

#[path = "common/scene_setup.rs"]
mod scene_setup;

use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use bevy::{asset::AssetLoader, prelude::*, reflect::TypePath};
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};
use shrubbery_voxel::{
    bevy_plugin::{RonLoaderError, ShrubberyAsset},
    prelude::*,
    voxel::{DecorationSelector, LeafDecoration},
};

/// asset-server-relative path of the persisted color table
const COLOR_MAP_ASSET_PATH: &str = "editor_voxel_color_map.ron";
/// filesystem path of the same file. Inside `assets/` on purpose: bevy's file
/// watcher only watches that folder, so this placement is what makes manual
/// edits to the color table hot-reload.
const COLOR_MAP_FILE_PATH: &str = "assets/editor_voxel_color_map.ron";

/// how often the open file is polled for on-disk changes, in seconds
const FILE_POLL_INTERVAL_SECS: f32 = 0.5;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ShrubberyPlugin)
        .add_plugins(ShrubberyDebugDrawPlugin)
        .add_plugins(scene_setup::SceneSetupPlugin) // camera+lights+circular base is setup here
        .init_asset::<EditorVoxelColorMapAsset>()
        .init_asset_loader::<VoxelColorMapAssetLoader>()
        .insert_resource(TreeSeed(0))
        .add_systems(Startup, (setup_editor, spawn_ui))
        .add_systems(Update, open_file_picker_on_button)
        .add_systems(Update, reload_file_on_disk_change)
        .add_systems(Update, sync_on_color_map_events)
        .add_systems(Update, rebuild_tree_on_update)
        .add_systems(Update, sync_status_text)
        .add_systems(Update, sync_notice_ui)
        .add_systems(Update, update_seed_on_press)
        .run();
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct VoxelColorEntry {
    name: String,
    red: f32,
    green: f32,
    blue: f32,
}

/// Editor-owned voxel name -> color table. An entry's index doubles as its
/// [`VoxelId`]. Lives under `assets/` so bevy's file watcher hot-reloads it.
#[derive(Clone, Debug, Default, Asset, TypePath, Serialize, Deserialize)]
#[serde(transparent)]
struct EditorVoxelColorMapAsset(Vec<VoxelColorEntry>);

impl EditorVoxelColorMapAsset {
    fn color(&self, index: usize) -> Color {
        let Some(entry) = self.0.get(index) else {
            error!("no color at index: {index}");
            return Color::WHITE;
        };
        Color::srgb(entry.red, entry.green, entry.blue)
    }
}

#[derive(Default, TypePath)]
struct VoxelColorMapAssetLoader;

impl AssetLoader for VoxelColorMapAssetLoader {
    type Asset = EditorVoxelColorMapAsset;
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
        &["ron"]
    }
}

/// handle to the loaded color map asset
#[derive(Resource)]
struct EditorVoxelColorMapHandle(Handle<EditorVoxelColorMapAsset>);

/// absolute path of the color map file, for display in the UI
#[derive(Resource)]
struct ColorMapFilePath(String);

/// voxel names that had no color preset during the most recent load; shown in
/// the bottom-left warning window. empty = window hidden.
#[derive(Resource, Default)]
struct UnknownVoxelNotice(Vec<String>);

/// the file currently being previewed
#[derive(Resource)]
struct CurrentShrubberyFile {
    path: PathBuf,
    handle: Handle<ShrubberyAsset>,
    last_modified: SystemTime,
}

#[derive(Resource)]
struct TreeSeed(pub u64);

/// root entity holding tree voxels
#[derive(Component)]
pub struct Tree {
    pub tree_handle: Handle<ShrubberyAsset>,
}

#[derive(Component)]
struct StatusText;

/// bottom-left warning window listing voxels without a color preset
#[derive(Component)]
struct NoticeWindow;

#[derive(Component)]
struct NoticeText;

/// voxel registry, and spawns the (initially empty) tree entity
fn setup_editor(mut commands: Commands, asset_server: Res<AssetServer>) {
    ensure_color_map_file_exists();
    let color_map_handle = asset_server.load::<EditorVoxelColorMapAsset>(COLOR_MAP_ASSET_PATH);
    let display_path = std::fs::canonicalize(COLOR_MAP_FILE_PATH)
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| COLOR_MAP_FILE_PATH.to_string());
    commands.insert_resource(EditorVoxelColorMapHandle(color_map_handle));
    commands.insert_resource(ColorMapFilePath(display_path));
    commands.insert_resource(VoxelDefinitions::default());
    commands.init_resource::<UnknownVoxelNotice>();
    commands.spawn((
        Transform::default(),
        Visibility::Visible,
        Tree {
            tree_handle: Handle::default(),
        },
        ShrubberyDebugDraw {
            asset: Handle::default(),
            seed: 0,
        },
    ));
}

fn spawn_ui(mut commands: Commands) {
    let container = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(8.0)),
            row_gap: Val::Px(8.0),
            ..default()
        })
        .id();
    commands.spawn((
        Text::new("tap: R     to reset seed\ntap: G     toggle gizmos\ntap: H     toggle attractors\ntap: B     toggle branches"),
        Node { ..default() },
        ChildOf(container),
    ));
    let button = commands
        .spawn((
            Button,
            Node {
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.25, 0.25, 0.3)),
            ChildOf(container),
        ))
        .id();
    commands.spawn((Text::new("file picker"), ChildOf(button)));
    commands.spawn((
        StatusText,
        Text::new("no file loaded"),
        Node { ..default() },
        ChildOf(container),
    ));

    let notice_window = commands
        .spawn((
            NoticeWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(8.0),
                bottom: Val::Px(8.0),
                padding: UiRect::all(Val::Px(10.0)),
                max_width: Val::Px(560.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.85, 0.15, 0.15)),
            Visibility::Hidden,
        ))
        .id();
    commands.spawn((
        NoticeText,
        Text::new(""),
        TextColor(Color::BLACK),
        ChildOf(notice_window),
    ));
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

#[allow(clippy::too_many_arguments)]
fn open_file_picker_on_button(
    interactions: Query<&Interaction, (Changed<Interaction>, With<Button>)>,
    current_file: Option<Res<CurrentShrubberyFile>>,
    color_map_handle: Res<EditorVoxelColorMapHandle>,
    mut color_maps: ResMut<Assets<EditorVoxelColorMapAsset>>,
    mut voxel_definitions: ResMut<VoxelDefinitions>,
    mut notice: ResMut<UnknownVoxelNotice>,
    mut tree_assets: ResMut<Assets<ShrubberyAsset>>,
    mut trees: Query<&mut Tree>,
    mut commands: Commands,
) {
    if !interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        return;
    }
    let Some(path) = rfd::FileDialog::new()
        .set_title("open shrubbery file")
        .add_filter("shrubbery", &["ron"])
        .pick_file()
    else {
        return;
    };
    let existing_handle = current_file.as_ref().map(|file| file.handle.clone());
    let Some((handle, modified)) = load_shrubbery_file(
        &path,
        color_maps.as_mut(),
        &color_map_handle.0,
        voxel_definitions.as_mut(),
        notice.as_mut(),
        tree_assets.as_mut(),
        existing_handle.as_ref(),
    ) else {
        return;
    };
    for mut tree in trees.iter_mut() {
        tree.tree_handle = handle.clone();
    }
    commands.insert_resource(CurrentShrubberyFile {
        path,
        handle,
        last_modified: modified,
    });
}

/// bevy's asset watcher only sees `assets/`, so poll the open file's mtime
/// ourselves and re-parse + re-insert it when it changes on disk
#[allow(clippy::too_many_arguments)]
fn reload_file_on_disk_change(
    current_file: Option<ResMut<CurrentShrubberyFile>>,
    mut time_since_poll: Local<f32>,
    time: Res<Time>,
    color_map_handle: Res<EditorVoxelColorMapHandle>,
    mut color_maps: ResMut<Assets<EditorVoxelColorMapAsset>>,
    mut voxel_definitions: ResMut<VoxelDefinitions>,
    mut notice: ResMut<UnknownVoxelNotice>,
    mut tree_assets: ResMut<Assets<ShrubberyAsset>>,
    mut trees: Query<&mut Tree>,
) {
    *time_since_poll += time.delta_secs();
    if *time_since_poll < FILE_POLL_INTERVAL_SECS {
        return;
    }
    *time_since_poll = 0.0;
    let Some(mut current_file) = current_file else {
        return;
    };
    let Some(modified) = file_modified_time(&current_file.path) else {
        return; // file temporarily missing (e.g. mid-save); retry next poll
    };
    if modified == current_file.last_modified {
        return;
    }
    // record the new mtime even when the reload below fails, so a broken
    // intermediate save isn't re-parsed (and re-logged) every poll
    current_file.last_modified = modified;
    info!("{} changed on disk, reloading", current_file.path.display());
    let path = current_file.path.clone();
    let handle = current_file.handle.clone();
    let Some((handle, modified)) = load_shrubbery_file(
        &path,
        color_maps.as_mut(),
        &color_map_handle.0,
        voxel_definitions.as_mut(),
        notice.as_mut(),
        tree_assets.as_mut(),
        Some(&handle),
    ) else {
        return;
    };
    current_file.last_modified = modified;
    current_file.handle = handle.clone();
    for mut tree in trees.iter_mut() {
        tree.tree_handle = handle.clone();
    }
}

/// the color map is a real bevy asset under `assets/`, so manual edits to it
/// arrive here as asset events: rebuild the voxel registry, cover any new
/// voxel names in the loaded tree, and re-resolve the tree's voxel ids
#[allow(clippy::too_many_arguments)]
fn sync_on_color_map_events(
    mut events: MessageReader<AssetEvent<EditorVoxelColorMapAsset>>,
    color_map_handle: Res<EditorVoxelColorMapHandle>,
    mut color_maps: ResMut<Assets<EditorVoxelColorMapAsset>>,
    mut voxel_definitions: ResMut<VoxelDefinitions>,
    current_file: Option<Res<CurrentShrubberyFile>>,
    mut tree_assets: ResMut<Assets<ShrubberyAsset>>,
    mut notice: ResMut<UnknownVoxelNotice>,
) {
    let mut changed = false;
    for event in events.read() {
        if let AssetEvent::Added { .. }
        | AssetEvent::Modified { .. }
        | AssetEvent::LoadedWithDependencies { .. } = event
        {
            changed = true;
            break;
        }
    }
    if !changed {
        return;
    }
    let Some(map) = color_maps.get(&color_map_handle.0) else {
        return;
    };
    *voxel_definitions = definitions_from_map(map);

    // if a tree is already loaded, make sure its voxel names are covered,
    // then re-resolve its ids against the (possibly reordered) registry
    let Some(current_file) = current_file else {
        return;
    };
    let Some(tree_asset) = tree_assets.get(&current_file.handle) else {
        return;
    };
    let names = collect_voxel_names(tree_asset);
    let new_names = ensure_colors(color_maps.as_mut(), &color_map_handle.0, &names);
    if !new_names.is_empty() {
        notice.0 = new_names;
        if let Some(map) = color_maps.get(&color_map_handle.0) {
            *voxel_definitions = definitions_from_map(map);
        }
    }
    // emits AssetEvent::Modified, which rebuilds the tree + debug gizmos
    if let Some(mut tree_asset) = tree_assets.get_mut(&current_file.handle) {
        tree_asset.0.resolve_voxel_definitions(&voxel_definitions);
    }
}

fn sync_status_text(
    current_file: Option<Res<CurrentShrubberyFile>>,
    mut texts: Query<&mut Text, With<StatusText>>,
) {
    let Some(current_file) = current_file.filter(|file| file.is_changed()) else {
        return;
    };
    let file_name = current_file
        .path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| current_file.path.display().to_string());
    for mut text in texts.iter_mut() {
        text.0 = file_name.clone();
    }
}

/// shows/hides the bottom-left warning window listing voxel names that had no
/// color preset (they were assigned random colors), plus the color file path
fn sync_notice_ui(
    notice: Res<UnknownVoxelNotice>,
    color_map_path: Res<ColorMapFilePath>,
    mut windows: Query<&mut Visibility, With<NoticeWindow>>,
    mut texts: Query<&mut Text, With<NoticeText>>,
) {
    if !notice.is_changed() {
        return;
    }
    let visible = !notice.0.is_empty();
    for mut visibility in windows.iter_mut() {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !visible {
        return;
    }
    let message = format!(
        "no color preset for: {}\nassigned random colors, edit:\n{}",
        notice.0.join(", "),
        color_map_path.0,
    );
    for mut text in texts.iter_mut() {
        text.0 = message.clone();
    }
}

fn file_modified_time(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}

/// loads + parses a shrubbery file from disk, registers colors for any new
/// voxel names, resolves voxel ids and upserts the asset. Overwriting in place
/// emits `AssetEvent::Modified` so the tree and debug gizmos rebuild.
#[allow(clippy::too_many_arguments)]
fn load_shrubbery_file(
    path: &Path,
    color_maps: &mut Assets<EditorVoxelColorMapAsset>,
    color_map_handle: &Handle<EditorVoxelColorMapAsset>,
    voxel_definitions: &mut VoxelDefinitions,
    notice: &mut UnknownVoxelNotice,
    tree_assets: &mut Assets<ShrubberyAsset>,
    existing_handle: Option<&Handle<ShrubberyAsset>>,
) -> Option<(Handle<ShrubberyAsset>, SystemTime)> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) => {
            error!("failed to read {}: {err}", path.display());
            return None;
        }
    };
    let mut asset: ShrubberyAsset = match ron::de::from_bytes(&bytes) {
        Ok(asset) => asset,
        Err(err) => {
            error!("failed to parse {}: {err}", path.display());
            return None;
        }
    };

    // register random colors for voxel names we have no mapping for yet
    let names = collect_voxel_names(&asset);
    let new_names = ensure_colors(color_maps, color_map_handle, &names);
    if !new_names.is_empty()
        && let Some(map) = color_maps.get(color_map_handle)
    {
        *voxel_definitions = definitions_from_map(map);
    }
    notice.0 = new_names;
    asset.0.resolve_voxel_definitions(voxel_definitions);

    let handle = match existing_handle {
        Some(handle) if tree_assets.get(handle).is_some() => {
            *tree_assets.get_mut(handle).expect("checked above") = asset;
            handle.clone()
        }
        _ => tree_assets.add(asset),
    };

    let Some(modified) = file_modified_time(path) else {
        error!("failed to stat {}", path.display());
        return None;
    };
    Some((handle, modified))
}

/// assigns a random color to every name the color map doesn't know yet and
/// persists the map; returns the names that were newly added
fn ensure_colors(
    color_maps: &mut Assets<EditorVoxelColorMapAsset>,
    color_map_handle: &Handle<EditorVoxelColorMapAsset>,
    names: &[String],
) -> Vec<String> {
    let missing: Vec<String> = {
        let Some(map) = color_maps.get(color_map_handle) else {
            warn!("color map asset not loaded yet");
            return Vec::new();
        };
        names
            .iter()
            .filter(|name| !map.0.iter().any(|entry| entry.name == **name))
            .cloned()
            .collect()
    };
    if missing.is_empty() {
        return missing;
    }
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    );
    let Some(mut map) = color_maps.get_mut(color_map_handle) else {
        return Vec::new();
    };
    for name in &missing {
        info!("assigning random color to unmapped voxel '{name}'");
        map.0.push(VoxelColorEntry {
            name: name.clone(),
            red: rng.random_range(0.2..=1.0),
            green: rng.random_range(0.2..=1.0),
            blue: rng.random_range(0.2..=1.0),
        });
    }
    save_color_map(&map);
    missing
}

fn save_color_map(map: &EditorVoxelColorMapAsset) {
    match ron::ser::to_string_pretty(map, ron::ser::PrettyConfig::default()) {
        Ok(serialized) => {
            if let Err(err) = std::fs::write(COLOR_MAP_FILE_PATH, serialized) {
                error!("failed to write {COLOR_MAP_FILE_PATH}: {err}");
            }
        }
        Err(err) => error!("failed to serialize color map: {err}"),
    }
}

fn ensure_color_map_file_exists() {
    if Path::new(COLOR_MAP_FILE_PATH).exists() {
        return;
    }
    if let Err(err) = std::fs::write(COLOR_MAP_FILE_PATH, "[]") {
        error!("failed to create {COLOR_MAP_FILE_PATH}: {err}");
    }
}

fn definitions_from_map(map: &EditorVoxelColorMapAsset) -> VoxelDefinitions {
    let definitions = map
        .0
        .iter()
        .enumerate()
        .map(|(i, entry)| (entry.name.clone(), VoxelId(i as u32)))
        .collect();
    VoxelDefinitions(definitions)
}

fn collect_voxel_names(settings: &ShrubberySettings) -> Vec<String> {
    let mut names = Vec::new();
    for step in &settings.build_steps {
        let decoration = match step {
            ShrubberyStep::Grow(step) => Some(&step.voxel),
            ShrubberyStep::Shape(step) => Some(&step.voxel),
            _ => None,
        };
        if let Some(decoration) = decoration {
            collect_selector_names(decoration, &mut names);
        }
    }
    names.sort();
    names.dedup();
    names
}

fn collect_selector_names(selector: &DecorationSelector, names: &mut Vec<String>) {
    match selector {
        DecorationSelector::Value(decoration) => collect_decoration_names(decoration, names),
        DecorationSelector::Random(decorations) => decorations
            .iter()
            .for_each(|decoration| collect_decoration_names(decoration, names)),
        DecorationSelector::RandomWeighted(entries) => entries
            .iter()
            .for_each(|entry| collect_decoration_names(&entry.voxel, names)),
    }
}

fn collect_decoration_names(decoration: &LeafDecoration, names: &mut Vec<String>) {
    match decoration {
        LeafDecoration::Solid(mapping) => names.push(mapping.name.clone()),
        LeafDecoration::RandomSolid(entries) => entries
            .iter()
            .for_each(|entry| names.push(entry.voxel.name.clone())),
        LeafDecoration::Gradient(settings) => settings
            .steps
            .iter()
            .for_each(|step| names.push(step.voxel.name.clone())),
    }
}

#[allow(clippy::too_many_arguments)]
fn rebuild_tree_on_update(
    mut shrubbery_events: MessageReader<AssetEvent<ShrubberyAsset>>,
    mut color_map_events: MessageReader<AssetEvent<EditorVoxelColorMapAsset>>,
    tree_assets: Res<Assets<ShrubberyAsset>>,
    color_map_handle: Res<EditorVoxelColorMapHandle>,
    color_maps: Res<Assets<EditorVoxelColorMapAsset>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    mut tree_entities: Query<(Entity, &Tree, &mut ShrubberyDebugDraw)>,
    changed_tree_entities: Query<&Tree, Changed<Tree>>,
    tree_seed: Res<TreeSeed>,
) {
    let mut rebuild_all = false;
    for event in shrubbery_events.read() {
        if let AssetEvent::Added { .. } | AssetEvent::Modified { .. } = event {
            rebuild_all = true;
            break;
        }
    }
    for event in color_map_events.read() {
        if let AssetEvent::Added { .. }
        | AssetEvent::Modified { .. }
        | AssetEvent::LoadedWithDependencies { .. } = event
        {
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
            let color = color_maps
                .get(&color_map_handle.0)
                .map(|map| map.color(voxel_id.0 as usize))
                .unwrap_or(Color::WHITE);
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(1., 1., 1.))),
                MeshMaterial3d(materials.add(color)),
                Transform::from_translation(IVec3::from(pos.to_array()).as_vec3()),
                ChildOf(tree_entity),
            ));
        }
    }
}
