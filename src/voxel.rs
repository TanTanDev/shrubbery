//! voxel id management
use ahash::HashMap;

#[cfg(feature = "bevy")]
use bevy::ecs::resource::Resource;
use glam::{IVec3, Vec3, ivec3, vec3};
use rand::{RngExt, SeedableRng, seq::IndexedRandom};

use rand_chacha::ChaCha8Rng;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{prelude::ShrubberyGenerator, value_or_range::ValueOrRangeF32};
const EPSILON: f32 = 0.0001;

/// Logs through bevy when the `bevy` feature is on, otherwise to stderr —
macro_rules! log_error {
    ($($t:tt)*) => {{
        #[cfg(feature = "bevy")]
        { bevy::log::error!($($t)*); }
        #[cfg(not(feature = "bevy"))]
        { eprintln!($($t)*); }
    }};
}

/// Logs through bevy when the `bevy` feature is on, otherwise to stderr —
macro_rules! log_warn {
    ($($t:tt)*) => {{
        #[cfg(feature = "bevy")]
        { bevy::log::warn!($($t)*); }
        #[cfg(not(feature = "bevy"))]
        { eprintln!($($t)*); }
    }};
}

/// The raw voxel data representation that Shrubbery produce
/// [`VoxelDefinitions`] holds a registry mapping the names to the id.
/// [`VoxelDefinitions::id_from_name`].
#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VoxelId(pub u32);

/// Used to serialize a "pretty" voxel name
/// to later be resolved into a runtime friendly VoxelId
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct VoxelMapping {
    #[cfg_attr(feature = "serde", serde(default))]
    pub name: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub id: VoxelId,
}

impl VoxelMapping {
    pub fn resolve(&mut self, definitions: &VoxelDefinitions) {
        self.id = definitions.id_from_name(self.name.as_str());
    }
}

/// Registry mapping voxel name to runtime friendly [`VoxelId`]s.
/// Built by the host application; voxelize output is only meaningful once every
/// referenced name has been resolved against one of these.
#[derive(Default)]
#[cfg_attr(feature = "bevy", derive(Resource))]
pub struct VoxelDefinitions(pub HashMap<String, VoxelId>);

impl VoxelDefinitions {
    pub fn get_id_from_name(&self, name: &str) -> Option<VoxelId> {
        self.0.get(name).copied()
    }
    pub fn id_from_name(&self, name: &str) -> VoxelId {
        self.0.get(name).copied().unwrap_or_else(|| {
            log_warn!("no named voxel: '{}' in VoxelDefinitions", name);
            VoxelId(0u32)
        })
    }
}

/// the [`LeafDecoration::RandomSolid`] entry for randomizing voxel, weighted
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RandomVoxelEntry {
    pub weight: i32,
    pub voxel: VoxelMapping,
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Axis {
    X,
    #[default]
    Y,
    Z,
}

/// Perturbs the gradient sampling position to break up banding.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LeafGradientModulation {
    Random { percent_offset: f32 },
    Wave { frequency: f32, amplitude: f32 },
}

/// Describes how [`LeafDecoration::Gradient`] should classify voxel selection
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LeafGradientSamplingMethod {
    World { axis: Axis },
    IterationPercent,
}

impl Default for LeafGradientSamplingMethod {
    fn default() -> Self {
        Self::World {
            axis: Axis::default(),
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LeafGradientSettings {
    #[cfg_attr(feature = "serde", serde(default))]
    pub sampling_method: LeafGradientSamplingMethod,
    #[cfg_attr(feature = "serde", serde(default))]
    pub modulation: Option<LeafGradientModulation>,
    pub steps: Vec<LeafGradientEntry>,
}

/// Entry explaining at what threshold to pick what voxel
/// used by [`LeafGradientSettings`]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LeafGradientEntry {
    /// Upper threshold (0..1) at which this colour stops applying.
    pub percent: f32,
    pub voxel: VoxelMapping,
}

/// Entry for [`DecorationSelector::RandomWeighted`]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WeightedDecorationEntry {
    pub weight: u32,
    pub voxel: LeafDecoration,
}

/// Describes how to select voxel/s
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DecorationSelector {
    Value(LeafDecoration),
    Random(Vec<LeafDecoration>),
    RandomWeighted(Vec<WeightedDecorationEntry>),
}

impl Default for DecorationSelector {
    fn default() -> Self {
        Self::Value(LeafDecoration::Solid(VoxelMapping {
            name: "bark".to_string(),
            id: VoxelId::default(),
        }))
    }
}

impl DecorationSelector {
    pub fn resolve(&mut self, voxel_definitions: &VoxelDefinitions) {
        match self {
            DecorationSelector::Value(leaf_decoration) => {
                leaf_decoration.resolve(voxel_definitions)
            }
            DecorationSelector::Random(leaf_decorations) => leaf_decorations
                .iter_mut()
                .for_each(|decor| decor.resolve(voxel_definitions)),
            DecorationSelector::RandomWeighted(weighted_decoration_entry) => {
                weighted_decoration_entry
                    .iter_mut()
                    .for_each(|we| we.voxel.resolve(voxel_definitions))
            }
        }
    }

    fn get_leaf_decoration(&self, rng: &mut ChaCha8Rng) -> Option<&LeafDecoration> {
        match self {
            DecorationSelector::Value(leaf_decoration) => Some(leaf_decoration),
            DecorationSelector::Random(leaf_decorations) => leaf_decorations.choose(rng),
            DecorationSelector::RandomWeighted(items) => items
                .choose_weighted(rng, |i| i.weight)
                .map(|i| &i.voxel)
                .ok(),
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LeafDecoration {
    Solid(VoxelMapping),
    RandomSolid(Vec<RandomVoxelEntry>),
    Gradient(LeafGradientSettings),
}

impl Default for LeafDecoration {
    fn default() -> Self {
        Self::Solid(VoxelMapping::default())
    }
}

impl LeafDecoration {
    pub fn resolve(&mut self, voxel_definitions: &VoxelDefinitions) {
        match self {
            LeafDecoration::Solid(voxel_mapping) => voxel_mapping.resolve(voxel_definitions),
            LeafDecoration::RandomSolid(items) => {
                items
                    .iter_mut()
                    .for_each(|entry| entry.voxel.resolve(voxel_definitions));
            }
            LeafDecoration::Gradient(settings) => {
                settings
                    .steps
                    .iter_mut()
                    .for_each(|entry| entry.voxel.resolve(voxel_definitions));
            }
        }
    }

    fn get_voxel_id(
        &self,
        rng: &mut ChaCha8Rng,
        sample_pos: Vec3,
        bounds_min: Vec3,
        bounds_max: Vec3,
        iteration_percent: f32,
    ) -> VoxelId {
        match self {
            LeafDecoration::Solid(m) => m.id,
            LeafDecoration::RandomSolid(items) => items
                .choose_weighted(rng, |i| i.weight)
                .map(|v| v.voxel.id)
                .unwrap_or_default(),
            LeafDecoration::Gradient(gradient_settings) => {
                let mut percent = match &gradient_settings.sampling_method {
                    LeafGradientSamplingMethod::World { axis } => {
                        let (low, high, pos_v) = match axis {
                            Axis::X => (bounds_min.x, bounds_max.x, sample_pos.x),
                            Axis::Y => (bounds_min.y, bounds_max.y, sample_pos.y),
                            Axis::Z => (bounds_min.z, bounds_max.z, sample_pos.z),
                        };
                        ((pos_v - low) / (high - low)).clamp(0.0, 1.0)
                    }
                    LeafGradientSamplingMethod::IterationPercent => iteration_percent,
                };

                if let Some(modulation) = &gradient_settings.modulation {
                    percent += match modulation {
                        LeafGradientModulation::Random { percent_offset } => {
                            rng.random_range(-*percent_offset..=*percent_offset)
                        }
                        LeafGradientModulation::Wave {
                            frequency,
                            amplitude,
                        } => {
                            let coord = (sample_pos.x + sample_pos.z) * frequency;
                            coord.sin() * amplitude
                        }
                    };
                }
                percent = percent.clamp(0.0, 1.0);

                let mut selected = VoxelId::default();
                for step in gradient_settings.steps.iter() {
                    if percent > step.percent {
                        continue;
                    }
                    selected = step.voxel.id;
                    break;
                }
                selected
            }
        }
    }
}

/// Describes a shape, to later be voxelized
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Shape {
    Sphere { radius: ValueOrRangeF32 },
    ConiferWhorl(ConiferWhorlShape),
    StarLeaf(StarLeafShape),
}

/// the shape data of a [`Shape::StarLeaf`]
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StarLeafShape {
    pub arm_length: ValueOrRangeF32,
    pub arm_width: f32,
    pub branch_sharpness: f32,
    pub thickness: u32,
    pub droop: f32,
    pub tip_lift: f32,
}

/// How the whorl radius/length scales from base to apex.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ConiferTaper {
    /// Taper by Y-height: at `min_y` arms are full size, at `max_y` they are
    /// zero.  Swap min_y/max_y to invert (arms grow *larger* toward the apex).
    Height { min_y: f32, max_y: f32 },
    /// Taper by generation: generation 0 → full size, `max_generation` → zero.
    Generation { max_generation: i32 },
    /// No taper — all whorls are the same size regardless of height/generation.
    None,
}

/// Parameters for the `ConiferWhorl` leaf shape.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ConiferWhorlShape {
    /// Maximum arm reach from the branch tip at the base of the canopy.
    pub max_branch_length: f32,
    /// Maximum arm half-width (perpendicular) at the base.
    pub max_branch_width: f32,
    /// Arm taper rate: 0 = parallel sides, 1 = perfect triangle.
    pub branch_sharpness: f32,
    /// Voxel layers per whorl (thickness).
    pub branch_thickness: u32,
    /// Y-downward droop per unit of horizontal distance from the branch centre.
    /// Positive = tips droop down (classic fir), negative = tips curve up.
    pub branch_droop: f32,
    /// Extra upward Y lift applied at the arm tip, creating a concave-up arc.
    /// Positive = tips curve upward (tree 3 / tree 7 style), zero = straight.
    pub tip_lift: f32,
    /// Angle (radians) added to whorl rotation per branch index.
    /// ~2.39996 (golden angle) minimises overlap between whorls.
    pub rotation_step: f32,
    /// Per-whorl random length jitter as a fraction of `max_branch_length`.
    pub length_jitter_ratio: f32,
    /// What drives the taper from fat (base) to slim (apex).
    pub taper: ConiferTaper,
    /// Distance between whorl layers: 1 = dense, 2= one air space between layers
    pub whorl_spacing: f32,
}

/// Largest leaf radius across all leaf groups, for bounding-box padding.
fn leaf_padding(generator: &ShrubberyGenerator) -> i32 {
    generator
        .leaf_groups
        .iter()
        .map(|(shape, _)| match shape {
            Shape::Sphere { radius: r } => r.max().ceil() as i32,
            Shape::ConiferWhorl(w) => w.max_branch_length.ceil() as i32,
            Shape::StarLeaf(s) => s.arm_length.max().ceil() as i32,
        })
        .max()
        .unwrap_or(0)
}

type BranchMap = ahash::HashMap<IVec3, (f32, VoxelId)>;
type VoxelMap = ahash::HashMap<IVec3, VoxelId>;

impl ShrubberyGenerator {
    /// Rasterize the generated branches and shapes into a flat set of voxels.
    pub fn voxelize(&mut self) -> Vec<(IVec3, VoxelId)> {
        let (mut min_bounds, mut max_bounds) = self.bounds();
        let padding = leaf_padding(self);
        min_bounds -= IVec3::splat(padding);
        max_bounds += IVec3::splat(padding);

        let mut voxels = BranchMap::default();
        process_branches(self, &mut voxels);
        let mut voxel_map: VoxelMap = voxels
            .into_iter()
            .map(|(pos, (_dist, voxel_id))| (pos, voxel_id))
            .collect();
        process_shapes(self, &mut voxel_map);

        voxel_map.into_iter().collect()
    }
}

fn process_sphere_leaves(
    generator: &ShrubberyGenerator,
    out: &mut VoxelMap,
    leaf_index: usize,
    leaf_decoration: &LeafDecoration,
    radius: &ValueOrRangeF32,
) {
    for (branch_index, branch) in generator
        .branches
        .iter()
        .enumerate()
        .filter(|(_, b)| b.leaf_group == Some(leaf_index))
    {
        let mut rng = ChaCha8Rng::seed_from_u64(branch_index as u64 + generator.seed);
        let r = radius.get(&mut rng);
        let ri = r.ceil() as i32 + 1;
        let iteration_percent = branch.iteration as f32 / branch.iteration_total as f32;
        let bounds_min = branch.pos - Vec3::splat(r);
        let bounds_max = branch.pos + Vec3::splat(r);

        for dx in -ri..=ri {
            for dy in -ri..=ri {
                for dz in -ri..=ri {
                    let offset = vec3(dx as f32, dy as f32, dz as f32);
                    if offset.length_squared() > (r + EPSILON).powi(2) {
                        continue;
                    }
                    let world_f32 = branch.pos + offset;
                    let world_i32 = world_f32.floor().as_ivec3();
                    // First branch to claim a cell wins; later leaves don't overwrite.
                    if out.contains_key(&world_i32) {
                        continue;
                    }

                    out.insert(
                        world_i32,
                        leaf_decoration.get_voxel_id(
                            &mut rng,
                            world_f32,
                            bounds_min,
                            bounds_max,
                            iteration_percent,
                        ),
                    );
                }
            }
        }
    }
}

fn voxelize_star_leaves(
    generator: &ShrubberyGenerator,
    group_idx: usize,
    star: &StarLeafShape,
    decoration: &LeafDecoration,
    voxels: &mut VoxelMap,
) {
    for (layer_index, branch) in generator
        .branches
        .iter()
        .filter(|b| b.leaf_group == Some(group_idx))
        .enumerate()
    {
        let mut rng = ChaCha8Rng::seed_from_u64(layer_index as u64 + generator.seed);
        let arm_length = star.arm_length.get(&mut rng);

        let iteration_percent = branch.iteration as f32 / branch.iteration_total as f32;

        emit_star_arms(
            &ArmShapeParams {
                pos: branch.pos,
                dir: branch.dir,
                layer_index: layer_index as u32,
                arm_length,
                arm_width: star.arm_width,
                branch_sharpness: star.branch_sharpness,
                thickness: star.thickness,
                droop: star.droop,
                tip_lift: star.tip_lift,
                rotation_step: None,
            },
            decoration,
            voxels,
            &mut rng,
            iteration_percent,
        );
    }
}

fn process_shapes(shrubbery: &mut ShrubberyGenerator, voxels: &mut VoxelMap) {
    for (leaf_index, (leaf_shape, leaf_decoration_selector)) in
        shrubbery.leaf_groups.iter().enumerate()
    {
        let leaf_decoration = leaf_decoration_selector.get_leaf_decoration(&mut shrubbery.rng);
        let Some(leaf_decoration) = leaf_decoration else {
            log_error!("leaf decoration is None");
            continue;
        };

        match leaf_shape {
            Shape::Sphere { radius } => {
                process_sphere_leaves(shrubbery, voxels, leaf_index, leaf_decoration, radius);
            }
            Shape::ConiferWhorl(conifer_whorl_shape) => {
                voxelize_conifer_whorls(
                    shrubbery,
                    leaf_index,
                    conifer_whorl_shape,
                    leaf_decoration,
                    voxels,
                );
            }
            Shape::StarLeaf(star_leaf_shape) => {
                voxelize_star_leaves(
                    shrubbery,
                    leaf_index,
                    star_leaf_shape,
                    leaf_decoration,
                    voxels,
                );
            }
        }
    }
}

fn process_branches(shrubbery: &mut ShrubberyGenerator, voxels: &mut BranchMap) {
    for (branch_index, branch) in shrubbery.branches.iter().enumerate() {
        let Some(parent_index) = branch.parent_index else {
            continue;
        };
        let start_pos = shrubbery.branches[parent_index].pos;
        let end_pos = branch.pos;

        let min = start_pos.min(end_pos) - Vec3::splat(branch.thickness + 1.0);
        let max = start_pos.max(end_pos) + Vec3::splat(branch.thickness + 1.0);
        let (min, max) = (min.floor().as_ivec3(), max.ceil().as_ivec3());

        let Some(group_id) = branch.decoration_group else {
            panic!("branch has no decoration group");
        };
        let decoration_selector = shrubbery
            .branch_decorations
            .get(group_id)
            .expect("decoration group exists");

        let Some(decoration) = decoration_selector.get_leaf_decoration(&mut shrubbery.rng) else {
            panic!("decoration selector resolved to None");
        };

        for x in min.x..=max.x {
            for y in min.y..=max.y {
                for z in min.z..=max.z {
                    let pos = ivec3(x, y, z);
                    let sample = pos.as_vec3();
                    let dist = point_segment_distance_squared(sample, start_pos, end_pos);
                    if dist >= (branch.thickness + EPSILON).powi(2) {
                        continue;
                    }
                    let mut branch_rng = rand_chacha::ChaCha8Rng::seed_from_u64(
                        branch_index as u64 + shrubbery.seed,
                    );

                    let iteration_percent =
                        (branch.iteration as f32 / branch.iteration_total as f32).clamp(0.0, 1.0);
                    let voxel_id = decoration.get_voxel_id(
                        &mut branch_rng,
                        sample,
                        min.as_vec3(),
                        max.as_vec3(),
                        iteration_percent,
                    );

                    voxels
                        .entry(pos)
                        .and_modify(|(best, id)| {
                            if dist < *best {
                                *best = dist;
                                *id = voxel_id;
                            }
                        })
                        .or_insert((dist, voxel_id));
                }
            }
        }
    }
}

fn point_segment_distance_squared(point: Vec3, start: Vec3, end: Vec3) -> f32 {
    let seg = end - start;
    let seg_len_sq = seg.length_squared();

    if seg_len_sq < EPSILON {
        return point.distance_squared(start);
    }

    let t = ((point - start).dot(seg) / seg_len_sq).clamp(0.0, 1.0);
    let closest = start + seg * t;

    point.distance_squared(closest)
}

fn conifer_taper_t(taper: &ConiferTaper, pos_y: f32, iteration: u32) -> f32 {
    match taper {
        ConiferTaper::Height { min_y, max_y } => {
            let range = (max_y - min_y).max(0.001);
            1.0 - ((pos_y - min_y) / range).clamp(0.0, 1.0)
        }
        ConiferTaper::Generation { max_generation } => {
            let max_gen = (*max_generation).max(1) as f32;
            1.0 - (iteration as f32 / max_gen).clamp(0.0, 1.0)
        }
        ConiferTaper::None => 1.0,
    }
}

fn voxelize_conifer_whorls(
    generator: &ShrubberyGenerator,
    group_idx: usize,
    whorl: &ConiferWhorlShape,
    decoration: &LeafDecoration,
    voxels: &mut VoxelMap,
) {
    struct WhorlInfo {
        layer_index: u32,
        pos: Vec3,
        dir: Vec3,
        taper_t: f32,
        iteration_percent: f32,
    }

    let mut whorl_infos: Vec<WhorlInfo> = Vec::new();
    let mut layer_counter: u32 = 0;
    let spacing = whorl.whorl_spacing.max(0.1);

    for branch in generator
        .branches
        .iter()
        .filter(|b| b.leaf_group == Some(group_idx))
    {
        let Some(parent_index) = branch.parent_index else {
            // No parent segment to interpolate — fall back to one whorl at
            // the branch endpoint (e.g. the root branch).
            let taper_t = conifer_taper_t(&whorl.taper, branch.pos.y, branch.iteration);

            let iteration_percent = branch.iteration as f32 / branch.iteration_total as f32;
            whorl_infos.push(WhorlInfo {
                layer_index: layer_counter,
                pos: branch.pos,
                dir: branch.dir,
                taper_t,
                iteration_percent,
            });
            layer_counter += 1;
            continue;
        };

        let parent = &generator.branches[parent_index];
        let seg = branch.pos - parent.pos; // segment vector — carries any lean/angle
        let seg_len = seg.length();
        let steps = (seg_len / spacing).ceil().max(1.0) as u32;

        let taper_t_start = conifer_taper_t(&whorl.taper, parent.pos.y, parent.iteration);
        let taper_t_end = conifer_taper_t(&whorl.taper, branch.pos.y, branch.iteration);

        // t in (0,1], so each sub-layer's root center walks up the leaning
        // segment exactly, and the final step lands on b.pos (no duplicate
        // whorl at the shared joint with the next segment).
        let iteration_percent = branch.iteration as f32 / branch.iteration_total as f32;
        for s in 0..steps {
            let t = (s + 1) as f32 / steps as f32;
            let pos = parent.pos + seg * t; // interpolated root center, tracks lean
            let taper_t = taper_t_start + (taper_t_end - taper_t_start) * t;
            whorl_infos.push(WhorlInfo {
                layer_index: layer_counter,
                pos,
                dir: seg,
                taper_t,
                iteration_percent,
            });
            layer_counter += 1;
        }
    }

    for info in &whorl_infos {
        let mut seed_rng = ChaCha8Rng::seed_from_u64(info.layer_index as u64 + generator.seed);
        let length_jitter = if whorl.length_jitter_ratio > 0.0 {
            let max_j = whorl.max_branch_length * whorl.length_jitter_ratio;
            seed_rng.random_range(-max_j..=max_j)
        } else {
            0.0
        };
        let arm_length = ((whorl.max_branch_length * info.taper_t) + length_jitter).max(0.0);
        let arm_width = whorl.max_branch_width * info.taper_t;

        emit_star_arms(
            &ArmShapeParams {
                pos: info.pos,
                dir: info.dir,
                layer_index: info.layer_index,
                arm_length,
                arm_width,
                branch_sharpness: whorl.branch_sharpness,
                thickness: whorl.branch_thickness,
                droop: whorl.branch_droop,
                tip_lift: whorl.tip_lift * info.taper_t,
                rotation_step: Some(whorl.rotation_step),
            },
            decoration,
            voxels,
            &mut seed_rng,
            info.iteration_percent,
        );
    }
}

struct ArmShapeParams {
    pos: Vec3,
    dir: Vec3,
    layer_index: u32,
    arm_length: f32,
    arm_width: f32,
    branch_sharpness: f32,
    thickness: u32,
    droop: f32,
    tip_lift: f32,
    /// `Some(step)` rotates each layer by `layer_index * step`; `None` randomizes.
    rotation_step: Option<f32>,
}

fn emit_star_arms(
    params: &ArmShapeParams,
    decoration: &LeafDecoration,
    voxels: &mut VoxelMap,
    rng: &mut ChaCha8Rng,
    iteration_percent: f32,
) {
    if params.arm_length < 0.5 {
        return;
    }
    let branch_dir = params.dir.normalize_or(Vec3::Y);
    let forward = Vec3::new(branch_dir.z, 0.0, -branch_dir.x).normalize_or(Vec3::X);
    let right = forward.cross(Vec3::Y).normalize_or(Vec3::Z);
    let angle = match params.rotation_step {
        Some(rotation_step) => params.layer_index as f32 * rotation_step,
        None => rng.random(),
    };
    let (sin_a, cos_a) = (angle.sin(), angle.cos());
    let arm_a = forward * cos_a - right * sin_a;
    let arm_b = forward * sin_a + right * cos_a;

    let cross_a = Vec3::new(-arm_a.z, 0.0, arm_a.x).normalize_or(Vec3::Z);
    let cross_b = Vec3::new(-arm_b.z, 0.0, arm_b.x).normalize_or(Vec3::X);

    let arm_length = params.arm_length;
    let search_radius = arm_length.ceil() as i32;

    // Vertical extent of the arm arc: y(d) = -d*droop + (d/arm_length)^2 * tip_lift.
    let y_at_tip = -arm_length * params.droop + params.tip_lift;
    let mut y_min = y_at_tip.min(0.0);
    let mut y_max = y_at_tip.max(0.0);

    // The arc is quadratic in d, so an interior extremum exists when droop and
    // tip_lift pull in opposite directions.
    if params.tip_lift.abs() > EPSILON {
        let d_crit = params.droop * arm_length * arm_length / (2.0 * params.tip_lift);
        if d_crit > 0.0 && d_crit < arm_length {
            let t = d_crit / arm_length;
            let y_crit = -d_crit * params.droop + t * t * params.tip_lift;
            y_min = y_min.min(y_crit);
            y_max = y_max.max(y_crit);
        }
    }

    let thickness_extra = params.thickness.saturating_sub(1) as f32;
    let bounds_min = vec3(
        params.pos.x - arm_length,
        params.pos.y + y_min - thickness_extra,
        params.pos.z - arm_length,
    );
    let bounds_max = vec3(
        params.pos.x + arm_length,
        params.pos.y + y_max,
        params.pos.z + arm_length,
    );
    for r_x in -search_radius..=search_radius {
        for r_z in -search_radius..=search_radius {
            let fx = r_x as f32;
            let fz = r_z as f32;
            let offset_h = Vec3::new(fx, 0.0, fz);

            let proj_a_main = offset_h.dot(arm_a);
            let proj_a_cross = offset_h.dot(cross_a);
            let proj_b_main = offset_h.dot(arm_b);
            let proj_b_cross = offset_h.dot(cross_b);

            let progress_a = proj_a_main.abs() / arm_length;
            let allowed_w_a = params.arm_width * (1.0 - progress_a * params.branch_sharpness);
            let progress_b = proj_b_main.abs() / arm_length;
            let allowed_w_b = params.arm_width * (1.0 - progress_b * params.branch_sharpness);

            let in_arm_a =
                proj_a_main.abs() <= arm_length && proj_a_cross.abs() <= allowed_w_a.max(0.5);
            let in_arm_b =
                proj_b_main.abs() <= arm_length && proj_b_cross.abs() <= allowed_w_b.max(0.5);
            let is_center = r_x == 0 && r_z == 0;

            if !in_arm_a && !in_arm_b && !is_center {
                continue;
            }

            let dist_from_center = (fx * fx + fz * fz).sqrt();
            let normalized_dist = (dist_from_center / arm_length).clamp(0.0, 1.0);
            let droop_y = dist_from_center * params.droop;
            let lift_y = normalized_dist * normalized_dist * params.tip_lift;
            let y_offset = -droop_y + lift_y;

            for t in 0..params.thickness {
                let world_x = params.pos.x + fx;
                let world_y = params.pos.y + y_offset - t as f32;
                let world_z = params.pos.z + fz;
                let grid_pos = ivec3(
                    world_x.floor() as i32,
                    world_y.floor() as i32,
                    world_z.floor() as i32,
                );
                let sample_pos = vec3(world_x, world_y, world_z);
                let voxel_id = decoration.get_voxel_id(
                    rng,
                    sample_pos,
                    bounds_min,
                    bounds_max,
                    iteration_percent,
                );
                voxels.insert(grid_pos, voxel_id);
            }
        }
    }
}
