use crate::{
    algorithm_settings::AlgorithmSettings, attractor::Attractor,
    attractor_generator_settings::AttractorGeneratorSettings, branch::Branch, shape::Shape,
    vec::Vector, voxel::BranchSizeSetting,
};

use glam::{ivec3, vec2, vec3, IVec3, Vec2, Vec3};

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

    pub fn post_process_gravity(&mut self, max_gravity: f32) {
        let plane_half_size = self.get_plane_half_size();
        for branch in self.branches.iter_mut() {
            // branch.
            let branch_plane = vec2(branch.pos.x, branch.pos.z);
            let root = Vec2::ZERO;
            let dist_to_root = branch_plane.distance(root);

            let weight = (dist_to_root / plane_half_size);
            branch.pos.y -= weight;
            // let dist_from_root = vec3(b)
        }
    }

    pub fn post_process_spin(&mut self, max_spin: f32) {
        let plane_half_size = self.get_plane_half_size();
        for branch in self.branches.iter_mut() {
            // branch.
            let branch_xz = vec2(branch.pos.x, branch.pos.z);
            let root = Vec2::ZERO;
            let dist_to_root = branch_xz.distance(root);
            let weight = dist_to_root / plane_half_size;

            let y_weight = (branch.pos.y * 0.3).cos() * 0.5 + 0.5;

            let new_xz = rotate_point(branch_xz, max_spin * weight * y_weight);
            branch.pos.x = new_xz.x;
            branch.pos.z = new_xz.y;

            // branch.pos.y -= weight;
            // let dist_from_root = vec3(b)
        }
    }

    pub fn get_plane_half_size(&self) -> f32 {
        let size = self.get_bounding_size();
        (size.x.max(size.z) as f32 * 0.5).ceil()
    }

    pub fn get_bounding_size(&self) -> IVec3 {
        let (min_bounds, max_bounds) = self.get_bounds();
        max_bounds - min_bounds
    }

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

    pub fn update_bound(min_bounds: &mut Vec3, max_bounds: &mut Vec3, branch_pos: Vec3) {
        if branch_pos.x < min_bounds.x {
            min_bounds.x = branch_pos.x;
        }
        if branch_pos.x > max_bounds.x {
            max_bounds.x = branch_pos.x;
        }
        if branch_pos.y < min_bounds.y {
            min_bounds.y = branch_pos.y;
        }
        if branch_pos.y > max_bounds.y {
            max_bounds.y = branch_pos.y;
        }
        if branch_pos.z < min_bounds.z {
            min_bounds.z = branch_pos.z;
        }
        if branch_pos.z > max_bounds.z {
            max_bounds.z = branch_pos.z;
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
                self.branches[closest_branch_index].attractors_count += 1;
            }
        }
        // remove reached leaves
        self.attractors.retain(|leaf| !leaf.reached);

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

    pub fn distance_to_branch(&self, pos: Vec3) -> (f32, usize) {
        let mut closest = f32::MAX;
        let mut index = 0;
        for (i, branch) in self.branches.iter().enumerate() {
            let Some(parent_index) = branch.parent_index else {
                continue;
            };

            let d = dist_to_line(pos, self.branches[parent_index].pos, branch.pos);
            // let d = dist_to_line(pos, branch.pos, self.branches[parent_index].pos);
            if d < closest {
                closest = d;
                index = i;
            }
            // closest = closest.min(d);
        }
        (closest, index)
        // self.branches
        //     .iter()
        //     .map(|branch| branch.pos.distance(pos))
        //     .reduce(f32::min)
        //     .unwrap_or(0f32)
    }
}

// fn dist_to_line(pos: Vec3, line_start: Vec3, line_end: Vec3) -> f32 {
//     // x2 = line_end
//     // x1 = line_start
//     // x0 = pos

//     // Vec3::cross(line_end - line_start, line_start - pos).length() / (line_end - line_start).length()
//     ((line_end - line_start) * (line_start - pos)).length() / (line_end - line_start).length()
// }

// fn dist_to_line(pos: Vec3, line_start: Vec3, line_end: Vec3) -> f32 {
//     // x2 = line_end
//     // x1 = line_start
//     // x0 = pos
//     let ab = line_end - line_start;
//     // let ac = line_start - pos;
//     let ac = pos - line_start;
//     let area = Vec3::cross(ab, ac).length();
//     let cd = area / ab.length();
//     cd
// }

fn rotate_point(pos: Vec2, radians: f32) -> Vec2 {
    let (cos_theta, sin_theta) = (radians.cos(), radians.sin());
    let out = vec2(
        cos_theta * pos.x - sin_theta * pos.y,
        sin_theta * pos.x + cos_theta * pos.y,
    );
    out
}

fn dist_to_line(pos: Vec3, line_start: Vec3, line_end: Vec3) -> f32 {
    // x2 = line_end
    // x1 = line_start
    // x0 = pos
    let ab = line_end - line_start;
    let ac = pos - line_start;
    if ac.dot(ab) <= 0.0 {
        return ac.length();
    }
    let bv = pos - line_end;
    if bv.dot(ab) >= 0.0 {
        return bv.length();
    }
    ab.cross(ac).length() / ab.length()
    // let area = Vec3::cross(ab, ac).length();
    // let cd = area / ab.length();
    // cd
}
