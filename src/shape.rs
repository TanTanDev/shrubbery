//! attractor spawning shapes
use glam::*;
use rand::RngExt;
use rand_chacha::ChaCha8Rng;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{attractor::Attractor, shrubbery::AttractorSpacing};

/// The shape that is used to spawn Attractors
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AttractorShape {
    Cube(CubeShape),
}

impl AttractorShape {
    pub fn generate(
        &self,
        pos: Vec3,
        attractors: &mut Vec<Attractor>,
        settings: &AttractorSpacing,
        rng: &mut ChaCha8Rng,
    ) {
        match self {
            AttractorShape::Cube(cube) => cube.generate(pos, attractors, settings, rng),
        }
    }
}

/// Shape for spawning Attractors. `size_(x/y/z)` specify full length.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CubeShape {
    pub size_x: f32,
    pub size_y: f32,
    pub size_z: f32,
}

impl CubeShape {
    fn generate(
        &self,
        shape_pos: Vec3,
        attractors: &mut Vec<Attractor>,
        settings: &AttractorSpacing,
        rng: &mut ChaCha8Rng,
    ) {
        let spacing = settings.attractor_spacing.max(0.001);
        let jitter_ratio = settings.jitter_ratio.clamp(0.0, 1.0);
        let scatter = spacing * 0.5 * jitter_ratio;
        let center_offset = -vec3(self.size_x, self.size_y, self.size_z) * 0.5;

        let [nx, ny, nz] =
            [self.size_x, self.size_y, self.size_z].map(|v| (v / spacing).ceil() as i32);

        for x in 0..nx {
            for y in 0..ny {
                for z in 0..nz {
                    let cell = vec3(
                        (x as f32 + 0.5) * spacing,
                        (y as f32 + 0.5) * spacing,
                        (z as f32 + 0.5) * spacing,
                    );
                    let jitter = vec3(
                        rng.random_range(-scatter..=scatter),
                        rng.random_range(-scatter..=scatter),
                        rng.random_range(-scatter..=scatter),
                    );
                    attractors.push(Attractor::new(shape_pos + cell + center_offset + jitter));
                }
            }
        }
    }
}
