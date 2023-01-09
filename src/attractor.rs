use glam::Vec3;

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
