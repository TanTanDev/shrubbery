use crate::{
    algorithm_settings::AlgorithmSettings, attractor::Attractor,
    attractor_generator_settings::AttractorGeneratorSettings, branch::Branch, shape::Shape,
    vec::Vector,
};

use glam::{vec3, Vec3};

pub struct Shrubbery {
    pub branches: Vec<Branch>,
    pub attractors: Vec<Attractor>,
    pub settings: AlgorithmSettings,
    pub generator_settings: AttractorGeneratorSettings,
}

impl Shrubbery {
    pub fn new(
        root_pos: Vec3,
        initial_dir: Vec3,
        settings: AlgorithmSettings,
        generator_settings: AttractorGeneratorSettings,
    ) -> Self {
        let mut branches = Vec::new();
        let root = Branch {
            pos: root_pos,
            parent_index: None,
            dir: initial_dir,
            count: 0,
            original_dir: initial_dir,
        };
        branches.push(root);
        Self {
            branches,
            attractors: Vec::new(),
            settings,
            generator_settings,
        }
    }

    pub fn build_trunk(&mut self) {
        let mut root_end_pos = self.branches[0].pos;
        let dir = self.branches[0].dir;
        let mut consumed_height = 0.;
        // the first root will be as long as it needs to be until it starts gaining attractions
        let max_iterations = 1000;
        'a: for _ in 0..max_iterations {
            consumed_height += self.settings.branch_len;
            root_end_pos += self.settings.branch_len * dir;
            // root_end_pos = dir;
            // if consumed_height > self.settings.trunk_height {
            //     break;
            // }
            for leaf in self.attractors.iter() {
                let dist = root_end_pos.distance(leaf.pos);
                if dist < self.settings.leaf_attraction_dist {
                    break 'a;
                }
            }
        }

        // keep adding branches upwards until we reach the trunk_height
        let new_branch = self.branches[0].next(0, consumed_height);
        self.branches.push(new_branch);
        while consumed_height < self.settings.min_trunk_height {
            consumed_height += self.settings.branch_len;
            let last_index = self.branches.len() - 1;
            self.branches
                .push(self.branches[last_index].next(last_index, self.settings.branch_len))
        }
    }

    pub fn grow(&mut self) {
        // find closest
        // pull branch directions towards leafs
        for leaf in self.attractors.iter_mut() {
            let mut closest_branch: Option<usize> = None;
            let mut closest_dist = 999999999.;
            for (branch_index, branch) in self.branches.iter_mut().enumerate() {
                let dist = leaf.pos.distance(branch.pos);
                // is this branch to close to the leaf, discard it
                if dist < self.settings.kill_distance {
                    leaf.reached = true;
                    closest_branch = None;
                    break;
                }
                // to far away to branch to
                if dist > self.settings.leaf_attraction_dist {
                    continue;
                }
                if dist < closest_dist {
                    closest_branch = Some(branch_index);
                    closest_dist = dist;
                }
            }
            // pull closest branch towards us
            if let Some(closest_branch_index) = closest_branch {
                let closest_branch_pos = self.branches[closest_branch_index].pos;
                let new_branch_dir = leaf.pos - closest_branch_pos;
                let new_branch_dir = new_branch_dir.normalize();
                self.branches[closest_branch_index].dir += new_branch_dir;
                self.branches[closest_branch_index].count += 1;
            }
        }
        // remove reached leaves
        self.attractors.retain(|leaf| !leaf.reached);

        let mut to_add = vec![];
        for (branch_index, branch) in self
            .branches
            .iter_mut()
            .enumerate()
            .filter(|(_, branch)| branch.count > 0)
        {
            branch.dir = branch.dir.normalize();
            to_add.push(branch.next(branch_index, self.settings.branch_len));
            branch.reset();
        }
        self.branches.extend(to_add);
    }

    pub fn spawn_leaves<TShape>(&mut self, pos: Vec3, shape: TShape)
    where
        TShape: Shape,
    {
        shape.generate(
            pos,
            &mut self.attractors,
            &self.settings,
            &self.generator_settings,
        );
    }
}
