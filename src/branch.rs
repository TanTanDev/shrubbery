use glam::Vec3;

pub struct Branch {
    pub pos: Vec3,
    pub parent_index: Option<usize>,
    pub dir: Vec3,
    pub original_dir: Vec3,
    pub count: i32,
}

impl Branch {
    pub fn next(&self, index: usize, branch_len: f32) -> Self {
        Self {
            pos: self.pos + self.dir * branch_len,
            parent_index: Some(index),
            dir: self.dir,
            count: 0,
            original_dir: self.dir,
        }
    }

    pub fn reset(&mut self) {
        self.count = 0;
        self.dir = self.original_dir;
    }
}
