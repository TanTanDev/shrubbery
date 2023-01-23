use glam::{ivec3, vec3, IVec3, Vec3};
use rand::{thread_rng, Rng};

use crate::{leaf_classifier::LeafClassifier, shrubbery::Shrubbery};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum VoxelType {
    Air,
    Branch,
    Greenery,
}

pub enum LeafShape {
    Sphere { r: f32 },
}

pub enum LeafSetting {
    // generate no leaves
    None,
    // color branches as leaves
    BranchIsLeaf(LeafClassifier),
    // spawn leaf shapes
    Shape(LeafShape),
}

pub enum BranchSizeSetting {
    Value { distance: f32 },
    Generation { distances: Vec<f32> },
}

pub struct BranchRootSizeIncreaser {
    pub height: f32,
    // how much to maximally add to the root size
    pub additional_size: f32,
}

pub struct VoxelizeSettings {
    // pub leaf_shape: Option<LeafShape>,
    pub leaf_settings: LeafSetting,
    // pub leaf_classifier: LeafClassifier,
    pub branch_size_setting: BranchSizeSetting,
    pub branch_root_size_increaser: Option<BranchRootSizeIncreaser>,
}

pub fn drop_leaves(voxels: &mut Vec<(IVec3, VoxelType)>, procentage: f32) {
    let mut branch_indices = voxels
        .iter()
        .enumerate()
        .filter(|(_i, (_p, v))| v == &VoxelType::Greenery)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();

    let to_drop = (branch_indices.len() as f32 * procentage) as usize;
    let mut to = Vec::with_capacity(to_drop);
    for _ in 0..to_drop {
        let branch_indicies_i = thread_rng().gen_range(0..branch_indices.len());
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

///
pub fn voxelize(shrubbery: &Shrubbery, settings: &VoxelizeSettings) -> Vec<(IVec3, VoxelType)> {
    let (min_bounds, max_bounds) = shrubbery.get_bounds();
    let mut size = max_bounds - min_bounds;
    // I use half the size to go -half_size -> half_size
    size.x = (size.x as f32 * 0.5).ceil() as i32;
    size.z = (size.z as f32 * 0.5).ceil() as i32;

    // apply extra padding in size from leaves
    if let LeafSetting::Shape(leaf_shape) = &settings.leaf_settings {
        let padding: i32 = match leaf_shape {
            LeafShape::Sphere { r } => r.ceil() as i32,
        };
        // todo: include root size into padding
        size += IVec3::splat(padding);
    }

    let mut voxels = Vec::with_capacity(128);
    for x in -size.x..size.x {
        for y in 0..size.y {
            for z in -size.z..size.z {
                let pos = ivec3(x, y, z);
                process_voxel(pos, shrubbery, &settings, &mut voxels);
            }
        }
    }

    voxels
}

fn process_voxel(
    pos: IVec3,
    shrubbery: &Shrubbery,
    settings: &VoxelizeSettings,
    voxels: &mut Vec<(IVec3, VoxelType)>,
) {
    let sample_pos = vec3(pos.x as f32, pos.y as f32, pos.z as f32);

    // leaf shape
    if let LeafSetting::Shape(leaf_shape) = &settings.leaf_settings {
        if generate_leaf(
            sample_pos,
            &shrubbery,
            voxels,
            leaf_shape,
            &LeafClassifier::LastBranch,
        ) {
            // no need to check for branch
            return;
        }
    }

    let (dist_to_branch, closest_branch_index) = shrubbery.distance_to_branch(sample_pos);
    let mut size = match &settings.branch_size_setting {
        BranchSizeSetting::Value { distance } => *distance,
        BranchSizeSetting::Generation { distances } => {
            let closest_branch = &shrubbery.branches[closest_branch_index];
            let index = closest_branch.generation.min(distances.len() as i32 - 1);
            *distances.get(index as usize).unwrap_or(&f32::MIN)
        }
    };
    if let Some(increaser) = &settings.branch_root_size_increaser {
        let h_m = 1.0 - (sample_pos.y / increaser.height).min(1.0);
        size += h_m * increaser.additional_size;
    }
    if dist_to_branch < size {
        let closest_branch = &shrubbery.branches[closest_branch_index];
        let is_leaf = if let LeafSetting::BranchIsLeaf(classifier) = &settings.leaf_settings {
            closest_branch.is_leaf(classifier)
        } else {
            false
        };
        let voxel_type = if is_leaf {
            VoxelType::Greenery
        } else {
            VoxelType::Branch
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
    shrubbery: &Shrubbery,
    voxels: &mut Vec<(IVec3, VoxelType)>,
    leaf_shape: &LeafShape,
    leaf_classifier: &LeafClassifier,
) -> bool {
    for leaf_branch in shrubbery
        .branches
        .iter()
        .filter(|branch| branch.is_leaf(leaf_classifier))
    {
        // make leaf
        let leaf_pos = leaf_branch.pos;
        match leaf_shape {
            LeafShape::Sphere { r } => {
                if is_inside_sphere(pos, leaf_pos, *r) {
                    voxels.push((
                        ivec3(
                            pos.x.ceil() as i32,
                            pos.y.ceil() as i32,
                            pos.z.ceil() as i32,
                        ),
                        VoxelType::Greenery,
                    ));
                    // quick exit, we found greenery at this position
                    return true;
                }
            }
        }
    }
    false
}

fn is_inside_sphere(pos: Vec3, sphere_pos: Vec3, radius: f32) -> bool {
    pos.distance(sphere_pos) <= radius
}
