#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::branch::Branch;

/// Decides what branches to operate on based upon different filtering criteria
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct Filter {
    /// Skip branches that already have a leaf group assigned.
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub ignore_shapes: bool,
    /// Skip root branches (those with no parent).
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub ignore_root: bool,
    pub id: IdFilter,
    pub iteration: IterationFilter,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            ignore_shapes: true,
            ignore_root: true,
            id: IdFilter::default(),
            iteration: IterationFilter::default(),
        }
    }
}

impl Filter {
    pub fn should_include_branch(&self, branch: &Branch, last_id: u32) -> bool {
        if self.ignore_shapes && branch.leaf_group.is_some() {
            return false;
        }
        if self.ignore_root && branch.parent_index.is_none() {
            return false;
        }
        if !self.id.is_id_included(branch, last_id) {
            return false;
        }
        if !self
            .iteration
            .is_iteration_included(branch.iteration, branch.iteration_total)
        {
            return false;
        }
        true
    }
}

/// Decides what branches to operate on based upon filtering the iteration value
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

/// specify when to ignore/include a branch depending on the id value
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

#[cfg(feature = "serde")]
fn default_true() -> bool {
    true
}
