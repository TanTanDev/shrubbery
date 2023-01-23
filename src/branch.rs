use glam::Vec3;

use crate::leaf_classifier::LeafClassifier;

pub struct Branch {
    pub pos: Vec3,
    pub parent_index: Option<usize>,
    pub dir: Vec3,
    pub original_dir: Vec3,
    // how many attractors are pulling this node
    pub attractors_count: i32,
    // how man branchings are from this node
    pub child_count: i32,
    // todo: explain
    pub generation: i32,
}

impl Branch {
    // make a new branch based on this branch calculated growth direciton
    pub fn next(&self, index: usize, branch_len: f32, is_new_generation: bool) -> Self {
        let mut generation = self.generation;
        if is_new_generation {
            generation += 1;
        }
        Self {
            pos: self.pos + self.dir * branch_len,
            parent_index: Some(index),
            dir: self.dir,
            attractors_count: 0,
            original_dir: self.dir,
            child_count: 0,
            generation,
        }
    }

    // no child branches: is leaf
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
