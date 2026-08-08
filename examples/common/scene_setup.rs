//! just some helpers to set up the scene
//! lights / camera. same setup used for all bevy examples

#[path = "bevy_fly_cam.rs"]
mod bevy_fly_cam;
use bevy_fly_cam::FlyCameraPlugin;

use std::f32::consts::PI;

use bevy::{
    color::palettes::css::{WHITE, YELLOW},
    light::CascadeShadowConfigBuilder,
    prelude::*,
};

use crate::scene_setup::bevy_fly_cam::FlyCamera;

// use crate::bevy_fly_cam::FlyCamera;

/// All bevy examples use the same scene setup camera/lights etc...
pub struct SceneSetupPlugin;

impl Plugin for SceneSetupPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FlyCameraPlugin)
            .add_systems(Startup, setup)
            .add_systems(Update, move_light);
    }
}

#[derive(Component)]
pub struct RotatingLightTag;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-11.5, 54.5, 29.0).looking_at(Vec3::ZERO, Vec3::Y),
        FlyCamera::default(),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Circle::new(10.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    commands.insert_resource(GlobalAmbientLight {
        color: WHITE.into(),
        brightness: 400.0,
        affects_lightmapped_meshes: true,
    });

    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 4.0,
            maximum_distance: 10.0,
            ..default()
        }
        .build(),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        PointLight {
            shadow_maps_enabled: true,
            color: YELLOW.into(),
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
        RotatingLightTag,
    ));
}

fn move_light(mut query: Query<&mut Transform, With<RotatingLightTag>>, time: Res<Time>) {
    let speed = 5.0;
    let rotation = time.elapsed_secs() * speed;
    let quat = Quat::from_rotation_y(rotation);
    let distance = 14.0;
    let boom = Vec3::X * distance;
    for mut transform in query.iter_mut() {
        transform.translation = quat * boom;
        transform.translation.y = 8.0;
    }
}
