use shrubbery::shape::BoxShape;
use shrubbery::voxel::{
    drop_leaves, voxelize, BranchRootSizeIncreaser, BranchSizeSetting, LeafSetting, LeafShape,
    VoxelType, VoxelizeSettings,
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
            kill_distance: 2.0,
            branch_len: 2.0,
            leaf_attraction_dist: 6.0,
            min_trunk_height: 3.0,
        },
        AttractorGeneratorSettings::default(),
    );
    shrubbery.spawn_attractors_from_shape(
        vec3(0., 5. + 8.0, 0.),
        BoxShape {
            x: 15.0,
            y: 10.0,
            z: 15.,
        },
    );
    shrubbery.build_trunk();
    shrubbery
}

fn main() {
    let mut window = Window::new("that's a fine shrubbery");
    window.set_light(Light::StickToCamera);
    let mut shrubbery = make_shrubbery();

    // reference kiss3d box models so we can remove them
    let mut vis_nodes = vec![];

    let settings = VoxelizeSettings {
        branch_size_setting: BranchSizeSetting::Generation {
            distances: vec![1.5, 1.0, 1.0, 1.0],
        },
        branch_root_size_increaser: Some(BranchRootSizeIncreaser {
            height: 2.0,
            additional_size: 2.0,
        }),
        leaf_settings: LeafSetting::Shape(LeafShape::Sphere { r: 2.7 }),
    };

    while window.render() {
        for event in window.events().iter() {
            match event.value {
                WindowEvent::Key(button, Action::Press, _) => {
                    // flag to indicate rebuilding the voxels
                    // process button input: it's dirty
                    let mut dirty = true;
                    match button {
                        Key::R => shrubbery = make_shrubbery(),
                        Key::G => shrubbery.post_process_gravity(1.0),
                        Key::T => shrubbery.post_process_spin(3.14 * 0.5),
                        Key::N => shrubbery.grow(),
                        _ => dirty = false,
                    }
                    if dirty {
                        build_voxels(&mut shrubbery, &settings, &mut vis_nodes, &mut window);
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

            let mut is_leaf = false;
            if let LeafSetting::BranchIsLeaf(classifier) = &settings.leaf_settings {
                is_leaf = branch.is_leaf(classifier);
            }
            let color = if is_leaf {
                kiss3d::nalgebra::Point3::new(0.0, 1.0, 0.0)
            } else {
                kiss3d::nalgebra::Point3::new(0.4, 0.2, 0.0)
            };
            window.draw_line(&from, &to, &color);
        }

        for attractor in shrubbery.attractors.iter() {
            let pos =
                kiss3d::nalgebra::Point3::new(attractor.pos.x, attractor.pos.y, attractor.pos.z);
            window.set_point_size(6.0);
            window.draw_point(&pos, &kiss3d::nalgebra::Point3::new(1.0, 1.0, 0.0));
        }
    }
}

fn build_voxels(
    shrubbery: &mut Shrubbery,
    settings: &VoxelizeSettings,
    vis_nodes: &mut Vec<kiss3d::scene::SceneNode>,
    window: &mut Window,
) {
    let mut gen_voxels = voxelize(shrubbery, settings);
    drop_leaves(&mut gen_voxels, 0.1);

    vis_nodes
        .iter_mut()
        .for_each(|mut n| window.remove_node(&mut n));

    for (pos, voxel) in gen_voxels.iter() {
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
