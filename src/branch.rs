use glam::Vec3;

use crate::shrubbery::BranchGrowthDirection;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A single segment of the generated tree.
#[derive(Debug)]
pub struct Branch {
    pub pos: Vec3,
    pub parent_index: Option<usize>,
    pub dir: Vec3,
    /// Thickness radius used when rasterizing bark.
    pub thickness: f32,
    /// Direction before attractors pulled it; restored by [`reset`](Self::reset).
    pub original_dir: Vec3,
    /// How many attractors are currently pulling this node.
    pub attractors_count: i32,
    pub child_count: i32,
    /// Generation id, used for filtering.
    pub id: u32,
    pub iteration: u32,
    pub iteration_total: u32,
    /// Index into the generator's `leaf_groups`, set by a `SpawnLeaves` step.
    pub leaf_group: Option<usize>,
    /// Index into the generator's `branch_decorations`.
    pub decoration_group: Option<usize>,
}

impl Branch {
    /// Build the next segment growing from this one along `growth_dir`.
    #[allow(clippy::too_many_arguments)]
    pub fn child(
        &self,
        index: usize,
        mut branch_len: f32,
        id: u32,
        growth_dir: &BranchGrowthDirection,
        thickness: f32,
        iteration: u32,
        iteration_total: u32,
    ) -> Self {
        let dir = match growth_dir {
            BranchGrowthDirection::Normal => self.dir,
            BranchGrowthDirection::GravityLean { strength } => {
                (self.dir + Vec3::NEG_Y * *strength).normalize()
            }
            BranchGrowthDirection::Target(dir) => *dir,
            BranchGrowthDirection::WorldPos(world_pos) => {
                let to_world = world_pos - self.pos;
                branch_len = to_world.length();
                to_world.normalize()
            }
        };

        Self {
            pos: self.pos + dir * branch_len,
            parent_index: Some(index),
            dir,
            attractors_count: 0,
            original_dir: dir,
            child_count: 0,
            leaf_group: None,
            thickness,
            decoration_group: None,
            iteration,
            iteration_total,
            id,
        }
    }

    pub fn reset(&mut self) {
        self.attractors_count = 0;
        self.dir = self.original_dir;
    }
}

#[cfg(feature = "serde")]
fn default_true() -> bool {
    true
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BranchFilter {
    /// Skip branches that already have a leaf group assigned.
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub ignore_shapes: bool,
    /// Skip root branches (those with no parent).
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub ignore_root: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub id_filter: IdFilter,
    #[cfg_attr(feature = "serde", serde(default))]
    pub iteration_filter: IterationFilter,
}

impl Default for BranchFilter {
    fn default() -> Self {
        Self {
            ignore_shapes: true,
            ignore_root: true,
            id_filter: IdFilter::default(),
            iteration_filter: IterationFilter::default(),
        }
    }
}

impl BranchFilter {
    pub fn should_include_branch(&self, branch: &Branch, last_id: u32) -> bool {
        if self.ignore_shapes && branch.leaf_group.is_some() {
            return false;
        }
        if self.ignore_root && branch.parent_index.is_none() {
            return false;
        }
        if !self.id_filter.is_id_included(branch, last_id) {
            return false;
        }
        if !self
            .iteration_filter
            .is_iteration_included(branch.iteration, branch.iteration_total)
        {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum IterationFilter {
    #[default]
    All,
    Last,
    Target(u32),
    Greater(u32),
    Lower(u32),
}

impl IterationFilter {
    pub fn is_iteration_included(&self, iteration: u32, max: u32) -> bool {
        match self {
            IterationFilter::All => true,
            IterationFilter::Last => iteration == u32::checked_sub(max, 1).unwrap_or(u32::MAX),
            IterationFilter::Greater(higher) => iteration > *higher,
            IterationFilter::Lower(lower) => iteration < *lower,
            IterationFilter::Target(target) => *target == iteration,
        }
    }
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum IdFilter {
    #[default]
    Last,
    All,
    Target(u32),
}

impl IdFilter {
    pub fn is_id_included(&self, branch: &Branch, last_generation: u32) -> bool {
        match self {
            IdFilter::All => true,
            IdFilter::Last => branch.id == last_generation,
            IdFilter::Target(target_gen) => branch.id == *target_gen,
        }
    }
}
