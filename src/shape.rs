use crate::{
    algorithm_settings::AlgorithmSettings, attractor::Attractor,
    attractor_generator_settings::AttractorGeneratorSettings,
};
use glam::*;
use rand::Rng;

/// A shape to spawn attractors inside
pub trait Shape {
    fn generate(
        &self,
        pos: Vec3,
        attractors: &mut Vec<Attractor>,
        algorithm_settings: &AlgorithmSettings,
        generator_settings: &AttractorGeneratorSettings,
    );
}

/// x,y,z is total size
pub struct BoxShape {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Shape for BoxShape {
    fn generate(
        &self,
        pos: Vec3,
        attractors: &mut Vec<Attractor>,
        algorithm_settings: &AlgorithmSettings,
        generator_settings: &AttractorGeneratorSettings,
    ) {
        let mut ideal_spacing =
            algorithm_settings.leaf_attraction_dist * 0.5 - algorithm_settings.kill_distance * 0.5;
        ideal_spacing *= 1.0 / generator_settings.density;

        let x_l = (self.x / ideal_spacing) as i32;
        let x_y = (self.y / ideal_spacing) as i32;
        let x_z = (self.z / ideal_spacing) as i32;

        let center_shape_offset = -vec3(self.x * 0.5, self.y * 0.5, self.z * 0.5);
        let start_pos = vec3(
            ideal_spacing * 0.5,
            ideal_spacing * 0.5,
            ideal_spacing * 0.5,
        );

        let scatter_distance = ideal_spacing * 0.5;
        let mut rng = rand::thread_rng();

        for x in 0..x_l {
            for y in 0..x_y {
                for z in 0..x_z {
                    let jitter = vec3(
                        x as f32 * ideal_spacing
                            + rng.gen_range(-scatter_distance..scatter_distance),
                        y as f32 * ideal_spacing
                            + rng.gen_range(-scatter_distance..scatter_distance),
                        z as f32 * ideal_spacing
                            + rng.gen_range(-scatter_distance..scatter_distance),
                    );
                    attractors.push(Attractor::new(
                        pos + start_pos + center_shape_offset + jitter,
                    ));
                }
            }
        }
    }
}
