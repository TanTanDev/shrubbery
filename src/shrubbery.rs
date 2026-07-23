use ahash::HashSet;

use crate::{
    attractor::Attractor,
    branch::{Branch, BranchFilter},
    math_utils::rotate_point,
    shape::Shape,
    voxel::{DecorationSelector, LeafDecoration, LeafShape, VoxelDefinitions, VoxelMapping},
};

use glam::{IVec3, Quat, Vec2, Vec3, ivec3, vec2, vec3};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ValueOrRangeU32 {
    Value(u32),
    /// Inclusive `[min, max]`.
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
    Value(f32),
    /// Inclusive `[min, max]`.
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

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GrowRadial {
    #[cfg_attr(feature = "serde", serde(default))]
    pub chance: StepChance,
    pub count: ValueOrRangeU32,
    pub pitch_degrees: ValueOrRangeF32,
    /// randomizes spacing between the branches
    /// 0.0: spaced equal distance. 1.0: maximum chaos  
    pub spacing_jitter: f32,
    pub branch_len: ValueOrRangeF32,
    pub branch_thickness: BranchThickness,
    pub decoration: DecorationSelector,
    #[cfg_attr(feature = "serde", serde(default))]
    pub assign_id: AssignBranchId,
    #[cfg_attr(feature = "serde", serde(default))]
    pub filter: BranchFilter,
}

/// One build step in a tree's growth recipe.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ShrubberyStep {
    SpawnRootBranch(SpawnRootBranch),
    GrowDirection(GrowDirection),
    GrowRadial(GrowRadial),
    /// Grow branches toward existing attractors using space colonization.
    GrowToAttractors(GrowToAttractors),
    /// Spawn attractors at a fixed world position.
    SpawnAttractors(SpawnAttractors),
    /// Spawn one attractor shape per selected branch tip.
    SpawnAttractorOnBranches(SpawnAttractorsOnBranches),
    ClearAttractors,
    /// Assign a leaf shape to branches matching the filter.
    ///
    /// Only branches that have not yet been assigned a leaf group are
    /// considered, unless `overwrite` is true.  Run this immediately after
    /// the growth step that produced the branches you want to decorate.
    SpawnLeaves(SpawnLeavesStep),
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ShrubberySettings {
    pub build_steps: Vec<ShrubberyStep>,
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AssignBranchId {
    #[default]
    AutoIncrement,
    AssignId(u32),
}

impl AssignBranchId {
    pub fn get(&self, last_id: u32) -> u32 {
        match self {
            AssignBranchId::AutoIncrement => last_id + 1,
            AssignBranchId::AssignId(id) => *id,
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GrowDirection {
    #[cfg_attr(feature = "serde", serde(default))]
    pub chance: StepChance,
    pub times: ValueOrRangeU32,
    pub trunk_growth_direction: BranchGrowthDirection,
    pub branch_len: ValueOrRangeF32,
    pub branch_thickness: BranchThickness,
    pub decoration: DecorationSelector,
    #[cfg_attr(feature = "serde", serde(default))]
    pub assign_id: AssignBranchId,
    #[cfg_attr(feature = "serde", serde(default))]
    pub filter: BranchFilter,
}

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
    #[cfg_attr(feature = "serde", serde(default))]
    pub chance: StepChance,
    pub times: ValueOrRangeU32,
    #[cfg_attr(feature = "serde", serde(default))]
    pub iteration_calculation: IterationCalculation,
    pub branch_len: ValueOrRangeF32,
    /// radius to delete attractors
    pub kill_distance: f32,
    /// how close an attractor has to be to pull branch
    pub leaf_attraction_dist: f32,
    pub decoration: DecorationSelector,
    pub branch_thickness: BranchThickness,
    #[cfg_attr(feature = "serde", serde(default))]
    pub assign_id: AssignBranchId,
    #[cfg_attr(feature = "serde", serde(default))]
    pub filter: BranchFilter,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BranchGrowthDirection {
    /// grow from parent normal
    Normal,
    Target(Vec3),
    WorldPos(Vec3),
    /// grow from parent normal, but apply gravity
    GravityLean {
        strength: f32,
    },
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum InitialDir {
    Value(Vec3),
    Random {
        /// angle in degrees
        y_rotation_range: ValueOrRangeU32,
        /// angle in degrees
        z_rotation_max: ValueOrRangeU32,
    },
}

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
pub struct SpawnRootBranch {
    #[cfg_attr(feature = "serde", serde(default))]
    pub assign_id: AssignBranchId,
    /// how many initial trunks to spawn
    pub count: usize,
    pub branch_len: ValueOrRangeF32,
    pub pos: Vec3,
    pub initial_dir: InitialDir,
}

impl Default for SpawnRootBranch {
    fn default() -> Self {
        Self {
            count: 1,
            initial_dir: InitialDir::Value(Vec3::Y),
            branch_len: ValueOrRangeF32::Value(2.0),
            pos: Vec3::ZERO,
            assign_id: AssignBranchId::default(),
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SpawnAttractors {
    #[cfg_attr(feature = "serde", serde(default))]
    pub chance: StepChance,
    pub pos: Vec3,
    pub shape: Shape,
    pub attractor_spacing: AttractorSpacing,
}

/// the chances that a `ShrubberyStep` will execute
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum StepChance {
    #[default]
    Always,
    Chance(f32),
}

impl StepChance {
    fn should_run(&self, rng: &mut ChaCha8Rng) -> bool {
        match self {
            StepChance::Always => true,
            StepChance::Chance(chance) => {
                let percent = chance / 100.0;
                rng.random_bool(percent as f64)
            }
        }
    }
}

/// Assign a leaf decoration to a set of branches at this point in the build.
///
/// The decoration is stored as a `LeafGroup` on the generator and referenced
/// by index from each selected branch, so the shape definition is shared
/// rather than cloned per branch.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SpawnLeavesStep {
    #[cfg_attr(feature = "serde", serde(default))]
    pub chance: StepChance,
    /// The voxel shape to place at each qualifying branch tip.
    pub shape: LeafShape,
    /// How to colour the leaf voxels.
    pub decoration: DecorationSelector,
    #[cfg_attr(feature = "serde", serde(default))]
    pub filter: BranchFilter,
    /// If true, overwrite any leaf group already assigned to a branch.
    /// If false (default), only undecorated branches are affected.
    #[cfg_attr(feature = "serde", serde(default))]
    pub overwrite: bool,
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
    #[cfg_attr(feature = "serde", serde(default))]
    pub chance: StepChance,
    /// How far along `offset_dir` from the branch tip to place the shape centre.
    pub offset_distance: f32,
    /// Direction to offset from the branch tip.
    pub offset_dir: BranchOffsetDir,
    /// The attractor volume shape.
    pub shape: Shape,
    /// Density / jitter settings for attractor placement inside the shape.
    pub attractor_spacing: AttractorSpacing,
    #[cfg_attr(feature = "serde", serde(default))]
    pub filter: BranchFilter,
}

impl ShrubberySettings {
    pub fn resolve_voxel_definitions(&mut self, voxel_definitions: &VoxelDefinitions) {
        for step in self.build_steps.iter_mut() {
            match step {
                ShrubberyStep::GrowDirection(grow_direction) => {
                    grow_direction.decoration.resolve(voxel_definitions);
                }
                ShrubberyStep::GrowToAttractors(grow_to_attractors) => {
                    grow_to_attractors.decoration.resolve(voxel_definitions);
                }
                ShrubberyStep::SpawnLeaves(spawn_leaves_step) => {
                    spawn_leaves_step.decoration.resolve(voxel_definitions);
                }
                ShrubberyStep::GrowRadial(grow_radial) => {
                    grow_radial.decoration.resolve(voxel_definitions);
                }
                ShrubberyStep::SpawnAttractors(_)
                | ShrubberyStep::SpawnAttractorOnBranches(_)
                | ShrubberyStep::ClearAttractors
                | ShrubberyStep::SpawnRootBranch(_) => (),
            }
        }
    }
}

impl Default for ShrubberySettings {
    fn default() -> Self {
        Self {
            build_steps: vec![ShrubberyStep::GrowToAttractors(GrowToAttractors {
                times: ValueOrRangeU32::Value(5),
                branch_len: ValueOrRangeF32::Value(5.0),
                kill_distance: 0.3,
                leaf_attraction_dist: 5.,
                decoration: DecorationSelector::Value(LeafDecoration::Solid(
                    VoxelMapping::default(),
                )),
                branch_thickness: BranchThickness::ValueOrRange(ValueOrRangeF32::Value(1.0)),
                filter: BranchFilter::default(),
                iteration_calculation: IterationCalculation::default(),
                assign_id: AssignBranchId::AutoIncrement,
                chance: StepChance::default(),
            })],
        }
    }
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

/// describes how to assign iteration value
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum IterationCalculation {
    #[default]
    /// iteration from the current commands "times" variable
    Iteration,
    /// same iteration value as parent
    Parent,
}

/// Working state for a single tree's procedural generation.
///
/// Build with [`ShrubberyGenerator::new`] (or the [`generate`][Self::generate]
/// shortcut), run the recipe's steps, then call [`voxelize`][Self::voxelize] to
/// produce the final voxel grid.
pub struct ShrubberyGenerator {
    pub branches: Vec<Branch>,
    pub attractors: Vec<Attractor>,
    pub min_bounds: Vec3,
    pub max_bounds: Vec3,
    pub rng: ChaCha8Rng,
    /// Seed this generator was created from. Voxelization re-derives
    /// per-branch RNG streams from it so output stays deterministic.
    pub seed: u64,
    /// All leaf group definitions registered via `SpawnLeaves` steps.
    /// Each entry is `(shape, decoration)`; branches reference by index.
    pub leaf_groups: Vec<(LeafShape, DecorationSelector)>,
    /// One entry per growth step, indexed by `Branch::decoration_group`.
    pub branch_decorations: Vec<DecorationSelector>,

    pub last_known_id: u32,
}

impl ShrubberyGenerator {
    pub fn new(seed: u64) -> Self {
        Self {
            branches: Vec::new(),
            attractors: Vec::new(),
            min_bounds: Vec3::ZERO,
            max_bounds: Vec3::ZERO,
            rng: ChaCha8Rng::seed_from_u64(seed),
            seed,
            leaf_groups: Vec::new(),
            branch_decorations: Vec::new(),
            last_known_id: 0,
        }
    }

    /// Create a generator and immediately run all of `settings`' build steps.
    ///
    /// Equivalent to [`new`][Self::new] followed by
    /// [`execute_all_steps`][Self::execute_all_steps]; useful when you don't
    /// need to drive individual steps.
    pub fn generate(seed: u64, settings: &ShrubberySettings) -> Self {
        let mut generator = Self::new(seed);
        generator.execute_all_steps(settings);
        generator
    }

    fn get_branch_indices_filtered(&self, filter: &BranchFilter) -> Vec<usize> {
        self.branches
            .iter()
            .enumerate()
            .filter_map(|(i, branch)| {
                filter
                    .should_include_branch(branch, self.last_known_id)
                    .then_some(i)
            })
            .collect()
    }

    pub fn execute_step(&mut self, step: &ShrubberyStep) {
        match step {
            ShrubberyStep::SpawnRootBranch(spawn_root_branch) => {
                self.spawn_root_branch(spawn_root_branch);
            }
            ShrubberyStep::GrowToAttractors(grow_to_attractors) => {
                self.grow_to_attractors(grow_to_attractors);
            }
            ShrubberyStep::SpawnAttractors(spawn_attractor) => {
                spawn_attractor.shape.generate(
                    spawn_attractor.pos,
                    &mut self.attractors,
                    &spawn_attractor.attractor_spacing,
                    &mut self.rng,
                );
            }
            ShrubberyStep::GrowDirection(grow_trunk) => {
                self.grow_direction(grow_trunk);
            }
            ShrubberyStep::SpawnAttractorOnBranches(s) => {
                self.spawn_attractors_on_branches(s);
            }
            ShrubberyStep::SpawnLeaves(s) => {
                self.spawn_leaves(s);
            }
            ShrubberyStep::ClearAttractors => {
                self.attractors.clear();
            }
            ShrubberyStep::GrowRadial(grow_radial) => {
                self.grow_radial(grow_radial);
            }
        }
    }

    pub fn execute_all_steps(&mut self, settings: &ShrubberySettings) {
        for step in settings.build_steps.iter() {
            self.execute_step(step);
        }
    }

    /// Spawn one attractor shape per selected branch, offset from that branch
    /// along the configured direction.
    pub fn spawn_attractors_on_branches(&mut self, s: &SpawnAttractorsOnBranches) {
        if !s.chance.should_run(&mut self.rng) {
            return;
        }
        // Snapshot tip positions/dirs first so we don't hold a borrow on
        // self.branches while mutating self.attractors below.
        let origins: Vec<(Vec3, Vec3)> = self
            .branches
            .iter()
            // Roots have no direction worth offsetting from.
            .filter(|b| b.parent_index.is_some())
            .filter(|b| s.filter.should_include_branch(b, self.last_known_id))
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

    /// Assign a leaf group to all branches matching the filter.
    ///
    /// Registers a new `(shape, decoration)` group in `leaf_groups`, then sets
    /// `branch.leaf_group = Some(group_index)` on every qualifying branch.
    /// By default only undecorated branches are touched; set `overwrite = true`
    /// to re-decorate already-assigned branches.
    pub fn spawn_leaves(&mut self, step: &SpawnLeavesStep) {
        if !step.chance.should_run(&mut self.rng) {
            return;
        }
        let group_index = self.leaf_groups.len();
        self.leaf_groups
            .push((step.shape.clone(), step.decoration.clone()));

        // Roots are skipped: with IterationFilter::Last they'd register as the
        // last iteration and get decorated at the tree base.
        for branch in self
            .branches
            .iter_mut()
            .filter(|branch| branch.parent_index.is_some())
        {
            let qualifies = step
                .filter
                .should_include_branch(branch, self.last_known_id);
            if !qualifies {
                continue;
            }
            if branch.leaf_group.is_none() || step.overwrite {
                branch.leaf_group = Some(group_index);
            }
        }
    }

    /// Half the larger of the X/Z bounding dimensions, rounded up.
    pub fn bounding_square_half(&self) -> f32 {
        let size = self.bounding_size();
        (size.x.max(size.z) as f32 * 0.5).ceil()
    }

    pub fn bounding_size(&self) -> IVec3 {
        let (min_bounds, max_bounds) = self.bounds();
        max_bounds - min_bounds
    }

    /// Integer min/max corners of the axis-aligned bounding box.
    pub fn bounds(&self) -> (IVec3, IVec3) {
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

    /// Expand the bounding box to include `pos`, padded by `radius` for thickness.
    fn update_bound(&mut self, pos: Vec3, radius: f32) {
        self.min_bounds.x = self.min_bounds.x.min(pos.x - radius);
        self.min_bounds.y = self.min_bounds.y.min(pos.y - radius);
        self.min_bounds.z = self.min_bounds.z.min(pos.z - radius);
        self.max_bounds.x = self.max_bounds.x.max(pos.x + radius);
        self.max_bounds.y = self.max_bounds.y.max(pos.y + radius);
        self.max_bounds.z = self.max_bounds.z.max(pos.z + radius);
    }

    pub fn grow_direction(&mut self, grow_trunk: &GrowDirection) {
        if !grow_trunk.chance.should_run(&mut self.rng) {
            return;
        }

        let decoration_index = self.branch_decorations.len();
        self.branch_decorations.push(grow_trunk.decoration.clone());
        let grow_times = grow_trunk.times.get(&mut self.rng);

        let indices = self.get_branch_indices_filtered(&grow_trunk.filter);

        let id = grow_trunk.assign_id.get(self.last_known_id);
        self.last_known_id = id;
        for branch_index in indices.iter() {
            let mut running_index = *branch_index;
            for i in 0..grow_times {
                let thickness = grow_trunk
                    .branch_thickness
                    .get(i, grow_times, &mut self.rng);

                let mut new_branch = self.branches[running_index].child(
                    running_index,
                    grow_trunk.branch_len.get(&mut self.rng),
                    id,
                    &grow_trunk.trunk_growth_direction,
                    thickness,
                    i,
                    grow_times,
                );
                new_branch.decoration_group = Some(decoration_index);
                self.branches[*branch_index].child_count += 1;
                self.update_bound(new_branch.pos, thickness);
                self.branches.push(new_branch);
                running_index = self.branches.len() - 1;
            }
        }
    }

    pub fn grow_radial(&mut self, grow_radial: &GrowRadial) {
        if !grow_radial.chance.should_run(&mut self.rng) {
            return;
        }
        let decoration_index = self.branch_decorations.len();
        self.branch_decorations.push(grow_radial.decoration.clone());

        let indices = self.get_branch_indices_filtered(&grow_radial.filter);

        let id = grow_radial.assign_id.get(self.last_known_id);
        self.last_known_id = id;

        for branch_index in indices {
            let count = grow_radial.count.get(&mut self.rng);
            if count == 0 {
                continue;
            }

            let rotation_offset = self.rng.random_range(0.0..360.0);

            for i in 0..count {
                let thickness = grow_radial.branch_thickness.get(i, count, &mut self.rng);
                let spacing = 360.0 / count as f32;
                let v = spacing * 0.5 * grow_radial.spacing_jitter;
                let jitter = self.rng.random_range(-v..=v);

                let yaw = (rotation_offset + spacing * i as f32 + jitter).to_radians();

                let pitch = grow_radial.pitch_degrees.get(&mut self.rng).to_radians();
                let mut dir = Vec3::X;

                dir = Quat::from_rotation_y(yaw) * dir;
                dir = Quat::from_axis_angle(dir.cross(Vec3::Y).normalize(), pitch) * dir;

                let mut new_branch = self.branches[branch_index].child(
                    branch_index,
                    grow_radial.branch_len.get(&mut self.rng),
                    id,
                    &BranchGrowthDirection::Target(dir),
                    thickness,
                    i,
                    count,
                );

                new_branch.decoration_group = Some(decoration_index);
                self.branches[branch_index].child_count += 1;

                self.update_bound(new_branch.pos, thickness);

                self.branches.push(new_branch);
            }
        }
    }

    pub fn grow_to_attractors(&mut self, grow_to_attractors: &GrowToAttractors) {
        if !grow_to_attractors.chance.should_run(&mut self.rng) {
            return;
        }
        let times = grow_to_attractors.times.get(&mut self.rng);

        let group_index = self.branch_decorations.len();
        self.branch_decorations
            .push(grow_to_attractors.decoration.clone());

        let id = grow_to_attractors.assign_id.get(self.last_known_id);
        self.last_known_id = id;

        let mut active_branches: HashSet<usize> = self
            .branches
            .iter_mut()
            .enumerate()
            // never grow from the root
            .filter(|(_i, branch)| branch.parent_index.is_some())
            .filter(|(_i, branch)| {
                grow_to_attractors
                    .filter
                    .should_include_branch(branch, self.last_known_id)
            })
            .map(|(i, _branch)| i)
            .collect();

        for i in 0..times {
            for attractor in self.attractors.iter_mut() {
                let mut closest_branch: Option<usize> = None;
                let mut closest_dist = 999999.;
                for (branch_index, branch) in self
                    .branches
                    .iter_mut()
                    .enumerate()
                    .filter(|(i, _branch)| active_branches.contains(i))
                {
                    let dist = attractor.pos.distance(branch.pos);
                    if dist < grow_to_attractors.kill_distance {
                        attractor.reached = true;
                        closest_branch = None;
                        break;
                    }
                    if dist > grow_to_attractors.leaf_attraction_dist {
                        continue;
                    }
                    if dist < closest_dist {
                        closest_branch = Some(branch_index);
                        closest_dist = dist;
                    }
                }
                if let Some(closest_branch_index) = closest_branch {
                    let closest_branch_pos = self.branches[closest_branch_index].pos;
                    let new_branch_dir = attractor.pos - closest_branch_pos;
                    let new_branch_dir = new_branch_dir.normalize();
                    self.branches[closest_branch_index].dir += new_branch_dir;
                    self.branches[closest_branch_index].attractors_count += 1;
                }
            }
            self.attractors.retain(|attractor| !attractor.reached);

            let mut to_add = vec![];
            for (branch_index, branch) in self
                .branches
                .iter_mut()
                .enumerate()
                .filter(|(_, branch)| branch.attractors_count > 0)
            {
                let mut branch_rng = rand_chacha::ChaCha8Rng::seed_from_u64(branch_index as u64);
                let thickness = grow_to_attractors
                    .branch_thickness
                    .get(i, times, &mut branch_rng);
                branch.dir = branch.dir.normalize();

                let (iteration_value, iteration_max) =
                    match grow_to_attractors.iteration_calculation {
                        IterationCalculation::Iteration => (i, times),
                        IterationCalculation::Parent => (branch.iteration, branch.iteration_total),
                    };

                let mut new_branch = branch.child(
                    branch_index,
                    grow_to_attractors.branch_len.get(&mut self.rng),
                    id,
                    &BranchGrowthDirection::Normal,
                    thickness,
                    iteration_value,
                    iteration_max,
                );
                self.last_known_id = u32::max(self.last_known_id, new_branch.id);
                new_branch.decoration_group = Some(group_index);
                branch.child_count += 1;
                to_add.push(new_branch);
                branch.reset();
            }
            // Bounds are updated outside the branch iterator so we don't hold
            // two `&mut self` borrows at once.
            for branch in &to_add {
                self.update_bound(branch.pos, branch.thickness);
            }
            let branch_len = self.branches.len();
            self.branches.extend(to_add);
            // Only newly-spawned branches are eligible to grow next round;
            // their parents already extended this iteration.
            active_branches.clear();
            active_branches.extend(branch_len..self.branches.len());
        }
    }

    /// Droop branches downward, scaled by horizontal distance from the root
    /// so the canopy sags more than the trunk.
    pub fn post_process_gravity(&mut self, gravity: f32) {
        let plane_half_size = self.bounding_square_half();
        for branch in self.branches.iter_mut() {
            let dist_to_root = vec2(branch.pos.x, branch.pos.z).distance(Vec2::ZERO);
            let weight = dist_to_root / plane_half_size;
            branch.pos.y -= weight * gravity;
        }
    }

    /// Twist branches around the Y axis, more strongly far from the root and
    /// higher up, for a spiralling canopy.
    pub fn post_process_spin(&mut self, spin_amount: f32) {
        let plane_half_size = self.bounding_square_half();
        for branch in self.branches.iter_mut() {
            let branch_xz = vec2(branch.pos.x, branch.pos.z);
            let dist_to_root = branch_xz.distance(Vec2::ZERO);
            let weight = dist_to_root / plane_half_size;
            let y_weight = (branch.pos.y * 0.3).cos() * 0.5 + 0.5;
            let new_xz = rotate_point(branch_xz, spin_amount * weight * y_weight);
            branch.pos.x = new_xz.x;
            branch.pos.z = new_xz.y;
        }
    }

    fn spawn_root_branch(&mut self, spawn_root_branch: &SpawnRootBranch) {
        for i in 0..spawn_root_branch.count {
            let dir = spawn_root_branch.initial_dir.get(&mut self.rng);
            let root = Branch {
                pos: spawn_root_branch.pos,
                parent_index: None,
                dir,
                attractors_count: 0,
                original_dir: dir,
                child_count: 0,
                iteration: i as u32,
                leaf_group: None,
                thickness: 1.0,
                decoration_group: None,
                iteration_total: spawn_root_branch.count as u32,
                id: spawn_root_branch.assign_id.get(self.last_known_id),
            };
            self.last_known_id = root.id;
            self.branches.push(root);
        }
    }
}
