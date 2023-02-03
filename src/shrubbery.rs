use crate::{
    algorithm_settings::AlgorithmSettings,
    attractor::Attractor,
    attractor_generator_settings::AttractorGeneratorSettings,
    branch::Branch,
    math_utils::{dist_to_line, rotate_point},
    shape::Shape,
};

use glam::{ivec3, vec2, IVec3, Vec2, Vec3};

/// a tree/bush, pressentation that you can grow and modify through post processing effect
pub struct Shrubbery {
    pub branches: Vec<Branch>,
    pub attractors: Vec<Attractor>,
    pub settings: AlgorithmSettings,
    pub generator_settings: AttractorGeneratorSettings,
    pub min_bounds: Vec3,
    pub max_bounds: Vec3,
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
            attractors_count: 0,
            original_dir: initial_dir,
            child_count: 0,
            generation: 0,
        };
        branches.push(root);
        Self {
            branches,
            attractors: Vec::new(),
            settings,
            generator_settings,
            min_bounds: Vec3::splat(0f32),
            max_bounds: Vec3::splat(0f32),
        }
    }

    /// return the half size bounds, imagine a quad on the xz axis
    pub fn get_plane_half_size(&self) -> f32 {
        let size = self.get_bounding_size();
        (size.x.max(size.z) as f32 * 0.5).ceil()
    }

    /// rturns size of the bounding box
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

    // expand bounding if branch_pos is outside
    pub fn update_bound(min_bounds: &mut Vec3, max_bounds: &mut Vec3, branch_pos: Vec3) {
        min_bounds.x = min_bounds.x.min(branch_pos.x);
        min_bounds.y = min_bounds.y.min(branch_pos.y);
        min_bounds.z = min_bounds.z.min(branch_pos.z);
        max_bounds.x = max_bounds.x.max(branch_pos.x);
        max_bounds.y = max_bounds.y.max(branch_pos.y);
        max_bounds.z = max_bounds.z.max(branch_pos.z);
    }

    /// spawn initial branches based on settings.
    pub fn build_trunk(&mut self) {
        let mut root_end_pos = self.branches[0].pos;
        let dir = self.branches[0].dir;
        let mut consumed_height = 0.;
        // the first root will be as long as it needs to be until it starts gaining attractions
        let max_iterations = 1000;
        'a: for _ in 0..max_iterations {
            consumed_height += self.settings.branch_len;
            root_end_pos += self.settings.branch_len * dir;
            for leaf in self.attractors.iter() {
                let dist = root_end_pos.distance(leaf.pos);
                if dist < self.settings.leaf_attraction_dist {
                    break 'a;
                }
            }
        }

        self.branches[0].child_count += 1;
        let new_branch = self.branches[0].next(0, consumed_height, false);
        Self::update_bound(&mut self.min_bounds, &mut self.max_bounds, new_branch.pos);
        self.branches.push(new_branch);

        // keep adding branches upwards until we reach the trunk_height
        while consumed_height < self.settings.min_trunk_height {
            consumed_height += self.settings.branch_len;
            let last_index = self.branches.len() - 1;
            let new_branch =
                self.branches[last_index].next(last_index, self.settings.branch_len, false);
            self.branches[last_index].child_count += 1;
            Self::update_bound(&mut self.min_bounds, &mut self.max_bounds, new_branch.pos);
            self.branches.push(new_branch)
        }
    }

    /// using space colonization algorithm, spawn new branches
    pub fn grow(&mut self) {
        for leaf in self.attractors.iter_mut() {
            let mut closest_branch: Option<usize> = None;
            let mut closest_dist = 999999.;
            // find shortest signed distance of all branches
            for (branch_index, branch) in self.branches.iter_mut().enumerate() {
                let dist = leaf.pos.distance(branch.pos);
                // is this branch to close to the leaf, discard it
                if dist < self.settings.kill_distance {
                    leaf.reached = true;
                    closest_branch = None;
                    break;
                }
                // to far away to be attracted towards
                if dist > self.settings.leaf_attraction_dist {
                    continue;
                }
                // record closest branch
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
                self.branches[closest_branch_index].attractors_count += 1;
            }
        }
        // remove reached leaves
        self.attractors.retain(|leaf| !leaf.reached);

        // spawn new branches using previous calculations
        let mut to_add = vec![];
        for (branch_index, branch) in self
            .branches
            .iter_mut()
            .enumerate()
            .filter(|(_, branch)| branch.attractors_count > 0)
        {
            branch.dir = branch.dir.normalize();
            let new_branch = branch.next(branch_index, self.settings.branch_len, true);
            branch.child_count += 1;
            Self::update_bound(&mut self.min_bounds, &mut self.max_bounds, new_branch.pos);
            to_add.push(new_branch);
            branch.reset();
        }
        self.branches.extend(to_add);
    }

    /// spawn particles inside provided shape, based on settings
    pub fn spawn_attractors_from_shape<TShape>(&mut self, pos: Vec3, shape: TShape)
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

    /// reduce y position of branches, weighted by dist to 0,0 xz.
    pub fn post_process_gravity(&mut self, gravity: f32) {
        let plane_half_size = self.get_plane_half_size();
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
        let plane_half_size = self.get_plane_half_size();
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
