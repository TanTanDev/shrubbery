use core::f32;

use crate::{attractor::Attractor, tree_space_colonization::AttractorSpacing};
use glam::*;
use rand::RngExt;
use rand_chacha::ChaCha8Rng;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Shape {
    Cube(CubeShape),
}

impl Shape {
    pub fn generate(
        &self,
        pos: Vec3,
        attractors: &mut Vec<Attractor>,
        generator_settings: &AttractorSpacing,
        rng: &mut ChaCha8Rng,
    ) {
        match self {
            Shape::Cube(box_shape) => box_shape.generate(pos, attractors, generator_settings, rng),
        }
    }
}

/// x,y,z is total size
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
        generator_settings: &AttractorSpacing,
        rng: &mut ChaCha8Rng,
    ) {
        let attractor_spacing = generator_settings.attractor_spacing.max(0.001);
        let jitter_ratio = generator_settings.jitter_ratio.clamp(0., 1.);

        // calculate iteration per axis
        let [x_l, x_y, x_z] =
            [self.size_x, self.size_y, self.size_z].map(|v| (v / attractor_spacing).ceil() as i32);

        let center_shape_offset = -vec3(self.size_x * 0.5, self.size_y * 0.5, self.size_z * 0.5);

        let scatter_distance = attractor_spacing * 0.5 * jitter_ratio;

        let max_iter = 100000;
        let mut i = 0;
        for x in 0..x_l {
            for y in 0..x_y {
                for z in 0..x_z {
                    i += 1;
                    if i > max_iter {
                        dbg!(x_l, x_y, x_z, attractor_spacing);
                        println!("MAX ITER");
                        return;
                    }
                    let cell_pos = vec3(
                        (x as f32 + 0.5) * attractor_spacing,
                        (y as f32 + 0.5) * attractor_spacing,
                        (z as f32 + 0.5) * attractor_spacing,
                    );
                    let jitter = vec3(
                        rng.random_range(-scatter_distance..scatter_distance),
                        rng.random_range(-scatter_distance..scatter_distance),
                        rng.random_range(-scatter_distance..scatter_distance),
                    );
                    attractors.push(Attractor::new(
                        shape_pos + cell_pos + center_shape_offset + jitter,
                    ));
                }
            }
        }
    }
}
