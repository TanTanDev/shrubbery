use shrubbery::shape::BoxShape;
use shrubbery::voxel::{
    voxelize, BranchRootSizeIncreaser, BranchSizeSetting, LeafGenerator, VoxelType,
    VoxelizeSettings,
};
use shrubbery::{
    algorithm_settings::AlgorithmSettings,
    attractor_generator_settings::AttractorGeneratorSettings, prelude::*,
};

use shrubbery::math::*;

use kiss3d::event::{Action, Key, WindowEvent};
use kiss3d::light::Light;
use kiss3d::window::Window;

fn make_shrubbery() -> Shrubbery {
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
    shrubbery
}

fn main() {
    let mut window = Window::new("that's a fine shrubbery");
    window.set_light(Light::StickToCamera);
    let mut shrubbery = make_shrubbery();

    let mut voxels = vec![];

    let mut vis_nodes = vec![];
    while window.render() {
        for event in window.events().iter() {
            match event.value {
                WindowEvent::Key(button, Action::Press, _) => {
                    if button == Key::R {
                        shrubbery = make_shrubbery();
                        vis_nodes
                            .iter_mut()
                            .for_each(|mut n| window.remove_node(&mut n));
                    }
                    if button == Key::N {
                        shrubbery.grow();
                        let settings = VoxelizeSettings {
                            leaf_generator: Some(LeafGenerator::Sphere { r: 5.0 }),
                            // branch_size_setting: BranchSizeSetting::Value { distance: 1.0 },
                            branch_size_setting: BranchSizeSetting::Generation {
                                distances: vec![3.0, 3.0, 2.0, 1.0],
                            },
                            branch_root_size_increaser: Some(BranchRootSizeIncreaser {
                                height: 10.0,
                                additional_size: 3.0,
                            }),
                        };
                        let gen_voxels = voxelize(&mut shrubbery, settings);
                        vis_nodes
                            .iter_mut()
                            .for_each(|mut n| window.remove_node(&mut n));
                        voxels = gen_voxels;
                        // let neg_offset = uniform as f32 * -0.5;
                        for ((pos, voxel)) in voxels.iter() {
                            let c_s = 1.0;
                            let mut c = window.add_cube(c_s, c_s, c_s);
                            c.append_translation(&kiss3d::nalgebra::Translation3::new(
                                pos.x as f32 + 40.0,
                                pos.y as f32,
                                pos.z as f32,
                            ));
                            match voxel {
                                VoxelType::Air => (),
                                VoxelType::Branch => {
                                    c.set_color(0.4, 0.2, 0.0);
                                }
                                VoxelType::Greenery => {
                                    c.set_color(0.0, 1.0, 0.0);
                                }
                            };
                            vis_nodes.push(c);
                        }
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

        // for attractor in shrubbery.attractors.iter() {
        //     let pos =
        //         kiss3d::nalgebra::Point3::new(attractor.pos.x, attractor.pos.y, attractor.pos.z);
        //     window.draw_point(&pos, &kiss3d::nalgebra::Point3::new(1.0, 1.0, 0.0));
        // }
    }
}
