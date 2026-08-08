//! A position in space used for space-colonization
use glam::Vec3;

/// A position in space used for space-colonization
pub struct Attractor {
    pub pos: Vec3,
    pub reached: bool,
}

impl Attractor {
    pub fn new(pos: Vec3) -> Self {
        Self {
            pos,
            reached: false,
        }
    }
}
