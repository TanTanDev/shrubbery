use kiss3d::camera::ArcBall;
use kiss3d::nalgebra::Point3;
use shrubbery::prelude::*;
use shrubbery::shape::CubeShape;
use shrubbery::tree_space_colonization::{AttractorSpacing, SpaceColonizationSettings};
use shrubbery::voxel::{
    BranchRootSizeIncreaser, BranchSizeSetting, LeafSetting, LeafShape, VoxelId, VoxelizeSettings,
    drop_id, voxelize,
};

use shrubbery::math::*;

use kiss3d::event::{Action, Key, WindowEvent};
use kiss3d::light::Light;
use kiss3d::window::Window;

fn make_tree_generator(
    algo_settings: &SpaceColonizationSettings,
    attractor_generator_settings: &AttractorSpacing,
) -> TreeGeneratorSpaceColonization {
    let mut shrubbery = TreeGeneratorSpaceColonization::new(vec3(0., 0., 0.), vec3(0., 1., 0.), 0);
    // shrubbery.spawn_attractors_from_shape(
    //     vec3(0., 5. + 8.0, 0.),
    //     BoxShape {
    //         size_x: 15.0,
    //         size_y: 10.0,
    //         size_z: 15.,
    //     },
    //     algo_settings,
    //     attractor_generator_settings,
    // );
    shrubbery.build_trunk(&algo_settings);
    shrubbery
}

pub enum GameVoxelId {
    Bark,
    LeafLight,
    LeafDark,
}

#[kiss3d::main]
async fn main() {
    let mut window = Window::new("that's a fine shrubbery");
    window.set_light(Light::StickToCamera);

    let algo_settings = SpaceColonizationSettings {
        kill_distance: 2.0,
        branch_len: 2.0,
        leaf_attraction_dist: 6.0,
        min_trunk_height: 3.0,
        seed: 0,
    };
    let attractor_generator_settings = AttractorSpacing::default();

    let mut shrubbery = make_tree_generator(&algo_settings, &attractor_generator_settings);

    // reference kiss3d box models so we can remove them
    let mut vis_nodes = vec![];

    let settings = VoxelizeSettings {
        branch_size_setting: BranchSizeSetting::Generation {
            sizes: vec![1.5, 1.0, 1.0, 1.0],
        },
        branch_root_size_increaser: Some(BranchRootSizeIncreaser {
            height: 2.0,
            additional_size: 2.0,
        }),
        leaf_settings: LeafSetting::Shape(LeafShape::Sphere { radius: 1.7 }),
        // leaf_settings: LeafSetting::Shape(LeafShape::Sphere { r: 2.7 }),
    };

    let eye = Point3::new(-40f32, 20f32, -5f32);
    let at = Point3::origin();
    let mut camera = ArcBall::new(eye, at);
    while window.render_with_camera(&mut camera).await {
        for event in window.events().iter() {
            match event.value {
                WindowEvent::Key(button, Action::Press, _) => {
                    // flag to indicate rebuilding the voxels
                    // process button input: it's dirty
                    let mut dirty = true;
                    match button {
                        Key::R => {
                            shrubbery =
                                make_tree_generator(&algo_settings, &attractor_generator_settings)
                        }
                        Key::G => shrubbery.post_process_gravity(1.0),
                        Key::T => shrubbery.post_process_spin(3.14 * 0.5),
                        Key::N => shrubbery.grow(&algo_settings),
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
    shrubbery: &mut TreeGeneratorSpaceColonization,
    settings: &VoxelizeSettings,
    vis_nodes: &mut Vec<kiss3d::scene::SceneNode>,
    window: &mut Window,
) {
    let mut gen_voxels = voxelize(shrubbery, settings);
    drop_id(&mut gen_voxels, VoxelId(0), 0.1, 0);

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
        // match voxel {
        //     VoxelType::Air => (),
        //     VoxelType::Branch => {
        //         c.set_color(0.4, 0.2, 0.0);
        //     }
        //     VoxelType::Greenery => {
        //         c.set_color(0.0, 1.0, 0.0);
        //     }
        // };
        match voxel.0 {
            0u32 => (),
            1u32 => {
                c.set_color(0.4, 0.2, 0.0);
            }
            _ => {
                c.set_color(0.0, 1.0, 0.0);
            }
        };
        vis_nodes.push(c);
    }
}
