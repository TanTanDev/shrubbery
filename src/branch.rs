use glam::Vec3;

use crate::shrubbery::TrunkGrowthDirection;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Branch {
    pub pos: Vec3,
    pub parent_index: Option<usize>,
    pub dir: Vec3,

    /// the thickness radius for making bark
    pub thickness: f32,
    /// todo: remove? might not be needed
    pub original_dir: Vec3,
    /// how many attractors are pulling this node
    pub attractors_count: i32,
    /// how many children branches this branch has
    pub child_count: i32,
    /// id used for filtering
    pub id: u32,
    /// what iteration this branch was made
    pub iteration: u32,
    /// what iteration this branch was made
    pub iteration_total: u32,
    /// Index into the generator's `leaf_groups` vec, set by a `SpawnLeaves`
    /// build step.  `None` means no leaf decoration has been assigned yet.
    pub leaf_group: Option<usize>,

    // todo name
    pub decoration_group: Option<usize>,
}

impl Branch {
    /// make a new branch based on this branch calculated growth direciton
    pub fn next(
        &self,
        index: usize,
        branch_len: f32,
        id: u32,
        trunk_growth_dir: &TrunkGrowthDirection,
        thickness: f32,
        iteration: u32,
        iteration_total: u32,
    ) -> Self {
        let dir = match trunk_growth_dir {
            TrunkGrowthDirection::Normal => self.dir,
            TrunkGrowthDirection::GravityLean { strength } => {
                (self.dir + Vec3::NEG_Y * *strength).normalize()
                // self.dir * branch_len
            }
            TrunkGrowthDirection::Target(dir) => *dir,
        };

        Self {
            pos: self.pos + dir * branch_len,
            parent_index: Some(index),
            dir: dir,
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

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BranchFilter {
    #[serde(default = "default_true")]
    // #[serde(default)]
    pub ignore_shapes: bool,
    #[serde(default = "default_true")]
    // #[serde(default)]
    pub ignore_root: bool,
    // #[serde(default)]
    #[serde(default)]
    pub id_filter: IdFilter,
    // #[serde(default)]
    #[serde(default)]
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
        if self.ignore_shapes {
            if branch.leaf_group.is_some() {
                return false;
            }
        }
        if self.ignore_root {
            // has no parent == is root
            if branch.parent_index.is_none() {
                return false;
            }
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
    /// only the last iteration of the last id
    Last,
    /// include all ids
    All,
    /// specify exact id
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
