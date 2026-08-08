//! the core building block for all shapes
//! Branch, could be also be though of as a "line"
use glam::Vec3;

use crate::shrubbery::BranchGrowthDirection;

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
            // should technically never be set here, but self.dir, shoul be set from attractors
            BranchGrowthDirection::Attractor { .. } => self.dir,
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
