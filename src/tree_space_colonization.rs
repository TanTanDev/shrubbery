use crate::{
    attractor::Attractor,
    branch::Branch,
    math_utils::{dist_to_line, rotate_point},
    shape::Shape,
    voxel::{
        BranchRootSizeIncreaser, BranchSizeSetting, LeafDecoration, LeafShape, VoxelDefinitions,
        VoxelMapping,
    },
};

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
    pub fn max(&self) -> f32 {
        match self {
            ValueOrRangeF32::Value(v) => *v,
            ValueOrRangeF32::Range(_, m) => *m,
        }
    }
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
    /// Grow branches in a fixed direction (trunk building)
    GrowDirection(GrowDirection),
    /// Run space-colonization growth toward existing attractors
    GrowToAttractors(GrowToAttractors),
    /// Spawn attractors at a fixed world position
    SpawnAttractor(SpawnAttractors),
    /// Spawn attractor shapes relative to each current tip branch
    SpawnAttractorOnBranches(SpawnAttractorsOnBranches),
    /// Remove all existing attractors
    ClearAttractors,
    /// Assign a leaf shape to branches matching the selector.
    ///
    /// Only branches that have not yet been assigned a leaf group are
    /// considered, unless `overwrite` is true.  Run this immediately after
    /// the growth step that produced the branches you want to decorate.
    SpawnLeaves(SpawnLeavesStep),
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SpaceColonizationSettings {
    /// how the initial branches will spawn
    pub trunk_settings: TrunkSettings,
    /// steps, how to grow/shape/modify the tree
    pub build_steps: Vec<SpaceColonizationStep>,
    /// what voxels to use for bark
    pub bark_decorator: BarkDecorator,
    /// branch voxel thickness (global, can vary by generation)
    pub branch_size_setting: BranchSizeSetting,
    /// optional extra width added near the root
    pub branch_root_size_increaser: Option<BranchRootSizeIncreaser>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GrowDirection {
    pub times: ValueOrRangeU32,
    pub trunk_growth_direction: TrunkGrowthDirection,
    pub branch_len: ValueOrRangeF32,
    pub branch_thickness: BranchThickness,
    pub decoration: LeafDecoration,
}
// use some form of mapping

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BranchThickness {
    ValueOrRange(ValueOrRangeF32),
    IterationScale {
        min: ValueOrRangeF32,
        max: ValueOrRangeF32,
    },
}

impl BranchThickness {
    pub fn get(&self, i: u32, i_max: u32, rng: &mut ChaCha8Rng) -> f32 {
        match self {
            BranchThickness::ValueOrRange(value) => value.get(rng),
            BranchThickness::IterationScale { min, max } => {
                let min = min.get(rng);
                let max = max.get(rng);

                let t = if i_max <= 1 {
                    1.0
                } else {
                    i as f32 / (i_max - 1) as f32
                };

                min + (max - min) * t
            }
        }
    }
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
    pub decoration: LeafDecoration,
    pub branch_thickness: BranchThickness,
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

/// Assign a leaf decoration to a set of branches at this point in the build.
///
/// The decoration is stored as a `LeafGroup` on the generator and referenced
/// by index from each selected branch, so the shape definition is shared
/// rather than cloned per branch.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SpawnLeavesStep {
    /// Which branches to assign leaves to.
    pub selector: BranchSelector,
    /// The voxel shape to place at each qualifying branch tip.
    pub shape: LeafShape,
    /// How to colour the leaf voxels.
    pub decoration: LeafDecoration,
    /// If true, overwrite any leaf group already assigned to a branch.
    /// If false (default), only undecorated branches are affected.
    #[serde(default)]
    pub overwrite: bool,
}

/// Which branches to use as anchor points when spawning per-branch attractors.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BranchSelector {
    /// Only branches with no children (current tips).
    Tips,
    /// Branches whose generation is exactly this value.
    ExactGeneration(i32),
    /// Branches whose generation equals or exceeds this value.
    MinGeneration(i32),
    /// Branches whose generation is at most this value.
    MaxGeneration(i32),
    /// Branches whose Y position is at or above this world-space value.
    MinHeight(f32),
    /// Branches whose Y position is at or below this world-space value.
    MaxHeight(f32),
    /// Tips that also satisfy a minimum generation.
    TipsWithMinGeneration(i32),
    /// Tips that also satisfy an exact generation.
    TipsWithExactGeneration(i32),
}

/// Offset direction when placing the attractor shape relative to a branch tip.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BranchOffsetDir {
    /// Offset along the branch's own direction vector.
    BranchForward,
    /// Offset straight up in world space (Y+).
    WorldUp,
    /// Offset straight down in world space (Y-).
    WorldDown,
    /// Offset along the branch direction, projected flat onto the XZ plane and
    /// then normalised — useful for palm fronds that fan out horizontally.
    BranchForwardFlat,
}

/// Spawn one copy of an attractor shape per selected branch, placed relative
/// to that branch's tip.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SpawnAttractorsOnBranches {
    /// Which branches act as origins for the spawned shapes.
    pub selector: BranchSelector,
    /// How far along `offset_dir` from the branch tip to place the shape centre.
    pub offset_distance: f32,
    /// Direction to offset from the branch tip.
    pub offset_dir: BranchOffsetDir,
    /// The attractor volume shape.
    pub shape: Shape,
    /// Density / jitter settings for attractor placement inside the shape.
    pub attractor_spacing: AttractorSpacing,
}

impl SpaceColonizationSettings {
    pub fn make_generator(&self, seed: u64) -> TreeGeneratorSpaceColonization {
        TreeGeneratorSpaceColonization::new(&self, seed)
    }

    pub fn resolve_voxel_definitions(&mut self, voxel_definitions: &VoxelDefinitions) {
        match &mut self.bark_decorator {
            BarkDecorator::Single(voxel_mapping) => {
                voxel_mapping.resolve(voxel_definitions);
            }
        }
        for step in self.build_steps.iter_mut() {
            if let SpaceColonizationStep::SpawnLeaves(s) = step {
                s.decoration.resolve(voxel_definitions);
            }
            if let SpaceColonizationStep::GrowDirection(grow_dir) = step {
                grow_dir.decoration.resolve(voxel_definitions);
            }
            if let SpaceColonizationStep::GrowToAttractors(grow_to_attractors) = step {
                grow_to_attractors.decoration.resolve(voxel_definitions);
            }
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
                decoration: LeafDecoration::Single(VoxelMapping::default()),
                branch_thickness: BranchThickness::ValueOrRange(ValueOrRangeF32::Value(1.0)),
            })],
            bark_decorator: BarkDecorator::Single(VoxelMapping::default()),
            branch_size_setting: BranchSizeSetting::default(),
            branch_root_size_increaser: None,
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
    /// All leaf group definitions registered via `SpawnLeaves` steps.
    /// Each entry is `(shape, decoration)`; branches reference by index.
    pub leaf_groups: Vec<(LeafShape, LeafDecoration)>,
    // todo proper naming
    pub branch_decorations: Vec<LeafDecoration>,
}

/// Returns true if `branch` matches the given selector.
/// Centralised here so all steps (SpawnLeaves, SpawnAttractorOnBranches, etc.)
/// use identical logic.
fn branch_selector_matches(selector: &BranchSelector, b: &Branch) -> bool {
    match selector {
        BranchSelector::Tips => b.child_count == 0,
        BranchSelector::ExactGeneration(generation) => b.generation == *generation,
        BranchSelector::MinGeneration(min) => b.generation >= *min,
        BranchSelector::MaxGeneration(max) => b.generation <= *max,
        BranchSelector::MinHeight(min_y) => b.pos.y >= *min_y,
        BranchSelector::MaxHeight(max_y) => b.pos.y <= *max_y,
        BranchSelector::TipsWithMinGeneration(min) => b.child_count == 0 && b.generation >= *min,
        BranchSelector::TipsWithExactGeneration(g) => b.child_count == 0 && b.generation == *g,
    }
}

impl TreeGeneratorSpaceColonization {
    // pub fn new(root_pos: Vec3, initial_dir: Vec3, seed: u64) -> Self {
    pub fn new(settings: &SpaceColonizationSettings, seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut branches = Vec::new();

        for i in 0..settings.trunk_settings.count {
            let dir = settings.trunk_settings.initial_dir.get(&mut rng);
            let root = Branch {
                pos: settings.trunk_settings.root_pos,
                parent_index: None,
                dir,
                attractors_count: 0,
                original_dir: dir,
                child_count: 0,
                generation: i as i32,
                leaf_group: None,
                thickness: 1.0,
                decoration_group: None,
                generation_total: settings.trunk_settings.count as i32,
            };
            branches.push(root);
        }

        Self {
            branches,
            attractors: Vec::new(),
            min_bounds: Vec3::splat(0f32),
            max_bounds: Vec3::splat(0f32),
            rng,
            leaf_groups: Vec::new(),
            branch_decorations: Vec::new(),
        }
    }

    pub fn execute_step(&mut self, step: &SpaceColonizationStep) {
        match step {
            SpaceColonizationStep::GrowToAttractors(grow_to_attractors) => {
                let times = grow_to_attractors.times.get(&mut self.rng);
                for i in 0..times {
                    self.grow_to_attractors(grow_to_attractors, i, times);
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
                self.grow_direction(grow_trunk);
            }
            SpaceColonizationStep::SpawnAttractorOnBranches(s) => {
                self.spawn_attractors_on_branches(s);
            }
            SpaceColonizationStep::SpawnLeaves(s) => {
                self.spawn_leaves(s);
            }
            SpaceColonizationStep::ClearAttractors => {
                self.attractors.clear();
            }
        }
    }

    pub fn execute_all_step(&mut self, settings: &SpaceColonizationSettings) {
        for step in settings.build_steps.iter() {
            self.execute_step(step);
        }
    }

    /// Spawn one attractor shape per selected branch, offset from that branch
    /// along the configured direction.
    pub fn spawn_attractors_on_branches(&mut self, s: &SpawnAttractorsOnBranches) {
        // Snapshot tip positions/dirs so we don't hold a borrow on self.branches.
        let origins: Vec<(Vec3, Vec3)> = self
            .branches
            .iter()
            .filter(|b| branch_selector_matches(&s.selector, b))
            .map(|b| (b.pos, b.dir))
            .collect();

        for (pos, dir) in origins {
            let offset_dir = match &s.offset_dir {
                BranchOffsetDir::BranchForward => dir.normalize_or(Vec3::Y),
                BranchOffsetDir::WorldUp => Vec3::Y,
                BranchOffsetDir::BranchForwardFlat => {
                    Vec3::new(dir.x, 0.0, dir.z).normalize_or(Vec3::X)
                }
                BranchOffsetDir::WorldDown => Vec3::NEG_Y,
            };
            let shape_centre = pos + offset_dir * s.offset_distance;
            s.shape.generate(
                shape_centre,
                &mut self.attractors,
                &s.attractor_spacing,
                &mut self.rng,
            );
        }
    }

    /// Assign a leaf group to all branches matching the selector.
    ///
    /// Registers a new `LeafGroup` (shape + decoration) in `self.leaf_groups`,
    /// then sets `branch.leaf_group = Some(group_index)` on every qualifying
    /// branch.  By default only undecorated branches are touched; set
    /// `step.overwrite = true` to re-decorate already-assigned branches.
    pub fn spawn_leaves(&mut self, step: &SpawnLeavesStep) {
        let group_index = self.leaf_groups.len();
        self.leaf_groups
            .push((step.shape.clone(), step.decoration.clone()));

        for branch in self.branches.iter_mut() {
            let qualifies = branch_selector_matches(&step.selector, branch);
            if !qualifies {
                continue;
            }
            if branch.leaf_group.is_none() || step.overwrite {
                branch.leaf_group = Some(group_index);
            }
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
    pub fn update_bound(
        min_bounds: &mut Vec3,
        max_bounds: &mut Vec3,
        branch_pos: Vec3,
        // expand if a branch has a thickness
        radius: f32,
    ) {
        min_bounds.x = min_bounds.x.min(branch_pos.x - radius);
        min_bounds.y = min_bounds.y.min(branch_pos.y - radius);
        min_bounds.z = min_bounds.z.min(branch_pos.z - radius);
        max_bounds.x = max_bounds.x.max(branch_pos.x + radius);
        max_bounds.y = max_bounds.y.max(branch_pos.y + radius);
        max_bounds.z = max_bounds.z.max(branch_pos.z + radius);
    }

    pub fn grow_direction(&mut self, grow_trunk: &GrowDirection) {
        let grow_times = grow_trunk.times.get(&mut self.rng);

        let group_index = self.branch_decorations.len();
        self.branch_decorations.push(grow_trunk.decoration.clone());
        for i in 0..grow_times {
            let thickness = grow_trunk
                .branch_thickness
                .get(i, grow_times, &mut self.rng);

            // todo: instead find the ends of all root branches
            let last_index = self.branches.len() - 1;
            let mut new_branch = self.branches[last_index].next(
                last_index,
                grow_trunk.branch_len.get(&mut self.rng),
                true,
                &grow_trunk.trunk_growth_direction,
                thickness,
                grow_times as i32,
            );
            new_branch.decoration_group = Some(group_index);
            self.branches[last_index].child_count += 1;
            Self::update_bound(
                &mut self.min_bounds,
                &mut self.max_bounds,
                new_branch.pos,
                thickness,
            );
            self.branches.push(new_branch);
        }
    }

    /// using space colonization algorithm, spawn new branches
    pub fn grow_to_attractors(
        &mut self,
        grow_to_attractors: &GrowToAttractors,
        call_i: u32,
        call_times: u32,
    ) {
        let group_index = self.branch_decorations.len();
        self.branch_decorations
            .push(grow_to_attractors.decoration.clone());
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
            let mut branch_rng = rand_chacha::ChaCha8Rng::seed_from_u64(branch_index as u64);
            let thickness =
                grow_to_attractors
                    .branch_thickness
                    .get(call_i, call_times, &mut branch_rng);
            branch.dir = branch.dir.normalize();
            // todo: fix thickness
            let mut new_branch = branch.next(
                branch_index,
                grow_to_attractors.branch_len.get(&mut self.rng),
                true,
                &TrunkGrowthDirection::Normal,
                // todo: proper thickness
                thickness,
                1,
            );
            new_branch.decoration_group = Some(group_index);
            branch.child_count += 1;
            Self::update_bound(
                &mut self.min_bounds,
                &mut self.max_bounds,
                new_branch.pos,
                thickness,
            );
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
