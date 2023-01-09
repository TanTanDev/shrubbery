pub mod algorithm_settings;
pub mod attractor;
pub mod attractor_generator_settings;
pub mod branch;
pub mod shape;
pub mod shrubbery;
pub mod vec;

pub mod prelude {
    pub use crate::shrubbery::Shrubbery;
}

pub mod math {
    pub use glam::*;
}
