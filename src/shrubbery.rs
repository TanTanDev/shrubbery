//! the core implementation for generating fine shrubberies
use ahash::HashSet;

use crate::{
    attractor::Attractor,
    branch::Branch,
    prelude::*,
    shape::AttractorShape,
    voxel::{DecorationSelector, Shape, VoxelDefinitions},
};

use glam::{IVec3, Quat, Vec3, ivec3, vec3};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// first iteration of a [`ShrubberyStep::Grow`]
/// you may want to spawn multiple branches radially, example: palm leaves
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BranchSpawnMethod {
    #[default]
    /// Only spawn 1 new branch
    Single,
    /// Spawn multiple branches in a radial formation
    GrowRadial(GrowRadial),
}

/// data for [`BranchSpawnMethod::GrowRadial`]
/// describes how to radially grow branches
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GrowRadial {
    /// how many branches to spawn initially
    pub count: ValueOrRangeU32,
    // angle of new branch
    pub pitch_degrees: ValueOrRangeF32,
    /// randomizes spacing between the branches
    /// 0.0: spaced equal distance. 1.0: maximum chaos  
    pub spacing_jitter: f32,
}

impl Default for GrowRadial {
    fn default() -> Self {
        Self {
            count: ValueOrRangeU32::Value(1),
            pitch_degrees: ValueOrRangeF32::Value(0.0),
            spacing_jitter: 1.0,
        }
    }
}

/// The most fundamental building block
/// Defines what action will be performed
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ShrubberyStep {
    /// Spawn a position where branches can grow from
    SpawnRoot(SpawnRootStep),
    /// Grow branches. specify how via `spawn_method`
    Grow(GrowStep),
    /// Spawn attractors, based upon `shape` and `location`
    SpawnAttractors(SpawnAttractors),
    /// delete ALL generated attractor points
    ClearAttractors,
    /// Assign a leaf shape to branches
    /// Already assigned leaf groups are skipped unless `overwrite` is true
    Shape(ShapeStep),
}

/// The core build instructions describing how to make a fine shrubbery
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ShrubberySettings {
    pub build_steps: Vec<ShrubberyStep>,
}

/// described what Id to assign a [`ShrubberyStep`]
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

/// data for [`ShrubberyStep::Grow`]
/// Describes how to grow branches
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct GrowStep {
    /// Chance that this grow step will execute
    pub chance: StepChance,
    /// Initial spawn method, single or radial (only executed first `times` iteration)
    pub spawn_method: BranchSpawnMethod,
    /// How many times to grow
    pub times: ValueOrRangeU32,
    /// What dir to grow the branch
    pub dir: BranchGrowthDirection,
    /// Branch length
    pub length: ValueOrRangeF32,
    /// Branch thickness
    pub thickness: BranchThickness,
    /// How voxels are decorated
    pub voxel: DecorationSelector,
    /// How to assign id to branches (used for Filtering)
    pub id: AssignBranchId,
    /// Filters what branches to grow from
    pub filter: Filter,
}

/// Describes how to select thickness value for [`Branch`]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BranchThickness {
    ValueOrRange(ValueOrRangeF32),
    /// thickness is scaled based upon the branch's start and end pos
    IterationScale {
        min: ValueOrRangeF32,
        max: ValueOrRangeF32,
    },
}

impl Default for BranchThickness {
    fn default() -> Self {
        Self::ValueOrRange(ValueOrRangeF32::default())
    }
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

/// describes what direction a branch should grow when executing [`ShrubberyStep::Grow`] commands
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BranchGrowthDirection {
    /// Grow from (dir, derived from parent branch)
    #[default]
    Normal,
    /// Grow in this direction (normalized internally)
    Target(Vec3),
    /// Branch will arrive at this world pos
    WorldPos(Vec3),
    /// Grow from dir, (derived from parent normal), but apply gravity on Y axis
    GravityLean { strength: f32 },
    /// Grow branches using space-colonization towards attractor points
    Attractor(AttractorSettings),
}

/// describes how attractors should behave in the space-colonization step
/// used inside [`BranchGrowthDirection::Attractor`]
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct AttractorSettings {
    /// Delete attractors within this distance, on arrival
    pub kill_distance: f32,
    /// How close an attractor has to be to pull branch
    pub attract_distance: f32,
    /// Describes how to assign iteration value
    pub iteration_calculation: IterationCalculation,
}

impl Default for AttractorSettings {
    fn default() -> Self {
        Self {
            kill_distance: 5.0,
            attract_distance: 10.0,
            iteration_calculation: IterationCalculation::default(),
        }
    }
}

impl BranchGrowthDirection {
    pub fn attractors(&self) -> Option<&AttractorSettings> {
        match self {
            Self::Attractor(attractor) => Some(attractor),
            _ => None,
        }
    }
}

/// the initial root direction [`SpawnRootStep`]
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

impl Default for InitialDir {
    fn default() -> Self {
        Self::Random {
            y_rotation_range: ValueOrRangeU32::Range(0, 360),
            z_rotation_max: ValueOrRangeU32::Range(0, 2),
        }
    }
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

/// data for [`ShrubberyStep::SpawnRoot`]
/// describes how to spawn root positions where [`Branch`] can grow from
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct SpawnRootStep {
    pub id: AssignBranchId,
    /// how many initial trunks to spawn
    pub times: ValueOrRangeU32,
    pub pos: Vec3,
    pub initial_dir: InitialDir,
}

impl Default for SpawnRootStep {
    fn default() -> Self {
        Self {
            times: ValueOrRangeU32::Value(1),
            initial_dir: InitialDir::default(),
            pos: Vec3::ZERO,
            id: AssignBranchId::default(),
        }
    }
}

/// describes where to spawn attractor shape
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SpawnAttractorLocation {
    Pos(Vec3),
    FromBranch(FromBranchSettings),
}

/// data for [`SpawnAttractorLocation::FromBranch`]
/// describes where to spawn the attractor shape, in relation to branch
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FromBranchSettings {
    #[cfg_attr(feature = "serde", serde(default))]
    pub offset: BranchOffsetDir,
    #[cfg_attr(feature = "serde", serde(default = "default_spawn_attractor_filter"))]
    pub filter: Filter,
}

/// attractor shapes, are more commonly spawned at end of branches iteration
#[cfg(feature = "serde")]
fn default_spawn_attractor_filter() -> Filter {
    Filter {
        iteration: IterationFilter::Last,
        ..Default::default()
    }
}

/// data for [`ShrubberyStep::SpawnAttractors`]
/// describes where/how attractors should spawn
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SpawnAttractors {
    #[cfg_attr(feature = "serde", serde(default))]
    pub chance: StepChance,
    pub location: SpawnAttractorLocation,
    pub shape: AttractorShape,
    #[cfg_attr(feature = "serde", serde(default))]
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
pub struct ShapeStep {
    #[cfg_attr(feature = "serde", serde(default))]
    pub chance: StepChance,
    /// The voxel shape to place at each qualifying branch tip.
    pub shape: Shape,
    /// How to colour the leaf voxels.
    pub voxel: DecorationSelector,
    #[cfg_attr(feature = "serde", serde(default))]
    pub filter: Filter,
    /// If true, overwrite any leaf group already assigned to a branch.
    /// If false (default), only undecorated branches are affected.
    #[cfg_attr(feature = "serde", serde(default))]
    pub overwrite: bool,
}

/// Offset direction when placing the attractor shape relative to a branch tip.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BranchOffsetDir {
    #[default]
    Zero,
    /// Offset along the branch's own direction vector.
    BranchForward(f32),
    /// Offset straight up in world space (Y+).
    WorldUp(f32),
    /// Offset straight down in world space (Y-).
    WorldDown(f32),
    /// Offset along the branch direction, projected flat onto the XZ plane and
    /// useful for palm fronds that fan out horizontally.
    BranchForwardFlat(f32),
}

impl BranchOffsetDir {
    pub fn offset_dir(&self, dir: Vec3) -> Vec3 {
        match self {
            BranchOffsetDir::BranchForward(distance) => dir.normalize_or(Vec3::Y) * distance,
            BranchOffsetDir::WorldUp(distance) => Vec3::Y * distance,
            BranchOffsetDir::BranchForwardFlat(distance) => {
                Vec3::new(dir.x, 0.0, dir.z).normalize_or(Vec3::X) * distance
            }
            BranchOffsetDir::WorldDown(distance) => Vec3::NEG_Y * distance,
            BranchOffsetDir::Zero => Vec3::ZERO,
        }
    }
}

impl ShrubberySettings {
    pub fn resolve_voxel_definitions(&mut self, voxel_definitions: &VoxelDefinitions) {
        for step in self.build_steps.iter_mut() {
            match step {
                ShrubberyStep::Grow(grow_direction) => {
                    grow_direction.voxel.resolve(voxel_definitions);
                }
                ShrubberyStep::Shape(spawn_leaves_step) => {
                    spawn_leaves_step.voxel.resolve(voxel_definitions);
                }
                ShrubberyStep::SpawnAttractors(_)
                | ShrubberyStep::ClearAttractors
                | ShrubberyStep::SpawnRoot(_) => (),
            }
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
            attractor_spacing: 5.0,
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
    pub leaf_groups: Vec<(Shape, DecorationSelector)>,
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

    /// Create a generator and immediately execute all steps
    pub fn generate(seed: u64, settings: &ShrubberySettings) -> Self {
        let mut generator = Self::new(seed);
        generator.execute_all_steps(settings);
        generator
    }

    fn branch_indices_filtered_vec(&self, filter: &Filter) -> Vec<usize> {
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

    fn branch_indices_filtered_hashset(&self, filter: &Filter) -> HashSet<usize> {
        self.branches
            .iter()
            .enumerate()
            .filter_map(|(i, b)| {
                filter
                    .should_include_branch(b, self.last_known_id)
                    .then_some(i)
            })
            .collect()
    }

    pub fn execute_step(&mut self, step: &ShrubberyStep) {
        match step {
            ShrubberyStep::SpawnRoot(spawn_root_branch) => {
                self.spawn_root_branch(spawn_root_branch);
            }
            ShrubberyStep::SpawnAttractors(spawn_attractor) => match &spawn_attractor.location {
                SpawnAttractorLocation::Pos(pos) => {
                    spawn_attractor.shape.generate(
                        *pos,
                        &mut self.attractors,
                        &spawn_attractor.attractor_spacing,
                        &mut self.rng,
                    );
                }
                SpawnAttractorLocation::FromBranch(from_branch) => {
                    let origins: Vec<(Vec3, Vec3)> = self
                        .branches
                        .iter()
                        .filter(|b| {
                            from_branch
                                .filter
                                .should_include_branch(b, self.last_known_id)
                        })
                        .map(|b| (b.pos, b.dir))
                        .collect();

                    for (pos, dir) in origins {
                        let shape_centre = pos + from_branch.offset.offset_dir(dir);
                        spawn_attractor.shape.generate(
                            shape_centre,
                            &mut self.attractors,
                            &spawn_attractor.attractor_spacing,
                            &mut self.rng,
                        );
                    }
                }
            },
            ShrubberyStep::Grow(grow_trunk) => {
                self.grow_direction(grow_trunk);
            }
            ShrubberyStep::Shape(s) => {
                self.spawn_leaves(s);
            }
            ShrubberyStep::ClearAttractors => {
                self.attractors.clear();
            }
        }
    }

    pub fn execute_all_steps(&mut self, settings: &ShrubberySettings) {
        for step in settings.build_steps.iter() {
            self.execute_step(step);
        }
    }

    /// Assign a leaf group to all branches matching the filter.
    ///
    /// Registers a new `(shape, decoration)` group in `leaf_groups`, then sets
    /// `branch.leaf_group = Some(group_index)` on every qualifying branch.
    /// By default only undecorated branches are touched; set `overwrite = true`
    /// to re-decorate already-assigned branches.
    pub fn spawn_leaves(&mut self, step: &ShapeStep) {
        if !step.chance.should_run(&mut self.rng) {
            return;
        }
        let group_index = self.leaf_groups.len();
        self.leaf_groups
            .push((step.shape.clone(), step.voxel.clone()));

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

    pub fn grow_direction(&mut self, grow_trunk: &GrowStep) {
        if !grow_trunk.chance.should_run(&mut self.rng) {
            return;
        }

        let decoration_index = self.branch_decorations.len();
        self.branch_decorations.push(grow_trunk.voxel.clone());
        let times = grow_trunk.times.get(&mut self.rng);

        let id = grow_trunk.id.get(self.last_known_id);

        match grow_trunk.dir.attractors() {
            Some(attractor_settings) => {
                let mut active_branches = self.branch_indices_filtered_hashset(&grow_trunk.filter);
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
                            if dist < attractor_settings.kill_distance {
                                attractor.reached = true;
                                closest_branch = None;
                                break;
                            }
                            if dist > attractor_settings.attract_distance {
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
                        let mut branch_rng =
                            rand_chacha::ChaCha8Rng::seed_from_u64(branch_index as u64);
                        let thickness = grow_trunk.thickness.get(i, times, &mut branch_rng);
                        branch.dir = branch.dir.normalize();

                        let (iteration_value, iteration_max) =
                            match attractor_settings.iteration_calculation {
                                IterationCalculation::Iteration => (i, times),
                                IterationCalculation::Parent => {
                                    (branch.iteration, branch.iteration_total)
                                }
                            };

                        let mut new_branch = branch.child(
                            branch_index,
                            grow_trunk.length.get(&mut self.rng),
                            id,
                            &BranchGrowthDirection::Normal,
                            thickness,
                            iteration_value,
                            iteration_max,
                        );
                        self.last_known_id = u32::max(self.last_known_id, new_branch.id);
                        new_branch.decoration_group = Some(decoration_index);
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
            None => match &grow_trunk.spawn_method {
                BranchSpawnMethod::Single => {
                    let indices = self.branch_indices_filtered_vec(&grow_trunk.filter);
                    self.grow_branches_from_indices(
                        grow_trunk,
                        decoration_index,
                        times,
                        indices,
                        id,
                    );
                }
                BranchSpawnMethod::GrowRadial(grow_radial) => {
                    self.grow_radial(grow_trunk, grow_radial, decoration_index, times, id);
                }
            },
        }
        self.last_known_id = id;
    }

    fn grow_branches_from_indices(
        &mut self,
        grow_step: &GrowStep,
        decoration_index: usize,
        grow_times: u32,
        indices: Vec<usize>,
        id: u32,
    ) {
        for branch_index in indices.iter() {
            let mut running_index = *branch_index;
            for i in 0..grow_times {
                let thickness = grow_step.thickness.get(i, grow_times, &mut self.rng);

                let mut new_branch = self.branches[running_index].child(
                    running_index,
                    grow_step.length.get(&mut self.rng),
                    id,
                    &grow_step.dir,
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
    pub fn grow_radial(
        &mut self,
        grow_step: &GrowStep,
        grow_radial: &GrowRadial,
        decoration_index: usize,
        times: u32,
        id: u32,
    ) {
        // let mut active_branches = self.branch_indices_filtered_hashset(&grow_step.filter);
        let mut active_branches = self.branch_indices_filtered_vec(&grow_step.filter);
        // for branch_index in active_branches.iter() {
        let new_branch_count = grow_radial.count.get(&mut self.rng);
        if new_branch_count == 0 {
            // continue;
            return;
        }
        // randomize base direction
        let rotation_offset = self.rng.random_range(0.0..360.0);
        let degrees_per_segment = 360.0 / new_branch_count as f32;
        let allowed_degree_range = degrees_per_segment * 0.5 * grow_radial.spacing_jitter;

        if times == 0 {
            return;
        }

        // first grow radially ONCE
        let mut to_add = vec![];
        for branch_index in active_branches.iter() {
            for i in 0..new_branch_count {
                let degree_jitter = self
                    .rng
                    .random_range(-allowed_degree_range..=allowed_degree_range);

                let yaw =
                    (rotation_offset + degrees_per_segment * i as f32 + degree_jitter).to_radians();

                let pitch = grow_radial.pitch_degrees.get(&mut self.rng).to_radians();
                let mut dir = Vec3::X;
                dir = Quat::from_rotation_y(yaw) * dir;
                dir = Quat::from_axis_angle(dir.cross(Vec3::Y).normalize(), pitch) * dir;

                let thickness = grow_step.thickness.get(i, new_branch_count, &mut self.rng);
                let mut new_branch = self.branches[*branch_index].child(
                    *branch_index,
                    grow_step.length.get(&mut self.rng),
                    id,
                    &BranchGrowthDirection::Target(dir),
                    thickness,
                    i,
                    new_branch_count,
                );

                new_branch.decoration_group = Some(decoration_index);
                self.branches[*branch_index].child_count += 1;

                self.update_bound(new_branch.pos, thickness);
                to_add.push(new_branch);
            }
        }
        active_branches.clear();
        let branch_len = self.branches.len();
        for new_branch in to_add.into_iter() {
            self.branches.push(new_branch);
        }
        active_branches.extend(branch_len..self.branches.len());

        // continue growing, from the newly created radial branches
        let grow_times_left = times - 1;
        if grow_times_left > 0 {
            self.grow_branches_from_indices(
                grow_step,
                decoration_index,
                grow_times_left,
                active_branches.into_iter().collect(),
                id,
            );
        }
    }

    fn spawn_root_branch(&mut self, spawn_root_branch: &SpawnRootStep) {
        let times = spawn_root_branch.times.get(&mut self.rng);
        for i in 0..times {
            let dir = spawn_root_branch.initial_dir.get(&mut self.rng);
            let root = Branch {
                pos: spawn_root_branch.pos,
                parent_index: None,
                dir,
                attractors_count: 0,
                original_dir: dir,
                child_count: 0,
                iteration: i,
                leaf_group: None,
                thickness: 1.0,
                decoration_group: None,
                iteration_total: times,
                id: spawn_root_branch.id.get(self.last_known_id),
            };
            self.last_known_id = root.id;
            self.branches.push(root);
        }
    }
}
