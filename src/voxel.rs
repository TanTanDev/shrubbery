use std::collections::HashMap;

#[cfg(feature = "bevy")]
use bevy::{ecs::resource::Resource, log::warn};
use glam::{IVec3, Vec3, ivec3, vec3};
use rand::{RngExt, SeedableRng, seq::IndexedRandom};

use rand_chacha::ChaCha8Rng;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    leaf_classifier::LeafClassifier,
    math_utils::percent_in_range,
    prelude::TreeGeneratorSpaceColonization,
    tree_space_colonization::{BarkDecorator, SpaceColonizationSettings},
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
    Sphere { r: f32 },
}

/// what method to use to classify leaves
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LeafSetting {
    // generate no leaves
    #[default]
    None,
    // color branches as leaves
    BranchIsLeaf(LeafClassifier),
    // spawn leaf shapes
    Shape {
        shape: LeafShape,
        decoration: LeafDecoration,
    },
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

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VoxelizeSettings {
    pub leaf_settings: LeafSetting,
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

fn leaf_padding(settings: &SpaceColonizationSettings) -> i32 {
    match &settings.voxelize_settings.leaf_settings {
        LeafSetting::Shape { shape, .. } => match shape {
            LeafShape::Sphere { r } => r.ceil() as i32,
        },
        _ => 0,
    }
}

/// construct voxel positions based upon tree
pub fn voxelize(
    generator: &mut TreeGeneratorSpaceColonization,
    settings: &SpaceColonizationSettings,
) -> Vec<(IVec3, VoxelId)> {
    let (mut min_bounds, mut max_bounds) = generator.get_bounds();
    let padding = leaf_padding(settings);
    min_bounds -= IVec3::splat(padding);
    max_bounds += IVec3::splat(padding);

    let mut voxels = Vec::with_capacity(128);
    for x in min_bounds.x..max_bounds.x {
        for y in min_bounds.y..max_bounds.y {
            for z in min_bounds.z..max_bounds.z {
                let pos = ivec3(x, y, z);
                process_voxel(pos, generator, &settings, &mut voxels);
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

    // leaf shape
    if let LeafSetting::Shape { shape, decoration } = &settings.voxelize_settings.leaf_settings {
        if generate_leaf(
            sample_pos,
            pos,
            shrubbery,
            voxels,
            shape,
            decoration,
            &LeafClassifier::LastBranch,
        ) {
            // no need to check for branch
            return;
        }
    }

    let (dist_to_branch, closest_branch_index) = shrubbery.distance_to_branch(sample_pos);
    let mut size = match &settings.voxelize_settings.branch_size_setting {
        BranchSizeSetting::Value { size: distance } => *distance,
        BranchSizeSetting::Generation { sizes: distances } => {
            let closest_branch = &shrubbery.branches[closest_branch_index];
            let index = closest_branch.generation.min(distances.len() as i32 - 1);
            *distances.get(index as usize).unwrap_or(&f32::MIN)
        }
    };
    if let Some(increaser) = &settings.voxelize_settings.branch_root_size_increaser {
        let h_m = 1.0 - (sample_pos.y / increaser.height.max(0.001)).min(1.0);
        size += h_m * increaser.additional_size;
    }
    if dist_to_branch < size + EPSILON {
        let closest_branch = &shrubbery.branches[closest_branch_index];
        let is_leaf = if let LeafSetting::BranchIsLeaf(classifier) =
            &settings.voxelize_settings.leaf_settings
        {
            closest_branch.is_leaf(classifier)
        } else {
            false
        };

        // let voxel_type = if is_leaf {
        //     VoxelType::Greenery
        // } else {
        //     VoxelType::Branch
        // };
        // todo proper handling
        let voxel_type = if is_leaf {
            VoxelId(0u32)
        } else {
            match &settings.bark_decorator {
                BarkDecorator::Single(voxel_mapping) => voxel_mapping.id.clone(),
            }
        };
        voxels.push((
            ivec3(
                sample_pos.x as i32,
                sample_pos.y as i32,
                sample_pos.z as i32,
            ),
            voxel_type,
        ));
    }
}

fn generate_leaf(
    pos: Vec3,
    grid_pos: IVec3,
    mut shrubbery: &mut TreeGeneratorSpaceColonization,
    voxels: &mut Vec<(IVec3, VoxelId)>,
    leaf_shape: &LeafShape,
    leaf_decoration: &LeafDecoration,
    leaf_classifier: &LeafClassifier,
) -> bool {
    let TreeGeneratorSpaceColonization { branches, rng, .. } = &mut shrubbery;
    for leaf_branch in branches
        .iter()
        .filter(|branch| branch.is_leaf(leaf_classifier))
    {
        // make leaf
        let leaf_pos = leaf_branch.pos;
        match leaf_shape {
            LeafShape::Sphere { r } => {
                if is_inside_sphere(pos, leaf_pos, *r) {
                    let voxel_id = match leaf_decoration {
                        LeafDecoration::Single(voxel_mapping) => voxel_mapping.id,
                        LeafDecoration::Randomized(items) => items
                            .choose_weighted(rng, |i| i.weight)
                            .map(|v| v.voxel_mapping.id.clone())
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
                            let bounds = (leaf_v - *r, leaf_v + *r);
                            let mut percent = percent_in_range(pos_v, bounds.0, bounds.1);

                            // apply modulaion
                            if let Some(modulation) = &gradient_settings.modulation {
                                percent += match modulation {
                                    LeafGradientModulation::Random { percent_offset } => {
                                        rng.random_range(-*percent_offset..*percent_offset)
                                        // todo
                                    }
                                    LeafGradientModulation::Wave {
                                        frequency,
                                        amplitude,
                                    } => {
                                        let coord = (pos.x + pos.z) * *frequency;
                                        let wave = coord.sin();
                                        // let wave = wave * 0.5 + 0.5;
                                        wave * amplitude // let x = (pos.x as i32) % modulus;
                                        // let z = (pos.z as i32) % modulus;
                                        // let offset = (x + z) % modulus;

                                        // let percent = offset as f32 / *modulus as f32;

                                        // percent * percent_offset
                                    }
                                }
                            }
                            percent = percent.clamp(0.0, 1.0);

                            let mut selected_voxel_id = VoxelId::default();
                            // find gradient
                            for step in gradient_settings.steps.iter() {
                                if percent > step.percent {
                                    continue;
                                }
                                // found
                                selected_voxel_id = step.voxel_mapping.id;
                                break;
                            }
                            selected_voxel_id
                        }
                    };
                    // todo use proper id
                    voxels.push((grid_pos, voxel_id));
                    // quick exit, we found greenery at this position
                    return true;
                }
            }
        }
    }
    false
}

fn is_inside_sphere(pos: Vec3, sphere_pos: Vec3, radius: f32) -> bool {
    pos.distance(sphere_pos) <= radius + EPSILON
}
