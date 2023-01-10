use glam::{ivec3, vec3, IVec3, Vec3};

use crate::shrubbery::Shrubbery;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum VoxelType {
    Air,
    Branch,
    Greenery,
}

pub enum LeafGenerator {
    Sphere { r: f32 },
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
    pub leaf_generator: Option<LeafGenerator>,
    pub branch_size_setting: BranchSizeSetting,
    pub branch_root_size_increaser: Option<BranchRootSizeIncreaser>,
}

///
pub fn voxelize(shrubbery: &Shrubbery, settings: VoxelizeSettings) -> Vec<(IVec3, VoxelType)> {
    let (min_bounds, max_bounds) = shrubbery.get_bounds();
    let size = max_bounds - min_bounds;
    let mut uniform = size.x.max(size.y.max(size.z));
    // apply extra padding
    if let Some(leaf_generator) = &settings.leaf_generator {
        let padding: i32 = match leaf_generator {
            LeafGenerator::Sphere { r } => r.ceil() as i32,
        };
        println!("padding: {:?}", padding);
        uniform += padding;
    }

    let capacity = uniform.pow(3) as usize;
    let mut voxels = Vec::with_capacity(128);
    println!(
        "capacity: {:?}, uniform: {:?}, min: {:?} max: {:?}",
        capacity, uniform, min_bounds, max_bounds
    );
    let mut cou = 0;
    let neg_offset = uniform as f32 * -0.5f32;
    for i in 0..capacity {
        let i = i as i32;
        let x = i / (uniform * uniform);
        let y = (i / uniform) % uniform;
        let z = i % uniform;

        let sample_pos = vec3(x as f32 + neg_offset, y as f32, z as f32 + neg_offset);
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
            voxels.push((
                ivec3(
                    sample_pos.x as i32,
                    sample_pos.y as i32,
                    sample_pos.z as i32,
                ),
                VoxelType::Branch,
            ));
            cou += 1;
        }
        if let Some(leaf_settings) = &settings.leaf_generator {
            generate_leaf(sample_pos, &shrubbery, &mut voxels, leaf_settings);
        }
    }
    println!("cou: {:?}", cou);
    voxels
}

fn generate_leaf(
    pos: Vec3,
    shrubbery: &Shrubbery,
    voxels: &mut Vec<(IVec3, VoxelType)>,
    generator: &LeafGenerator,
) {
    for leaf_branch in shrubbery.branches.iter().filter(|branch| branch.is_leaf()) {
        // make leaf
        let leaf_pos = leaf_branch.pos;
        match generator {
            LeafGenerator::Sphere { r } => {
                if is_inside_sphere(pos, leaf_pos, *r) {
                    voxels.push((
                        ivec3(
                            pos.x.ceil() as i32,
                            pos.y.ceil() as i32,
                            pos.z.ceil() as i32,
                        ),
                        VoxelType::Greenery,
                    ));
                }
            }
        }
    }
}

fn is_inside_sphere(pos: Vec3, sphere_pos: Vec3, radius: f32) -> bool {
    pos.distance(sphere_pos) <= radius
}
