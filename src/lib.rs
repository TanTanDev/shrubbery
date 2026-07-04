pub mod algorithm_settings;
pub mod attractor;
pub mod attractor_generator_settings;
pub mod branch;
pub mod shape;
pub mod shrubbery;
pub mod vec;
pub mod voxel;
pub use glam;
pub mod math_utils;

#[cfg(feature = "bevy")]
pub mod bevy_fly_cam;
#[cfg(feature = "bevy")]
pub mod bevy_plugin;

pub mod prelude {
    pub use crate::shrubbery::ShrubberyGenerator;

    #[cfg(feature = "bevy")]
    pub use crate::bevy_plugin::ShrubberyPlugin;
}

pub mod math {
    pub use glam::*;
}
