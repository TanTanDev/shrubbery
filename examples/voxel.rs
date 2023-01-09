use shrubbery::shape::BoxShape;
use shrubbery::{
    algorithm_settings::AlgorithmSettings,
    attractor_generator_settings::AttractorGeneratorSettings, prelude::*,
};

use shrubbery::math::*;

use kiss3d::event::{Action, Key, WindowEvent};
use kiss3d::light::Light;
use kiss3d::window::Window;

fn main() {
    let mut window = Window::new("that's a fine shrubbery");
    window.set_light(Light::StickToCamera);

    let mut shrubbery = Shrubbery::new(
        vec3(0., 0., 0.),
        vec3(0., 1., 0.),
        AlgorithmSettings {
            kill_distance: 5.0,
            branch_len: 4.0,
            leaf_attraction_dist: 16.0,
            min_trunk_height: 15.0,
        },
        AttractorGeneratorSettings::default(),
    );
    shrubbery.spawn_leaves(
        vec3(0., 25., 0.),
        BoxShape {
            x: 50.0,
            y: 30.0,
            z: 50.,
        },
    );
    shrubbery.build_trunk();
    while window.render() {
        for mut event in window.events().iter() {
            match event.value {
                WindowEvent::Key(button, Action::Press, _) => {
                    if button == Key::N {
                        shrubbery.grow();
                    }
                }
                _ => {}
            }
        }

        window.set_line_width(6.0);
        for branch in shrubbery.branches.iter() {
            let Some(parent_index) = branch.parent_index else {
                continue;
            };
            let p_pos = shrubbery.branches[parent_index].pos;
            let from = kiss3d::nalgebra::Point3::new(branch.pos.x, branch.pos.y, branch.pos.z);
            let to = kiss3d::nalgebra::Point3::new(p_pos.x, p_pos.y, p_pos.z);
            window.draw_line(&from, &to, &kiss3d::nalgebra::Point3::new(0.4, 0.2, 0.0));
        }

        for attractor in shrubbery.attractors.iter() {
            let pos =
                kiss3d::nalgebra::Point3::new(attractor.pos.x, attractor.pos.y, attractor.pos.z);
            window.draw_point(&pos, &kiss3d::nalgebra::Point3::new(1.0, 1.0, 0.0));
        }
    }
}
