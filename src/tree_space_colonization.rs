use crate::{
    attractor::Attractor,
    branch::Branch,
    math_utils::{dist_to_line, rotate_point},
    shape::{CubeShape, Shape},
    voxel::{LeafSetting, VoxelDefinitions, VoxelMapping, VoxelizeSettings},
};

use glam::{IVec3, Quat, Vec2, Vec3, ivec3, vec2, vec3};
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
            ValueOrRangeU32::Range(min, max) => rng.random_range((*min).min(*max)..=*max),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ValueOrRangeF32 {
    /// constant value
    Value(f32),
    /// value chosen between (min, max) inclusive
    Range(f32, f32),
}

impl ValueOrRangeF32 {
    pub fn get(&self, rng: &mut ChaCha8Rng) -> f32 {
        match self {
            ValueOrRangeF32::Value(v) => *v,
            ValueOrRangeF32::Range(min, max) => rng.random_range((*min).min(*max)..=*max),
        }
    }
}

/// instructions for how to build a space colonization tree
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SpaceColonizationStep {
    /// how many times to run the space colonization algorithm
    GrowDirection(GrowDirection),
    GrowToAttractors(GrowToAttractors),
    SpawnAttractor(SpawnAttractors),
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SpaceColonizationSettings {
    // how the initial branches will spawn
    pub trunk_settings: TrunkSettings,
    /// steps, how to grow/shape/modify the tree
    pub build_steps: Vec<SpaceColonizationStep>,
    /// what voxels to use for bark
    pub bark_decorator: BarkDecorator,
    /// how to spawn the leafs
    pub voxelize_settings: VoxelizeSettings,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GrowDirection {
    pub times: ValueOrRangeU32,
    pub trunk_growth_direction: TrunkGrowthDirection,
    pub branch_len: ValueOrRangeF32,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GrowToAttractors {
    pub times: ValueOrRangeU32,
    pub branch_len: ValueOrRangeF32,
    /// radius to delete attractors
    pub kill_distance: f32,
    /// how close an attractor has to be to pull branch
    pub leaf_attraction_dist: f32,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TrunkGrowthDirection {
    Normal,
    GravityLean { strength: f32 },
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum InitialDir {
    /// specific direction
    Value(Vec3),
    Random {
        /// angle in degrees
        y_rotation_range: ValueOrRangeU32,
        /// angle in degrees
        z_rotation_max: ValueOrRangeU32,
    },
}

// impl InitialDir {
//     pub fn get(&self, rng: &mut ChaCha8Rng) -> Vec3 {
//         Vec3::Y
//     }
// }

impl InitialDir {
    pub fn get(&self, rng: &mut ChaCha8Rng) -> Vec3 {
        match self {
            InitialDir::Value(dir) => dir.normalize(),

            InitialDir::Random {
                y_rotation_range,
                z_rotation_max,
            } => {
                let yaw = (y_rotation_range.get(rng) as f32).to_radians();

                let tilt = (z_rotation_max.get(rng) as f32).to_radians();

                let horizontal = vec3(yaw.cos(), 0.0, yaw.sin());

                (horizontal * tilt.sin() + Vec3::Y * tilt.cos()).normalize()
            }
        }
    }
}
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TrunkSettings {
    /// how many initial trunks to spawn
    count: usize,
    branch_len: ValueOrRangeF32,
    root_pos: Vec3,
    initial_dir: InitialDir,
}

impl Default for TrunkSettings {
    fn default() -> Self {
        Self {
            count: 1,
            initial_dir: InitialDir::Value(Vec3::Y),
            branch_len: ValueOrRangeF32::Value(2.0),
            root_pos: Vec3::ZERO,
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SpawnAttractors {
    pub pos: Vec3,
    pub shape: Shape,
    pub attractor_spacing: AttractorSpacing,
}

impl SpawnAttractors {}

impl SpaceColonizationSettings {
    pub fn make_generator(&self, seed: u64) -> TreeGeneratorSpaceColonization {
        let mut generator = TreeGeneratorSpaceColonization::new(&self, seed);

        let attractor_settings = AttractorSpacing::default();

        // generator.spawn_attractors_from_shape(
        //     vec3(0., 5. + 8.0, 0.),
        //     Shape::Box(BoxShape {
        //         size_x: 15.0,
        //         size_y: 10.0,
        //         size_z: 15.,
        //     }),
        //     &self,
        //     &attractor_settings,
        // );
        // generator.build_trunk(&self);
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
            build_steps: vec![SpaceColonizationStep::GrowToAttractors(GrowToAttractors {
                times: ValueOrRangeU32::Value(5),
                branch_len: ValueOrRangeF32::Value(5.0),
                kill_distance: 0.3,
                leaf_attraction_dist: 5.,
            })],
            bark_decorator: BarkDecorator::Single(VoxelMapping::default()),
            voxelize_settings: VoxelizeSettings::default(),
            trunk_settings: TrunkSettings::default(),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum BarkDecorator {
    /// use the same bark voxel
    Single(VoxelMapping),
}

/// describes how many attractors to spawn in shape
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AttractorSpacing {
    /// how far away the attractors spawn in the shape
    pub attractor_spacing: f32,
    // how far to randomize the attractor positions.
    // 0.0: perfect grid spacing. 1.0: attractors can just touch.
    pub jitter_ratio: f32,
}

impl Default for AttractorSpacing {
    fn default() -> Self {
        Self {
            attractor_spacing: 3.0,
            jitter_ratio: 1.0,
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
    // pub fn new(root_pos: Vec3, initial_dir: Vec3, seed: u64) -> Self {
    pub fn new(settings: &SpaceColonizationSettings, seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut branches = Vec::new();

        for _i in 0..settings.trunk_settings.count {
            let dir = settings.trunk_settings.initial_dir.get(&mut rng);
            let root = Branch {
                pos: settings.trunk_settings.root_pos,
                parent_index: None,
                dir,
                attractors_count: 0,
                original_dir: dir,
                child_count: 0,
                generation: 0,
            };
            branches.push(root);
        }

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
            SpaceColonizationStep::GrowToAttractors(grow_to_attractors) => {
                for _ in 0..grow_to_attractors.times.get(&mut self.rng) {
                    self.grow(settings, grow_to_attractors);
                }
            }
            SpaceColonizationStep::SpawnAttractor(spawn_attractor) => {
                spawn_attractor.shape.generate(
                    spawn_attractor.pos,
                    &mut self.attractors,
                    &spawn_attractor.attractor_spacing,
                    &mut self.rng,
                );
            }
            SpaceColonizationStep::GrowDirection(grow_trunk) => {
                self.grow_trunk(grow_trunk);
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
    // pub fn build_trunk(&mut self, settings: &SpaceColonizationSettings) {
    //     let mut root_end_pos = self.branches[0].pos;
    //     let dir = self.branches[0].dir;
    //     let mut consumed_height = 0.;
    //     // the first root will be as long as it needs to be until it starts gaining attractions
    //     let max_iterations = 1000;
    //     'a: for _i in 0..max_iterations {
    //         consumed_height += settings.branch_len;
    //         root_end_pos += settings.branch_len * dir;
    //         for leaf in self.attractors.iter() {
    //             let dist = root_end_pos.distance(leaf.pos);
    //             if dist < settings.leaf_attraction_dist {
    //                 break 'a;
    //             }
    //         }
    //     }

    //     self.branches[0].child_count += 1;
    //     let new_branch = self.branches[0].next(0, consumed_height, false);
    //     Self::update_bound(&mut self.min_bounds, &mut self.max_bounds, new_branch.pos);
    //     self.branches.push(new_branch);

    // keep adding branches upwards until we reach the trunk_height
    // let trunk_height = settings.min_trunk_height.get(&mut self.rng) as f32;
    // for _i in 0..max_iterations {
    //     if consumed_height > trunk_height {
    //         break;
    //     }
    //     consumed_height += settings.branch_len;
    //     let last_index = self.branches.len() - 1;
    //     let new_branch = self.branches[last_index].next(last_index, settings.branch_len, false);
    //     self.branches[last_index].child_count += 1;
    //     Self::update_bound(&mut self.min_bounds, &mut self.max_bounds, new_branch.pos);
    //     self.branches.push(new_branch);
    // }
    // }
    pub fn grow_trunk(&mut self, grow_trunk: &GrowDirection) {
        for _i in 0..grow_trunk.times.get(&mut self.rng) {
            // todo: instead find the ends of all root branches
            let last_index = self.branches.len() - 1;
            let new_branch = self.branches[last_index].next(
                last_index,
                grow_trunk.branch_len.get(&mut self.rng),
                false,
                &grow_trunk.trunk_growth_direction,
            );
            self.branches[last_index].child_count += 1;
            Self::update_bound(&mut self.min_bounds, &mut self.max_bounds, new_branch.pos);
            self.branches.push(new_branch);
        }
    }

    /// using space colonization algorithm, spawn new branches
    pub fn grow(
        &mut self,
        settings: &SpaceColonizationSettings,
        grow_to_attractors: &GrowToAttractors,
    ) {
        for attractor in self.attractors.iter_mut() {
            let mut closest_branch: Option<usize> = None;
            let mut closest_dist = 999999.;
            // find shortest signed distance of all branches
            for (branch_index, branch) in self
                .branches
                .iter_mut()
                // don't grow branches from the root
                .enumerate()
                .filter(|(_i, branch)| branch.parent_index.is_some())
            {
                let dist = attractor.pos.distance(branch.pos);
                // is this branch to close to the leaf, discard it
                if dist < grow_to_attractors.kill_distance {
                    attractor.reached = true;
                    closest_branch = None;
                    break;
                }
                // to far away to be attracted towards
                if dist > grow_to_attractors.leaf_attraction_dist {
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
                let new_branch_dir = attractor.pos - closest_branch_pos;
                let new_branch_dir = new_branch_dir.normalize();
                self.branches[closest_branch_index].dir += new_branch_dir;
                self.branches[closest_branch_index].attractors_count += 1;
            }
        }
        self.attractors.retain(|attractor| !attractor.reached);

        // spawn new branches using previous calculations
        let mut to_add = vec![];
        for (branch_index, branch) in self
            .branches
            .iter_mut()
            .enumerate()
            .filter(|(_, branch)| branch.attractors_count > 0)
        {
            branch.dir = branch.dir.normalize();
            let new_branch = branch.next(
                branch_index,
                grow_to_attractors.branch_len.get(&mut self.rng),
                true,
                &TrunkGrowthDirection::Normal,
            );
            branch.child_count += 1;
            Self::update_bound(&mut self.min_bounds, &mut self.max_bounds, new_branch.pos);
            to_add.push(new_branch);
            branch.reset();
        }
        self.branches.extend(to_add);
    }

    /// spawn particles inside provided shape, based on settings
    pub fn spawn_attractors_from_shape(
        &mut self,
        pos: Vec3,
        shape: Shape,
        attractor_generator_settings: &AttractorSpacing,
    ) {
        shape.generate(
            pos,
            &mut self.attractors,
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
