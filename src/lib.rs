//! Procedural voxel shape generation library.

pub mod attractor;
pub mod branch;
pub mod shape;
pub mod shrubbery;
pub mod voxel;

pub use glam;

#[cfg(feature = "bevy")]
pub mod bevy_debug_draw;
#[cfg(feature = "bevy")]
pub mod bevy_plugin;

pub mod prelude {
    pub use crate::shrubbery::{ShrubberyGenerator, ShrubberySettings, ShrubberyStep};
    pub use crate::voxel::{VoxelDefinitions, VoxelId, VoxelMapping};

    #[cfg(feature = "bevy")]
    pub use crate::bevy_debug_draw::{
        ShrubberyDebugConfig, ShrubberyDebugDraw, ShrubberyDebugDrawPlugin,
        ShrubberyDebugGizmoGroup,
    };
    #[cfg(feature = "bevy")]
    pub use crate::bevy_plugin::ShrubberyPlugin;
}
