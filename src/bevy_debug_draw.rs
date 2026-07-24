//! Debug gizmo rendering for [`ShrubberyAsset`] recipes.
//!
//! Attach [`ShrubberyDebugDraw`] to an entity to visualize its recipe's
//! [`ShrubberyStep::SpawnRootBranch`], [`ShrubberyStep::SpawnAttractors`] and
//! [`ShrubberyStep::SpawnAttractorOnBranches`] steps with gizmos. The recipe is
//! replayed through a [`ShrubberyGenerator`] with the same seed, so the overlay
//! matches the generated tree.

use ahash::HashSet;
use bevy::{color::palettes::css, prelude::*};
use glam::Vec3 as ShrubVec3;

use crate::{
    bevy_plugin::ShrubberyAsset,
    shape::Shape,
    shrubbery::{BranchOffsetDir, ShrubberyGenerator, ShrubberySettings, ShrubberyStep},
};

/// The crate's glam version can diverge from bevy's; convert by value.
fn to_bevy_vec3(v: ShrubVec3) -> Vec3 {
    Vec3::from_array(v.to_array())
}

/// Gizmo debug rendering for shrubbery recipes. Add alongside
/// [`crate::bevy_plugin::ShrubberyPlugin`], then attach [`ShrubberyDebugDraw`]
/// to the entities you want overlays for.
pub struct ShrubberyDebugDrawPlugin;

impl Plugin for ShrubberyDebugDrawPlugin {
    fn build(&self, app: &mut App) {
        info!("ShrubberyDebugDrawPlugin added");
        app.init_gizmo_group::<ShrubberyDebugGizmoGroup>();
        app.init_resource::<ShrubberyDebugConfig>();
        app.add_systems(Startup, configure_gizmo_group);
        app.add_systems(
            Update,
            (toggle_debug_config, refresh_debug_caches, draw_debug_gizmos),
        );
    }
}

/// Gizmo group for shrubbery debug overlays, configured with a negative depth
/// bias so markers show through solid voxel geometry.
#[derive(Default, Reflect, GizmoConfigGroup)]
#[reflect(Default)]
pub struct ShrubberyDebugGizmoGroup;

fn configure_gizmo_group(store: Option<ResMut<GizmoConfigStore>>) {
    let Some(mut store) = store else {
        warn!("debug draw: GizmoConfigStore missing, is GizmoPlugin (DefaultPlugins) added?");
        return;
    };
    let (config, _) = store.config_mut::<ShrubberyDebugGizmoGroup>();
    // x-ray: markers sit inside the solid trunk voxels, draw through them
    config.depth_bias = -1.0;
}

/// Attach to an entity to draw debug gizmos for a [`ShrubberyAsset`] recipe.
///
/// `seed` must match the seed passed to [`ShrubberyGenerator::generate`] for
/// the entity's voxels, or the overlay will diverge from the generated tree.
#[derive(Component, Clone, Debug)]
pub struct ShrubberyDebugDraw {
    pub asset: Handle<ShrubberyAsset>,
    pub seed: u64,
}

/// Toggles for shrubbery debug gizmos.
#[derive(Resource, Debug, Clone)]
pub struct ShrubberyDebugConfig {
    /// Master switch for all shrubbery debug gizmos.
    pub render_gizmos: bool,
    /// Also draw the individual attractor points (visualizes spacing/jitter).
    pub render_attractors: bool,
    /// Draw branch segments as arrows, colored by the step that grew them.
    pub render_branches: bool,
    pub toggle_gizmos_key: KeyCode,
    pub toggle_attractors_key: KeyCode,
    pub toggle_branches_key: KeyCode,
    /// Radius of the root-branch marker sphere/cross. Must exceed 0.5 to
    /// protrude from the 1-unit trunk voxels even without the x-ray depth bias.
    pub root_marker_radius: f32,
    /// Radius of each attractor point marker.
    pub attractor_point_radius: f32,
}

impl Default for ShrubberyDebugConfig {
    fn default() -> Self {
        Self {
            render_gizmos: true,
            render_attractors: true,
            render_branches: true,
            toggle_gizmos_key: KeyCode::KeyG,
            toggle_attractors_key: KeyCode::KeyH,
            toggle_branches_key: KeyCode::KeyB,
            root_marker_radius: 0.75,
            attractor_point_radius: 0.25,
        }
    }
}

/// A root branch spawned by a [`ShrubberyStep::SpawnRootBranch`] step.
#[derive(Clone, Copy, Debug)]
struct RootBranchDebug {
    pos: Vec3,
    dir: Vec3,
    /// Display-only arrow length (the step's max branch length).
    display_len: f32,
}

/// An attractor volume spawned by a [`ShrubberyStep::SpawnAttractors`] or
/// [`ShrubberyStep::SpawnAttractorOnBranches`] step.
#[derive(Clone, Debug, Default)]
struct AttractorVolumeDebug {
    center: Vec3,
    half_extents: Vec3,
    attractors: Vec<Vec3>,
}

/// A non-root branch segment, colored by the step that produced it.
///
/// Stores both endpoints rather than pos+dir: the segment runs from the
/// parent's position to the branch's position, and `dir` only describes where
/// the branch *head* points next (and may be bent by later growth steps).
#[derive(Clone, Copy, Debug)]
struct BranchDebug {
    /// Position of the parent branch (segment start).
    start: Vec3,
    /// Position of this branch (segment end / tip).
    end: Vec3,
    /// Index into the recipe's `build_steps` that produced this branch.
    step_index: usize,
}

/// Arrow colors for branch-producing steps, cycled by step index.
const STEP_PALETTE: &[(&str, Srgba)] = &[
    ("aqua", css::AQUA),
    ("orange", css::ORANGE),
    ("magenta", css::MAGENTA),
    ("turquoise", css::TURQUOISE),
    ("violet", css::VIOLET),
    ("salmon", css::SALMON),
];

/// Precomputed gizmo geometry for a [`ShrubberyDebugDraw`] entity, rebuilt by
/// [`refresh_debug_caches`] whenever the component or its asset changes.
#[derive(Component, Clone, Debug, Default)]
pub struct ShrubberyDebugCache {
    root_branches: Vec<RootBranchDebug>,
    attractor_volumes: Vec<AttractorVolumeDebug>,
    branches: Vec<BranchDebug>,
    /// (step index, step name, branch count) for the color legend log.
    branch_legend: Vec<(usize, &'static str, usize)>,
}

fn step_name(step: &ShrubberyStep) -> &'static str {
    match step {
        ShrubberyStep::SpawnRootBranch(_) => "SpawnRootBranch",
        ShrubberyStep::GrowDirection(_) => "GrowDirection",
        ShrubberyStep::GrowRadial(_) => "GrowRadial",
        ShrubberyStep::GrowToAttractors(_) => "GrowToAttractors",
        ShrubberyStep::SpawnAttractors(_) => "SpawnAttractors",
        ShrubberyStep::SpawnAttractorOnBranches(_) => "SpawnAttractorOnBranches",
        ShrubberyStep::ClearAttractors => "ClearAttractors",
        ShrubberyStep::SpawnLeaves(_) => "SpawnLeaves",
    }
}

/// Replay the recipe step-by-step, capturing debug geometry as it is spawned.
/// Executing with the same seed reproduces the generator's rng stream, so the
/// captured positions match the real tree.
fn compute_debug_cache(seed: u64, settings: &ShrubberySettings) -> ShrubberyDebugCache {
    let mut generator = ShrubberyGenerator::new(seed);
    let mut cache = ShrubberyDebugCache::default();
    // (branch index, producing step index); positions/dirs are read after the
    // replay so later growth adjustments still show the final directions
    let mut branch_steps: Vec<(usize, usize)> = Vec::new();
    for (step_index, step) in settings.build_steps.iter().enumerate() {
        let branch_start = generator.branches.len();
        let attractor_start = generator.attractors.len();
        generator.execute_step(step);
        branch_steps.extend((branch_start..generator.branches.len()).map(|i| (i, step_index)));
        match step {
            ShrubberyStep::SpawnRootBranch(params) => {
                let display_len = params.branch_len.max();
                cache
                    .root_branches
                    .extend(generator.branches[branch_start..].iter().map(|branch| {
                        RootBranchDebug {
                            pos: to_bevy_vec3(branch.pos),
                            dir: to_bevy_vec3(branch.dir),
                            display_len,
                        }
                    }));
            }
            ShrubberyStep::SpawnAttractors(params) => {
                let half_extents = match &params.shape {
                    Shape::Cube(cube) => Vec3::new(cube.size_x, cube.size_y, cube.size_z) * 0.5,
                };
                cache.attractor_volumes.push(AttractorVolumeDebug {
                    center: to_bevy_vec3(params.pos),
                    half_extents,
                    attractors: generator.attractors[attractor_start..]
                        .iter()
                        .map(|attractor| to_bevy_vec3(attractor.pos))
                        .collect(),
                });
            }
            ShrubberyStep::SpawnAttractorOnBranches(params) => {
                let new_attractors = &generator.attractors[attractor_start..];
                // empty when the step's chance roll skipped it for this seed
                if new_attractors.is_empty() {
                    continue;
                }
                // Replicate the center placement from
                // `ShrubberyGenerator::spawn_attractors_on_branches`. The step
                // only appends attractors, so branch state is unchanged and
                // the origins can be selected after execution.
                let centers: Vec<ShrubVec3> = generator
                    .branches
                    .iter()
                    .filter(|branch| branch.parent_index.is_some())
                    .filter(|branch| {
                        params
                            .filter
                            .should_include_branch(branch, generator.last_known_id)
                    })
                    .map(|branch| {
                        let offset_dir = match &params.offset_dir {
                            BranchOffsetDir::BranchForward => branch.dir.normalize_or(ShrubVec3::Y),
                            BranchOffsetDir::WorldUp => ShrubVec3::Y,
                            BranchOffsetDir::BranchForwardFlat => {
                                ShrubVec3::new(branch.dir.x, 0.0, branch.dir.z)
                                    .normalize_or(ShrubVec3::X)
                            }
                            BranchOffsetDir::WorldDown => ShrubVec3::NEG_Y,
                        };
                        branch.pos + offset_dir * params.offset_distance
                    })
                    .collect();
                let half_extents = match &params.shape {
                    Shape::Cube(cube) => Vec3::new(cube.size_x, cube.size_y, cube.size_z) * 0.5,
                };
                // attractors are appended one volume at a time, in branch order
                let per_volume: usize = match &params.shape {
                    Shape::Cube(cube) => {
                        let spacing = params.attractor_spacing.attractor_spacing.max(0.001);
                        [cube.size_x, cube.size_y, cube.size_z]
                            .map(|v| (v / spacing).ceil() as usize)
                            .iter()
                            .product()
                    }
                };
                for (i, center) in centers.into_iter().enumerate() {
                    let start = (i * per_volume).min(new_attractors.len());
                    let end = ((i + 1) * per_volume).min(new_attractors.len());
                    cache.attractor_volumes.push(AttractorVolumeDebug {
                        center: to_bevy_vec3(center),
                        half_extents,
                        attractors: new_attractors[start..end]
                            .iter()
                            .map(|attractor| to_bevy_vec3(attractor.pos))
                            .collect(),
                    });
                }
            }
            _ => {}
        }
    }
    for (index, step_index) in branch_steps {
        let branch = &generator.branches[index];
        // roots are covered by the root branch markers
        let Some(parent) = branch.parent_index else {
            continue;
        };
        cache.branches.push(BranchDebug {
            start: to_bevy_vec3(generator.branches[parent].pos),
            end: to_bevy_vec3(branch.pos),
            step_index,
        });
        match cache
            .branch_legend
            .iter_mut()
            .find(|(i, _, _)| *i == step_index)
        {
            Some((_, _, count)) => *count += 1,
            None => cache.branch_legend.push((
                step_index,
                step_name(&settings.build_steps[step_index]),
                1,
            )),
        }
    }
    cache
}

fn toggle_debug_config(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<ShrubberyDebugConfig>,
) {
    if keyboard.just_pressed(config.toggle_gizmos_key) {
        config.render_gizmos = !config.render_gizmos;
        info!("debug draw: render_gizmos = {}", config.render_gizmos);
    }
    if keyboard.just_pressed(config.toggle_attractors_key) {
        config.render_attractors = !config.render_attractors;
        info!(
            "debug draw: render_attractors = {}",
            config.render_attractors
        );
    }
    if keyboard.just_pressed(config.toggle_branches_key) {
        config.render_branches = !config.render_branches;
        info!("debug draw: render_branches = {}", config.render_branches);
    }
}

fn refresh_debug_caches(
    mut commands: Commands,
    mut events: MessageReader<AssetEvent<ShrubberyAsset>>,
    assets: Res<Assets<ShrubberyAsset>>,
    query: Query<(Entity, Ref<ShrubberyDebugDraw>)>,
    mut last_entity_count: Local<usize>,
) {
    if *last_entity_count != query.iter().len() {
        *last_entity_count = query.iter().len();
        info!(
            "debug draw: {} entities with ShrubberyDebugDraw",
            *last_entity_count
        );
    }
    // asset ids that (re)loaded this frame, e.g. hot-reloaded .ron files
    let mut asset_updates = HashSet::default();
    for event in events.read() {
        if let AssetEvent::Added { id }
        | AssetEvent::Modified { id }
        | AssetEvent::LoadedWithDependencies { id } = event
        {
            asset_updates.insert(*id);
        }
    }
    for (entity, draw) in &query {
        if !draw.is_changed() && !asset_updates.contains(&draw.asset.id()) {
            continue;
        }
        let Some(asset) = assets.get(&draw.asset) else {
            warn!(
                "debug draw: asset {:?} for entity {entity} is not loaded yet, \
                 will retry when it changes or loads",
                draw.asset.id()
            );
            continue;
        };
        let cache = compute_debug_cache(draw.seed, asset);
        info!(
            "debug draw: rebuilt cache for {entity} (seed {}): {} root branches{:?}, \
             {} attractor volumes{:?}, {} branch segments",
            draw.seed,
            cache.root_branches.len(),
            cache.root_branches.first().map(|r| r.pos),
            cache.attractor_volumes.len(),
            cache.attractor_volumes.first().map(|v| (
                v.center,
                v.half_extents * 2.0,
                v.attractors.len()
            )),
            cache.branches.len(),
        );
        for (step_index, name, count) in &cache.branch_legend {
            let (color_name, _) = STEP_PALETTE[*step_index % STEP_PALETTE.len()];
            info!("debug draw:   step #{step_index} {name}: {count} branch arrows ({color_name})");
        }
        commands.entity(entity).insert(cache);
    }
}

fn draw_debug_gizmos(
    mut gizmos: Gizmos<ShrubberyDebugGizmoGroup>,
    config: Res<ShrubberyDebugConfig>,
    query: Query<(&GlobalTransform, &ShrubberyDebugCache)>,
    mut last_state: Local<Option<(bool, bool, bool, usize)>>,
) {
    // log only on state changes, not every frame
    let state = (
        config.render_gizmos,
        config.render_attractors,
        config.render_branches,
        query.iter().len(),
    );
    if *last_state != Some(state) {
        info!(
            "debug draw: drawing frame with render_gizmos={}, render_attractors={}, \
             render_branches={}, entities with cache={}",
            state.0, state.1, state.2, state.3
        );
        *last_state = Some(state);
    }
    if !config.render_gizmos {
        return;
    }
    for (transform, cache) in &query {
        for root in &cache.root_branches {
            let pos = transform.transform_point(root.pos);
            gizmos.sphere(pos, config.root_marker_radius, css::ORANGE_RED);
            gizmos.cross(pos, config.root_marker_radius, css::ORANGE_RED);
            gizmos.arrow(pos, pos + root.dir * root.display_len, css::LIMEGREEN);
        }
        for volume in &cache.attractor_volumes {
            let center = transform.transform_point(volume.center);
            gizmos.cuboid(
                Transform::from_translation(center)
                    .with_rotation(transform.rotation())
                    .with_scale(volume.half_extents * 2.0),
                css::YELLOW,
            );
            if config.render_attractors {
                for attractor in &volume.attractors {
                    gizmos.sphere(
                        transform.transform_point(*attractor),
                        config.attractor_point_radius,
                        css::GOLD,
                    );
                }
            }
        }
        if config.render_branches {
            for branch in &cache.branches {
                let (_, color) = STEP_PALETTE[branch.step_index % STEP_PALETTE.len()];
                gizmos.arrow(
                    transform.transform_point(branch.start),
                    transform.transform_point(branch.end),
                    color,
                );
            }
        }
    }
}
