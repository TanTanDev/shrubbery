use glam::Vec3;

use crate::{leaf_classifier::LeafClassifier, tree_space_colonization::TrunkGrowthDirection};

#[derive(Debug)]
pub struct Branch {
    pub pos: Vec3,
    pub parent_index: Option<usize>,
    pub dir: Vec3,
    /// todo: remove? might not be needed
    pub original_dir: Vec3,
    /// how many attractors are pulling this node
    pub attractors_count: i32,
    /// how many children branches this branch has
    pub child_count: i32,
    /// what iteration this branch was made
    pub generation: i32,
    /// Index into the generator's `leaf_groups` vec, set by a `SpawnLeaves`
    /// build step.  `None` means no leaf decoration has been assigned yet.
    pub leaf_group: Option<usize>,
}

impl Branch {
    /// make a new branch based on this branch calculated growth direciton
    pub fn next(
        &self,
        index: usize,
        branch_len: f32,
        is_new_generation: bool,
        trunk_growth_dir: &TrunkGrowthDirection,
    ) -> Self {
        let mut generation = self.generation;
        if is_new_generation {
            generation += 1;
        }

        let dir = match trunk_growth_dir {
            TrunkGrowthDirection::Normal => self.dir,
            TrunkGrowthDirection::GravityLean { strength } => {
                (self.dir + Vec3::NEG_Y * *strength).normalize()
                // self.dir * branch_len
            }
        };

        Self {
            pos: self.pos + dir * branch_len,
            parent_index: Some(index),
            dir: dir,
            attractors_count: 0,
            original_dir: dir,
            child_count: 0,
            generation,
            leaf_group: None,
        }
    }

    /// no child branches: is leaf
    pub fn is_leaf(&self, classifier: &LeafClassifier) -> bool {
        match classifier {
            LeafClassifier::LastBranch => self.child_count == 0,
            LeafClassifier::NonRootBranch => self.generation != 0,
        }
    }

    pub fn reset(&mut self) {
        self.attractors_count = 0;
        self.dir = self.original_dir;
    }
}
