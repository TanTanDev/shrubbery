use std::collections::HashMap;

#[cfg(feature = "bevy")]
use bevy::{ecs::resource::Resource, log::warn};
use glam::{IVec3, Vec3, ivec3, vec3};
use rand::{RngExt, SeedableRng, seq::IndexedRandom};

use rand_chacha::ChaCha8Rng;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    math_utils::percent_in_range,
    prelude::TreeGeneratorSpaceColonization,
    tree_space_colonization::{BarkDecorator, SpaceColonizationSettings, ValueOrRangeF32},
};
const EPSILON: f32 = 0.0001;

/// the raw voxel representation, that will be sent back to library-implementor
#[derive(Eq, PartialEq, Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VoxelId(pub u32);

/// voxel name mapped to VoxelId
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct VoxelMapping {
    /// startup, mapping value to find voxel type
    #[serde(default)]
    pub name: String,
    /// runtime friendly voxel id
    #[serde(default)]
    pub id: VoxelId,
}

impl VoxelMapping {
    pub fn resolve(&mut self, definitions: &VoxelDefinitions) {
        self.id = definitions.id_from_name(self.name.as_str());
    }
}

#[cfg_attr(feature = "bevy", derive(Resource))]
pub struct VoxelDefinitions(pub HashMap<String, VoxelId>);

impl VoxelDefinitions {
    pub fn get_id_from_name(&self, name: &str) -> Option<VoxelId> {
        self.0.get(name).copied()
    }
    pub fn id_from_name(&self, name: &str) -> VoxelId {
        self.0.get(name).copied().unwrap_or_else(|| {
            #[cfg(feature = "bevy")]
            warn!("no named voxel: '{}' in VoxelDefinitions", name);
            VoxelId(0u32)
        })
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RandomizedVoxelEntry {
    weight: i32,
    voxel_mapping: VoxelMapping,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Axis {
    X,
    Y,
    Z,
}

/// adds offset unto gradient coloring, to add variation
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LeafGradientModulation {
    Random { percent_offset: f32 },
    Wave { frequency: f32, amplitude: f32 },
}

/// describes how to color voxels
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LeafGradientSettings {
    pub axis: Axis,
    pub modulation: Option<LeafGradientModulation>,
    pub steps: Vec<LeafGradientEntry>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LeafGradientEntry {
    pub percent: f32,
    pub voxel_mapping: VoxelMapping,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LeafDecoration {
    Single(VoxelMapping),
    Randomized(Vec<RandomizedVoxelEntry>),
    Gradient(LeafGradientSettings),
}

impl LeafDecoration {
    pub fn resolve(&mut self, voxel_definitions: &VoxelDefinitions) {
        match self {
            LeafDecoration::Single(voxel_mapping) => voxel_mapping.resolve(voxel_definitions),
            LeafDecoration::Randomized(items) => {
                items
                    .iter_mut()
                    .for_each(|entry| entry.voxel_mapping.resolve(voxel_definitions));
            }
            LeafDecoration::Gradient(settings) => {
                settings
                    .steps
                    .iter_mut()
                    .for_each(|entry| entry.voxel_mapping.resolve(voxel_definitions));
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LeafShape {
    Sphere {
        radius: ValueOrRangeF32,
    },
    /// Conifer-style whorl emitted at each leaf branch.
    ///
    /// Voxels are emitted directly (generative), bypassing the per-cell
    /// bounding-box scan, so this is much cheaper than `Sphere` for sparse
    /// canopy geometry.
    ConiferWhorl(ConiferWhorlShape),
}

/// Controls which direction the two whorl arms face outward from the branch.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ArmFacing {
    /// Arms are horizontal (perpendicular to world Y). Default for conifers.
    Horizontal,
    /// Arms shoot outward in a random direction for each branch, sampled
    /// within a cone around WorldUp.  Used for palm fronds fanning in all
    /// directions from a single crown tip.
    Random {
        /// Maximum angle from horizontal in degrees (0 = flat, 90 = full sphere).
        max_pitch_degrees: f32,
    },
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
    /// Which direction the arms face outward from the branch.
    /// `Horizontal` for conifers, `Random` for palm fronds.
    #[serde(default = "default_arm_facing")]
    pub arm_facing: ArmFacing,
}

fn default_arm_facing() -> ArmFacing {
    ArmFacing::Horizontal
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BranchSizeSetting {
    Value { size: f32 },
    Generation { sizes: Vec<f32> },
}

impl Default for BranchSizeSetting {
    fn default() -> Self {
        Self::Value { size: 1.0 }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BranchRootSizeIncreaser {
    /// height where to not add additional size
    pub height: f32,
    /// how much to maximally add to the root size
    pub additional_size: f32,
}

/// Bark and branch geometry settings — leaf settings are now in build steps.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VoxelizeSettings {
    pub branch_size_setting: BranchSizeSetting,
    pub branch_root_size_increaser: Option<BranchRootSizeIncreaser>,
}

pub fn drop_id(voxels: &mut Vec<(IVec3, VoxelId)>, voxel_id: VoxelId, procentage: f32, seed: u64) {
    let mut branch_indices = voxels
        .iter()
        .enumerate()
        .filter(|(_i, (_p, v))| v == &voxel_id)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();

    let to_drop = (branch_indices.len() as f32 * procentage) as usize;
    let mut to = Vec::with_capacity(to_drop);
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
    for _ in 0..to_drop {
        let branch_indicies_i = rng.random_range(0..branch_indices.len());
        let index = branch_indices[branch_indicies_i];
        branch_indices.remove(branch_indicies_i);
        to.push(index);
    }
    to.sort();
    to.reverse();
    for i in to.into_iter() {
        voxels.remove(i);
    }
}

/// Largest leaf radius across all leaf groups, for bounding-box padding.
fn leaf_padding(generator: &TreeGeneratorSpaceColonization) -> i32 {
    generator
        .leaf_groups
        .iter()
        .map(|(shape, _)| match shape {
            LeafShape::Sphere { radius: r } => r.max().ceil() as i32,
            LeafShape::ConiferWhorl(w) => w.max_branch_length.ceil() as i32,
        })
        .max()
        .unwrap_or(0)
}

/// Construct voxel positions for the tree.
///
/// Leaf shapes are driven by per-branch `leaf_group` indices set during build
/// steps — there is no longer a single global leaf setting.  Multiple leaf
/// groups can coexist (e.g. conifer whorls on lower branches, spheres on the
/// canopy).  ConiferWhorl groups are emitted generatively; Sphere groups are
/// tested per-cell in the bounding-box scan.
pub fn voxelize(
    generator: &mut TreeGeneratorSpaceColonization,
    settings: &SpaceColonizationSettings,
) -> Vec<(IVec3, VoxelId)> {
    let (mut min_bounds, mut max_bounds) = generator.get_bounds();
    let padding = leaf_padding(generator);
    min_bounds -= IVec3::splat(padding);
    max_bounds += IVec3::splat(padding);

    let mut voxels = Vec::with_capacity(128);

    // Pass 1: emit all ConiferWhorl groups directly (generative, no bbox scan).
    let whorl_group_indices: Vec<usize> = generator
        .leaf_groups
        .iter()
        .enumerate()
        .filter(|(_, (shape, _))| matches!(shape, LeafShape::ConiferWhorl(_)))
        .map(|(i, _)| i)
        .collect();

    for group_idx in whorl_group_indices {
        // Clone shape/decoration to avoid holding generator borrow.
        let (shape, decoration) = generator.leaf_groups[group_idx].clone();
        let LeafShape::ConiferWhorl(whorl) = &shape else {
            continue;
        };
        voxelize_conifer_whorls(generator, group_idx, whorl, &decoration, &mut voxels);
    }

    // Pass 2: bark and Sphere leaves via bounding-box scan.
    for x in min_bounds.x..max_bounds.x {
        for y in min_bounds.y..max_bounds.y {
            for z in min_bounds.z..max_bounds.z {
                let pos = ivec3(x, y, z);
                process_voxel(pos, generator, settings, &mut voxels);
            }
        }
    }

    voxels
}

fn process_voxel(
    pos: IVec3,
    shrubbery: &mut TreeGeneratorSpaceColonization,
    settings: &SpaceColonizationSettings,
    voxels: &mut Vec<(IVec3, VoxelId)>,
) {
    let sample_pos = vec3(pos.x as f32 + 0.5, pos.y as f32 + 0.5, pos.z as f32 + 0.5);

    // Test Sphere leaf groups: for each group that is Sphere-shaped, test this
    // cell against all branches that reference that group.
    let sphere_group_indices: Vec<usize> = shrubbery
        .leaf_groups
        .iter()
        .enumerate()
        .filter(|(_, (shape, _))| matches!(shape, LeafShape::Sphere { .. }))
        .map(|(i, _)| i)
        .collect();

    for group_idx in sphere_group_indices {
        if generate_sphere_leaf(sample_pos, pos, shrubbery, group_idx, &mut *voxels) {
            return; // cell claimed by a leaf — don't also render bark here
        }
    }

    // Bark rendering.
    let (dist_to_branch, closest_branch_index) = shrubbery.distance_to_branch(sample_pos);
    let mut size = match &settings.branch_size_setting {
        BranchSizeSetting::Value { size: distance } => *distance,
        BranchSizeSetting::Generation { sizes: distances } => {
            let closest_branch = &shrubbery.branches[closest_branch_index];
            let index = closest_branch.generation.min(distances.len() as i32 - 1);
            *distances.get(index as usize).unwrap_or(&f32::MIN)
        }
    };
    if let Some(increaser) = &settings.branch_root_size_increaser {
        let h_m = 1.0 - (sample_pos.y / increaser.height.max(0.001)).min(1.0);
        size += h_m * increaser.additional_size;
    }
    if dist_to_branch < size + EPSILON {
        let bark_id = match &settings.bark_decorator {
            BarkDecorator::Single(voxel_mapping) => voxel_mapping.id,
        };
        voxels.push((
            ivec3(
                sample_pos.x as i32,
                sample_pos.y as i32,
                sample_pos.z as i32,
            ),
            bark_id,
        ));
    }
}

/// Emit conifer-whorl voxels directly from each branch assigned to this leaf group.
fn voxelize_conifer_whorls(
    generator: &mut TreeGeneratorSpaceColonization,
    group_idx: usize,
    whorl: &ConiferWhorlShape,
    decoration: &LeafDecoration,
    voxels: &mut Vec<(IVec3, VoxelId)>,
) {
    struct WhorlInfo {
        branch_index: usize,
        pos: Vec3,
        dir: Vec3,
        taper_t: f32,
    }

    let whorl_infos: Vec<WhorlInfo> = generator
        .branches
        .iter()
        .enumerate()
        .filter(|(_, b)| b.leaf_group == Some(group_idx))
        .map(|(i, b)| {
            let taper_t = match &whorl.taper {
                ConiferTaper::Height { min_y, max_y } => {
                    let range = (max_y - min_y).max(0.001);
                    1.0 - ((b.pos.y - min_y) / range).clamp(0.0, 1.0)
                }
                ConiferTaper::Generation { max_generation } => {
                    let max_gen = (*max_generation).max(1) as f32;
                    1.0 - (b.generation as f32 / max_gen).clamp(0.0, 1.0)
                }
                ConiferTaper::None => 1.0,
            };
            WhorlInfo {
                branch_index: i,
                pos: b.pos,
                dir: b.dir,
                taper_t,
            }
        })
        .collect();

    for info in &whorl_infos {
        let mut branch_rng =
            rand_chacha::ChaCha8Rng::seed_from_u64(info.branch_index as u64 ^ 0xdeadbeef_cafebabe);

        let length_jitter = if whorl.length_jitter_ratio > 0.0 {
            let max_j = whorl.max_branch_length * whorl.length_jitter_ratio;
            branch_rng.random_range(-max_j..max_j)
        } else {
            0.0
        };

        let arm_length = ((whorl.max_branch_length * info.taper_t) + length_jitter).max(0.0);
        let arm_width = whorl.max_branch_width * info.taper_t;

        if arm_length < 0.5 {
            continue;
        }

        // Build the primary arm axis depending on ArmFacing.
        let (arm_a, arm_b) = match &whorl.arm_facing {
            ArmFacing::Horizontal => {
                // Arms perpendicular to the branch direction in the horizontal plane.
                let branch_dir = info.dir.normalize_or(Vec3::Y);
                let forward = Vec3::new(branch_dir.z, 0.0, -branch_dir.x).normalize_or(Vec3::X);
                let right = forward.cross(Vec3::Y).normalize_or(Vec3::Z);
                let angle = info.branch_index as f32 * whorl.rotation_step;
                let (sin_a, cos_a) = (angle.sin(), angle.cos());
                let arm_a = forward * cos_a - right * sin_a;
                let arm_b = forward * sin_a + right * cos_a;
                (arm_a, arm_b)
            }
            ArmFacing::Random { max_pitch_degrees } => {
                // Each branch gets a unique random direction for a palm-frond fan.
                // We generate two independent random arms spread ~90° apart.
                let max_pitch = max_pitch_degrees.to_radians();
                let yaw_a = branch_rng.random_range(0.0..std::f32::consts::TAU);
                let pitch_a = branch_rng.random_range(0.0..max_pitch);
                let yaw_b =
                    yaw_a + std::f32::consts::FRAC_PI_2 + branch_rng.random_range(-0.3..0.3);
                let pitch_b = branch_rng.random_range(0.0..max_pitch);
                let arm_a = Vec3::new(
                    yaw_a.cos() * pitch_a.cos(),
                    -pitch_a.sin(),
                    yaw_a.sin() * pitch_a.cos(),
                );
                let arm_b = Vec3::new(
                    yaw_b.cos() * pitch_b.cos(),
                    -pitch_b.sin(),
                    yaw_b.sin() * pitch_b.cos(),
                );
                (arm_a.normalize_or(Vec3::X), arm_b.normalize_or(Vec3::Z))
            }
        };

        // Cross-arm axes (perpendicular to each arm in the horizontal plane).
        let cross_a = Vec3::new(-arm_a.z, 0.0, arm_a.x).normalize_or(Vec3::Z);
        let cross_b = Vec3::new(-arm_b.z, 0.0, arm_b.x).normalize_or(Vec3::X);

        let search_radius = arm_length.ceil() as i32;

        for r_x in -search_radius..=search_radius {
            for r_z in -search_radius..=search_radius {
                let fx = r_x as f32;
                let fz = r_z as f32;
                let offset_h = Vec3::new(fx, 0.0, fz);

                let proj_a_main = offset_h.dot(arm_a);
                let proj_a_cross = offset_h.dot(cross_a);
                let proj_b_main = offset_h.dot(arm_b);
                let proj_b_cross = offset_h.dot(cross_b);

                let progress_a = if arm_length > 0.0 {
                    proj_a_main.abs() / arm_length
                } else {
                    1.0
                };
                let allowed_w_a = arm_width * (1.0 - progress_a * whorl.branch_sharpness);

                let progress_b = if arm_length > 0.0 {
                    proj_b_main.abs() / arm_length
                } else {
                    1.0
                };
                let allowed_w_b = arm_width * (1.0 - progress_b * whorl.branch_sharpness);

                let in_arm_a = arm_length > 0.0
                    && proj_a_main.abs() <= arm_length
                    && proj_a_cross.abs() <= allowed_w_a.max(0.5); // min 0.5 so centre voxels always fill

                let in_arm_b = arm_length > 0.0
                    && proj_b_main.abs() <= arm_length
                    && proj_b_cross.abs() <= allowed_w_b.max(0.5);

                let is_center = r_x == 0 && r_z == 0;

                if !in_arm_a && !in_arm_b && !is_center {
                    continue;
                }

                let dist_from_center = (fx * fx + fz * fz).sqrt();
                let normalized_dist = if arm_length > 0.0 {
                    (dist_from_center / arm_length).clamp(0.0, 1.0)
                } else {
                    1.0
                };

                // droop: positive branch_droop droops tips down; negative lifts them.
                // tip_lift adds an additional upward parabolic arc at the tip.
                let droop_y = dist_from_center * whorl.branch_droop;
                let lift_y = normalized_dist * normalized_dist * whorl.tip_lift;
                let y_offset = -droop_y + lift_y;

                for t in 0..whorl.branch_thickness {
                    let world_x = info.pos.x + fx;
                    let world_y = info.pos.y + y_offset - t as f32;
                    let world_z = info.pos.z + fz;

                    let grid_pos = ivec3(
                        world_x.floor() as i32,
                        world_y.floor() as i32,
                        world_z.floor() as i32,
                    );
                    let sample_pos = vec3(world_x, world_y, world_z);

                    let voxel_id = resolve_whorl_decoration(
                        decoration,
                        &mut branch_rng,
                        normalized_dist,
                        sample_pos,
                        info.pos,
                        arm_length,
                    );

                    voxels.push((grid_pos, voxel_id));
                }
            }
        }
    }
}

/// Resolve the decoration voxel id for a single whorl voxel.
///
/// `normalized_dist` is 0.0 at the branch centre, 1.0 at the arm tip.
/// `sample_pos` / `branch_pos` / `arm_length` are forwarded to Gradient so it
/// can re-use the existing axis/modulation logic.
fn resolve_whorl_decoration(
    decoration: &LeafDecoration,
    rng: &mut ChaCha8Rng,
    normalized_dist: f32,
    sample_pos: Vec3,
    branch_pos: Vec3,
    arm_length: f32,
) -> VoxelId {
    match decoration {
        LeafDecoration::Single(m) => m.id,
        LeafDecoration::Randomized(items) => items
            .choose_weighted(rng, |i| i.weight)
            .map(|v| v.voxel_mapping.id)
            .unwrap_or_default(),
        LeafDecoration::Gradient(g) => {
            // Re-use the same axis/modulation path as the Sphere gradient so
            // that existing RON configs work unchanged.
            let leaf_v = match g.axis {
                Axis::X => branch_pos.x,
                Axis::Y => branch_pos.y,
                Axis::Z => branch_pos.z,
            };
            let pos_v = match g.axis {
                Axis::X => sample_pos.x,
                Axis::Y => sample_pos.y,
                Axis::Z => sample_pos.z,
            };
            let bounds = (leaf_v - arm_length, leaf_v + arm_length);
            let mut percent = percent_in_range(pos_v, bounds.0, bounds.1);

            if let Some(modulation) = &g.modulation {
                percent += match modulation {
                    LeafGradientModulation::Random { percent_offset } => {
                        rng.random_range(-*percent_offset..*percent_offset)
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
            for step in g.steps.iter() {
                if percent > step.percent {
                    continue;
                }
                selected = step.voxel_mapping.id;
                break;
            }
            selected
        }
    }
}

/// Test a single voxel cell against all branches in the given sphere leaf group.
/// Returns true and emits the voxel if any branch's sphere covers this cell.
fn generate_sphere_leaf(
    pos: Vec3,
    grid_pos: IVec3,
    mut shrubbery: &mut TreeGeneratorSpaceColonization,
    group_idx: usize,
    // r: f32,
    // decoration: &LeafDecoration,
    voxels: &mut Vec<(IVec3, VoxelId)>,
) -> bool {
    let TreeGeneratorSpaceColonization {
        branches,
        rng,
        leaf_groups,
        ..
    } = &mut shrubbery;

    let (shape, decoration) = &leaf_groups[group_idx];
    let LeafShape::Sphere { radius } = &shape else {
        return false;
    };

    for (branch_index, branch) in branches
        .iter()
        .filter(|b| b.leaf_group == Some(group_idx))
        .enumerate()
    {
        let mut branch_rng = rand_chacha::ChaCha8Rng::seed_from_u64(branch_index as u64);
        let r = radius.get(&mut branch_rng);

        let leaf_pos = branch.pos;
        if !is_inside_sphere(pos, leaf_pos, r) {
            continue;
        }
        let voxel_id = match decoration {
            LeafDecoration::Single(voxel_mapping) => voxel_mapping.id,
            LeafDecoration::Randomized(items) => items
                .choose_weighted(rng, |i| i.weight)
                .map(|v| v.voxel_mapping.id)
                .unwrap_or_default(),
            LeafDecoration::Gradient(gradient_settings) => {
                let leaf_v = match gradient_settings.axis {
                    Axis::X => leaf_pos.x,
                    Axis::Y => leaf_pos.y,
                    Axis::Z => leaf_pos.z,
                };
                let pos_v = match gradient_settings.axis {
                    Axis::X => pos.x,
                    Axis::Y => pos.y,
                    Axis::Z => pos.z,
                };
                let bounds = (leaf_v - r, leaf_v + r);
                let mut percent = percent_in_range(pos_v, bounds.0, bounds.1);
                if let Some(modulation) = &gradient_settings.modulation {
                    percent += match modulation {
                        LeafGradientModulation::Random { percent_offset } => {
                            rng.random_range(-*percent_offset..*percent_offset)
                        }
                        LeafGradientModulation::Wave {
                            frequency,
                            amplitude,
                        } => {
                            let coord = (pos.x + pos.z) * *frequency;
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
                    selected = step.voxel_mapping.id;
                    break;
                }
                selected
            }
        };
        voxels.push((grid_pos, voxel_id));
        return true;
    }
    false
}

fn is_inside_sphere(pos: Vec3, sphere_pos: Vec3, radius: f32) -> bool {
    pos.distance(sphere_pos) <= radius + EPSILON
}
