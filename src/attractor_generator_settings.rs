pub struct AttractorGeneratorSettings {
    pub max_leaves: Option<i32>,
    pub min_leaves: Option<i32>,
    // a value of 1: will spawn enough leaves to expand the whole area
    // 1.0 is minimum recommended value, higher values will yield potentially more branching
    // but higher values also generates more leaves, tolling performance
    pub density: f32,
}

impl Default for AttractorGeneratorSettings {
    fn default() -> Self {
        Self {
            max_leaves: Some(500),
            min_leaves: Some(30),
            density: 1.0,
        }
    }
}
