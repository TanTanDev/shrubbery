//! Procedural voxel vegetation generation.
//!
//! Build a [`ShrubberyGenerator`] from a [`ShrubberySettings`]
//! then call [`ShrubberyGenerator::voxelize`] to get the voxel grid.
//! See the `examples/` for usage with Bevy.
//!
//! ```
//! use shrubbery::prelude::*;
//!
//! let settings: ShrubberySettings = Default::default();
//! let mut generator = ShrubberyGenerator::generate(42, &settings);
//! let voxels = generator.voxelize();
//! ```

pub mod attractor;
pub mod branch;
pub mod math_utils;
pub mod shape;
pub mod shrubbery;
pub mod voxel;

pub use glam;

#[cfg(feature = "bevy")]
pub mod bevy_debug_draw;
#[cfg(feature = "bevy")]
pub mod bevy_fly_cam;
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
