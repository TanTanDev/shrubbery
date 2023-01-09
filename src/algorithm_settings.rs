#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AlgorithmSettings {
    pub kill_distance: f32,
    pub branch_len: f32,
    pub leaf_attraction_dist: f32,
    pub min_trunk_height: f32,
}

impl Default for AlgorithmSettings {
    fn default() -> Self {
        Self {
            kill_distance: 0.3,
            branch_len: 0.3,
            leaf_attraction_dist: 5.,
            min_trunk_height: 1.,
        }
    }
}
