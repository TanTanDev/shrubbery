use crate::{
    attractor::Attractor,
    branch::Branch,
    math_utils::{dist_to_line, rotate_point},
    shape::{BoxShape, Shape},
    voxel::{LeafSetting, VoxelDefinitions, VoxelMapping, VoxelizeSettings},
};

use bevy::log::info;
#[cfg(feature = "bevy")]
use bevy::{asset::Asset, reflect::TypePath};

use glam::{IVec3, Vec2, Vec3, ivec3, vec2, vec3};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ValueOrRangeU32 {
    /// constant value
    Value(u32),
    /// value chosen between (min, max) inclusive
    Range(u32, u32),
}

impl ValueOrRangeU32 {
    pub fn get(&self, rng: &mut ChaCha8Rng) -> u32 {
        match self {
            ValueOrRangeU32::Value(v) => *v,
            ValueOrRangeU32::Range(min, max) => rng.random_range(*min..=*max),
        }
    }
}

/// instructions for how to build a space colonization tree
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SpaceColonizationStep {
    /// how many times to run the space colonization algorithm
    Grow { value_or_range: ValueOrRangeU32 },
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SpaceColonizationSettings {
    /// first branch position
    #[serde(default)]
    pub root_pos: Vec3,
    /// first branch direction
    #[serde(default = "default_initial_dir")]
    pub initial_dir: Vec3,
    /// radius to delete attractors
    pub kill_distance: f32,
    /// how far a single branch reaches
    pub branch_len: f32,
    /// how close an attractor has to be to pull branch
    pub leaf_attraction_dist: f32,
    /// force the minimum trunk height, within (min, max)
    pub min_trunk_height: ValueOrRangeU32,
    /// steps, how to grow/shape/modify the tree
    pub build_steps: Vec<SpaceColonizationStep>,
    /// what voxels to use for bark
    pub bark_decorator: BarkDecorator,
    /// how to spawn the leafs
    pub voxelize_settings: VoxelizeSettings,
}

fn default_initial_dir() -> Vec3 {
    Vec3::Y
}

impl SpaceColonizationSettings {
    pub fn make_generator(&self, seed: u64) -> TreeGeneratorSpaceColonization {
        let mut generator =
            TreeGeneratorSpaceColonization::new(self.root_pos, self.initial_dir, seed);

        let attractor_settings = AttractorGeneratorSettings::default();

        generator.spawn_attractors_from_shape(
            // vec3(0., 5. + 15.0, 0.),
            vec3(0., 5. + 8.0, 0.),
            BoxShape {
                x: 15.0,
                y: 10.0,
                z: 15.,
            },
            &self,
            &attractor_settings,
        );
        generator.build_trunk(&self);
        generator
    }

    pub fn resolve_voxel_definitions(&mut self, voxel_definitions: &VoxelDefinitions) {
        match &mut self.bark_decorator {
            BarkDecorator::Single(voxel_mapping) => {
                voxel_mapping.resolve(voxel_definitions);
            }
        }
        match &mut self.voxelize_settings.leaf_settings {
            LeafSetting::None => (),
            LeafSetting::BranchIsLeaf(_leaf_classifier) => (),
            LeafSetting::Shape {
                shape: _,
                decoration,
            } => decoration.resolve(voxel_definitions),
        }
    }
}

impl Default for SpaceColonizationSettings {
    fn default() -> Self {
        Self {
            root_pos: Vec3::ZERO,
            initial_dir: Vec3::Y,
            kill_distance: 0.3,
            branch_len: 0.3,
            leaf_attraction_dist: 5.,
            min_trunk_height: ValueOrRangeU32::Value(1),
            build_steps: vec![SpaceColonizationStep::Grow {
                value_or_range: ValueOrRangeU32::Value(5),
            }],
            bark_decorator: BarkDecorator::Single(VoxelMapping::default()),
            voxelize_settings: VoxelizeSettings::default(),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum BarkDecorator {
    /// use the same bark voxel
    Single(VoxelMapping),
}

/// describes how many attractor positions spawn
pub struct AttractorGeneratorSettings {
    pub max_leaves: Option<i32>,
    pub min_leaves: Option<i32>,
    // a value of 1: will spawn enough leaves to expand the whole area
    // 1.0 is minimum recommended value, higher values will yield potentially more branching
    // but higher values also generates more leaves, tolling performance
    pub density: f32,
}

impl Default for AttractorGeneratorSettings {
    fn default() -> Self {
        Self {
            max_leaves: Some(500),
            min_leaves: Some(30),
            density: 1.0,
        }
    }
}

pub struct TreeGeneratorSpaceColonization {
    pub branches: Vec<Branch>,
    pub attractors: Vec<Attractor>,
    pub min_bounds: Vec3,
    pub max_bounds: Vec3,
    pub rng: ChaCha8Rng,
}

impl TreeGeneratorSpaceColonization {
    pub fn new(root_pos: Vec3, initial_dir: Vec3, seed: u64) -> Self {
        let rng = ChaCha8Rng::seed_from_u64(seed);
        let mut branches = Vec::new();
        let root = Branch {
            pos: root_pos,
            parent_index: None,
            dir: initial_dir,
            attractors_count: 0,
            original_dir: initial_dir,
            child_count: 0,
            generation: 0,
        };
        branches.push(root);
        Self {
            branches,
            attractors: Vec::new(),
            min_bounds: Vec3::splat(0f32),
            max_bounds: Vec3::splat(0f32),
            rng,
        }
    }

    pub fn execute_step(
        &mut self,
        step: &SpaceColonizationStep,
        settings: &SpaceColonizationSettings,
    ) {
        match step {
            SpaceColonizationStep::Grow { value_or_range } => {
                for _ in 0..value_or_range.get(&mut self.rng) {
                    self.grow(settings);
                }
            }
        }
    }

    pub fn execute_all_step(&mut self, settings: &SpaceColonizationSettings) {
        for step in settings.build_steps.iter() {
            self.execute_step(step, settings);
        }
    }

    pub fn get_bound_square_half(&self) -> f32 {
        let size = self.get_bounding_size();
        (size.x.max(size.z) as f32 * 0.5).ceil()
    }

    pub fn get_bounding_size(&self) -> IVec3 {
        let (min_bounds, max_bounds) = self.get_bounds();
        max_bounds - min_bounds
    }

    /// returns the min x,y,z and max x,y,z position
    pub fn get_bounds(&self) -> (IVec3, IVec3) {
        (
            ivec3(
                self.min_bounds.x.ceil() as i32,
                self.min_bounds.y.ceil() as i32,
                self.min_bounds.z.ceil() as i32,
            ),
            ivec3(
                self.max_bounds.x.ceil() as i32,
                self.max_bounds.y.ceil() as i32,
                self.max_bounds.z.ceil() as i32,
            ),
        )
    }

    /// expand bounding if branch_pos is outside
    pub fn update_bound(min_bounds: &mut Vec3, max_bounds: &mut Vec3, branch_pos: Vec3) {
        min_bounds.x = min_bounds.x.min(branch_pos.x);
        min_bounds.y = min_bounds.y.min(branch_pos.y);
        min_bounds.z = min_bounds.z.min(branch_pos.z);
        max_bounds.x = max_bounds.x.max(branch_pos.x);
        max_bounds.y = max_bounds.y.max(branch_pos.y);
        max_bounds.z = max_bounds.z.max(branch_pos.z);
    }

    /// spawn initial branches based on settings.
    pub fn build_trunk(&mut self, settings: &SpaceColonizationSettings) {
        let mut root_end_pos = self.branches[0].pos;
        let dir = self.branches[0].dir;
        let mut consumed_height = 0.;
        // the first root will be as long as it needs to be until it starts gaining attractions
        let max_iterations = 1000;
        'a: for _ in 0..max_iterations {
            consumed_height += settings.branch_len;
            root_end_pos += settings.branch_len * dir;
            for leaf in self.attractors.iter() {
                let dist = root_end_pos.distance(leaf.pos);
                if dist < settings.leaf_attraction_dist {
                    break 'a;
                }
            }
        }
        info!("constumed height: {:?}", consumed_height);

        self.branches[0].child_count += 1;
        let new_branch = self.branches[0].next(0, consumed_height, false);
        Self::update_bound(&mut self.min_bounds, &mut self.max_bounds, new_branch.pos);
        self.branches.push(new_branch);

        // keep adding branches upwards until we reach the trunk_height
        let trunk_height = settings.min_trunk_height.get(&mut self.rng) as f32;
        while consumed_height < trunk_height {
            consumed_height += settings.branch_len;
            let last_index = self.branches.len() - 1;
            let new_branch = self.branches[last_index].next(last_index, settings.branch_len, false);
            self.branches[last_index].child_count += 1;
            Self::update_bound(&mut self.min_bounds, &mut self.max_bounds, new_branch.pos);
            self.branches.push(new_branch)
        }
    }

    /// using space colonization algorithm, spawn new branches
    pub fn grow(&mut self, settings: &SpaceColonizationSettings) {
        info!("growing");
        for leaf in self.attractors.iter_mut() {
            let mut closest_branch: Option<usize> = None;
            let mut closest_dist = 999999.;
            // find shortest signed distance of all branches
            for (branch_index, branch) in self.branches.iter_mut().enumerate() {
                let dist = leaf.pos.distance(branch.pos);
                // is this branch to close to the leaf, discard it
                if dist < settings.kill_distance {
                    leaf.reached = true;
                    closest_branch = None;
                    break;
                }
                // to far away to be attracted towards
                if dist > settings.leaf_attraction_dist {
                    continue;
                }
                // record closest branch
                if dist < closest_dist {
                    closest_branch = Some(branch_index);
                    closest_dist = dist;
                }
            }
            // pull closest branch towards attractor
            if let Some(closest_branch_index) = closest_branch {
                let closest_branch_pos = self.branches[closest_branch_index].pos;
                let new_branch_dir = leaf.pos - closest_branch_pos;
                let new_branch_dir = new_branch_dir.normalize();
                self.branches[closest_branch_index].dir += new_branch_dir;
                self.branches[closest_branch_index].attractors_count += 1;
            }
        }
        // remove reached leaves
        self.attractors.retain(|leaf| !leaf.reached);

        // spawn new branches using previous calculations
        let mut to_add = vec![];
        for (branch_index, branch) in self
            .branches
            .iter_mut()
            .enumerate()
            .filter(|(_, branch)| branch.attractors_count > 0)
        {
            branch.dir = branch.dir.normalize();
            let new_branch = branch.next(branch_index, settings.branch_len, true);
            branch.child_count += 1;
            Self::update_bound(&mut self.min_bounds, &mut self.max_bounds, new_branch.pos);
            to_add.push(new_branch);
            branch.reset();
        }
        self.branches.extend(to_add);
    }

    /// spawn particles inside provided shape, based on settings
    pub fn spawn_attractors_from_shape<TShape>(
        &mut self,
        pos: Vec3,
        shape: TShape,
        settings: &SpaceColonizationSettings,
        attractor_generator_settings: &AttractorGeneratorSettings,
    ) where
        TShape: Shape,
    {
        shape.generate(
            pos,
            &mut self.attractors,
            settings,
            attractor_generator_settings,
            &mut self.rng,
        );
    }

    /// reduce y position of branches, weighted by dist to 0,0 xz.
    pub fn post_process_gravity(&mut self, gravity: f32) {
        let plane_half_size = self.get_bound_square_half();
        for branch in self.branches.iter_mut() {
            // branch.
            let branch_plane = vec2(branch.pos.x, branch.pos.z);
            let root = Vec2::ZERO;
            let dist_to_root = branch_plane.distance(root);

            let weight = dist_to_root / plane_half_size;
            branch.pos.y -= weight * gravity;
        }
    }

    /// rotate branch x,z positions around origin: 0,0
    pub fn post_process_spin(&mut self, spin_amount: f32) {
        let plane_half_size = self.get_bound_square_half();
        for branch in self.branches.iter_mut() {
            let branch_xz = vec2(branch.pos.x, branch.pos.z);
            let root = Vec2::ZERO;
            let dist_to_root = branch_xz.distance(root);
            let weight = dist_to_root / plane_half_size;

            let y_weight = (branch.pos.y * 0.3).cos() * 0.5 + 0.5;

            let new_xz = rotate_point(branch_xz, spin_amount * weight * y_weight);
            branch.pos.x = new_xz.x;
            branch.pos.z = new_xz.y;
        }
    }

    /// returns (distance, index of branch)
    pub fn distance_to_branch(&self, pos: Vec3) -> (f32, usize) {
        let mut closest = f32::MAX;
        let mut index = 0;
        for (i, branch) in self.branches.iter().enumerate() {
            let Some(parent_index) = branch.parent_index else {
                continue;
            };

            let d = dist_to_line(pos, self.branches[parent_index].pos, branch.pos);
            if d < closest {
                closest = d;
                index = i;
            }
        }
        (closest, index)
    }
}
