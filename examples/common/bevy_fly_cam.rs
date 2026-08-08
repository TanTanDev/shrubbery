//! Minimal fly camera plugin
//! Only exported for example code
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

pub struct FlyCameraPlugin;

impl Plugin for FlyCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FlyCameraSettings>()
            .init_resource::<FlyCameraKeybindings>()
            .add_systems(Update, (sync_look_from_transform, look, movement).chain());
    }
}

#[derive(Resource)]
pub struct FlyCameraSettings {
    pub mouse_sensitivity: f32,
    pub move_speed: f32,
}

impl Default for FlyCameraSettings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 0.0015,
            move_speed: 64.0,
        }
    }
}

#[derive(Resource)]
pub struct FlyCameraKeybindings {
    pub forward: KeyCode,
    pub backward: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
    pub up: KeyCode,
    pub down: KeyCode,
    pub sprint: KeyCode,
}

impl Default for FlyCameraKeybindings {
    fn default() -> Self {
        Self {
            forward: KeyCode::KeyW,
            backward: KeyCode::KeyS,
            left: KeyCode::KeyA,
            right: KeyCode::KeyD,
            up: KeyCode::Space,
            down: KeyCode::ShiftLeft,
            sprint: KeyCode::ControlLeft,
        }
    }
}

/// Marker + look-state for a fly camera entity.
#[derive(Component, Default)]
pub struct FlyCamera {
    pub yaw: f32,
    pub pitch: f32,
}

fn sync_look_from_transform(mut query: Query<(&Transform, &mut FlyCamera), Added<FlyCamera>>) {
    for (transform, mut cam) in &mut query {
        let forward = transform.forward();
        cam.yaw = (-forward.x).atan2(-forward.z);
        cam.pitch = forward.y.asin();
    }
}

fn look(
    settings: Res<FlyCameraSettings>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut query: Query<(&mut Transform, &mut FlyCamera)>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        cursor_options.grab_mode = CursorGrabMode::Locked;
        cursor_options.visible = false;
    }
    if keys.just_pressed(KeyCode::Escape) {
        cursor_options.grab_mode = CursorGrabMode::None;
        cursor_options.visible = true;
    }

    if cursor_options.grab_mode == CursorGrabMode::None {
        mouse_motion.clear();
        return;
    }

    let mut delta = Vec2::ZERO;
    for ev in mouse_motion.read() {
        delta += ev.delta;
    }
    if delta == Vec2::ZERO {
        return;
    }

    for (mut transform, mut cam) in &mut query {
        cam.yaw -= delta.x * settings.mouse_sensitivity;
        cam.pitch = (cam.pitch - delta.y * settings.mouse_sensitivity)
            .clamp(-89f32.to_radians(), 89f32.to_radians());

        transform.rotation =
            Quat::from_axis_angle(Vec3::Y, cam.yaw) * Quat::from_axis_angle(Vec3::X, cam.pitch);
    }
}

fn movement(
    time: Res<Time>,
    settings: Res<FlyCameraSettings>,
    keybindings: Res<FlyCameraKeybindings>,
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<FlyCamera>>,
) {
    let mut dir = Vec3::ZERO;
    for mut transform in &mut query {
        if keys.pressed(keybindings.forward) {
            dir += *transform.forward();
        }
        if keys.pressed(keybindings.backward) {
            dir += *transform.back();
        }
        if keys.pressed(keybindings.left) {
            dir += *transform.left();
        }
        if keys.pressed(keybindings.right) {
            dir += *transform.right();
        }
        if keys.pressed(keybindings.up) {
            dir += Vec3::Y;
        }
        if keys.pressed(keybindings.down) {
            dir -= Vec3::Y;
        }

        if dir != Vec3::ZERO {
            dir = dir.normalize();
            let speed = if keys.pressed(keybindings.sprint) {
                settings.move_speed * 3.0
            } else {
                settings.move_speed
            };
            transform.translation += dir * speed * time.delta_secs();
        }
    }
}
