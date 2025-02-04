use crate::game::{CAMERA_ROTATION, CAMERA_TRANSLATION};
use bevy::{
    input::common_conditions::{input_just_pressed, input_pressed},
    prelude::*,
};
use std::f32::consts::FRAC_PI_8;

use super::SANDBOX_CTX_STATE;

const KEY_FORWARD: KeyCode = KeyCode::KeyW;
const KEY_BACK: KeyCode = KeyCode::KeyS;
const KEY_LEFT: KeyCode = KeyCode::KeyA;
const KEY_RIGHT: KeyCode = KeyCode::KeyD;
const KEY_ZOOM_IN: KeyCode = KeyCode::ShiftLeft;
const KEY_ZOOM_OUT: KeyCode = KeyCode::Space;
const KEY_ROTATE_FORWARD: KeyCode = KeyCode::KeyR;
const KEY_ROTATE_BACK: KeyCode = KeyCode::KeyF;
const KEY_RESET_XF: KeyCode = KeyCode::KeyQ;

const KEY_PRINT_XF: KeyCode = KeyCode::KeyP;
const KEY_TOGGLE_MSG: KeyCode = KeyCode::KeyM;

pub struct SandboxCameraControlPlugin;

impl Plugin for SandboxCameraControlPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(SANDBOX_CTX_STATE), show_control_info)
            .add_systems(
                Update,
                (
                    camera_vertical_move::<-1>.run_if(input_pressed(KEY_FORWARD)),
                    camera_vertical_move::<1>.run_if(input_pressed(KEY_BACK)),
                    camera_horizontal_move::<1>.run_if(input_pressed(KEY_RIGHT)),
                    camera_horizontal_move::<-1>.run_if(input_pressed(KEY_LEFT)),
                    zoom_camera::<1>.run_if(input_pressed(KEY_ZOOM_IN)),
                    zoom_camera::<-1>.run_if(input_pressed(KEY_ZOOM_OUT)),
                    rotate_camera::<-1>.run_if(input_pressed(KEY_ROTATE_FORWARD)),
                    rotate_camera::<1>.run_if(input_pressed(KEY_ROTATE_BACK)),
                    reset_camera.run_if(input_pressed(KEY_RESET_XF)),
                    print_camera_transform.run_if(input_just_pressed(KEY_PRINT_XF)),
                    toggle_control_msg.run_if(input_just_pressed(KEY_TOGGLE_MSG)),
                )
                    .run_if(in_state(SANDBOX_CTX_STATE)),
            );
    }
}

#[derive(Component)]
struct CameraControlInfo;

fn show_control_info(mut commands: Commands) {
    commands.spawn((
        StateScoped(SANDBOX_CTX_STATE),
        CameraControlInfo,
        Text(format!(
            "[Camera]\n  Move: {} {} {} {}\n  Zoom: {} {}\nRotate: {} {}\n Reset: {}\n\n{}",
            key_name(KEY_FORWARD),
            key_name(KEY_BACK),
            key_name(KEY_LEFT),
            key_name(KEY_RIGHT),
            key_name(KEY_ZOOM_IN),
            key_name(KEY_ZOOM_OUT),
            key_name(KEY_ROTATE_FORWARD),
            key_name(KEY_ROTATE_BACK),
            key_name(KEY_RESET_XF),
            format_args!(
                "[Misc]\nPrint camera xf: {}\nToggle this msg: {}",
                key_name(KEY_PRINT_XF),
                key_name(KEY_TOGGLE_MSG),
            ),
        )),
        Visibility::Hidden,
    ));
}

fn camera_horizontal_move<const D: i32>(mut xf: Single<&mut Transform, With<Camera3d>>) {
    xf.translation.x += D as f32 * 0.05;
}

fn camera_vertical_move<const D: i32>(mut xf: Single<&mut Transform, With<Camera3d>>) {
    xf.translation.z += D as f32 * 0.05;
}

fn zoom_camera<const D: i32>(mut xf: Single<&mut Transform, With<Camera3d>>) {
    let forward = xf.rotation.mul_vec3(Vec3::NEG_Z);
    xf.translation += forward * D as f32 * 0.1;
}

fn rotate_camera<const D: i32>(mut xf: Single<&mut Transform, With<Camera3d>>) {
    xf.rotation *= Quat::from_rotation_x(FRAC_PI_8 * 0.01 * D as f32);
}

fn reset_camera(mut xf: Single<&mut Transform, With<Camera3d>>) {
    xf.translation = CAMERA_TRANSLATION;
    xf.rotation = CAMERA_ROTATION;
}

fn print_camera_transform(xf: Single<&Transform, With<Camera3d>>) {
    info!("{:?}", *xf);
}

fn toggle_control_msg(mut vis: Single<&mut Visibility, With<CameraControlInfo>>) {
    **vis = match **vis {
        Visibility::Hidden => Visibility::Visible,
        Visibility::Visible => Visibility::Hidden,
        v => v,
    };
}

fn key_name(code: KeyCode) -> String {
    let s = format!("{:?}", code);
    s.strip_prefix("Key").map(|v| v.to_string()).unwrap_or(s)
}
