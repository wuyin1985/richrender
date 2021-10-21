use bevy::{input::mouse::MouseMotion, prelude::*};
use bevy::input::mouse::{MouseButtonInput, MouseWheel};

pub fn movement_axis(
    input: &Res<Input<KeyCode>>,
    plus: KeyCode,
    minus: KeyCode,
) -> f32 {
    let mut axis = 0.0;
    if input.pressed(plus) {
        axis += 1.0;
    }
    if input.pressed(minus) {
        axis -= 1.0;
    }
    axis
}


pub struct FlyCamera {
    /// The speed the FlyCamera accelerates at. Defaults to `1.0`
    pub accel: f32,
    /// The maximum speed the FlyCamera can move at. Defaults to `0.5`
    pub max_speed: f32,
    /// The sensitivity of the FlyCamera's motion based on mouse movement. Defaults to `3.0`
    pub sensitivity: f32,
    /// The amount of deceleration to apply to the camera's motion. Defaults to `1.0`
    pub friction: f32,
    /// The current pitch of the FlyCamera in degrees. This value is always up-to-date, enforced by [FlyCameraPlugin](struct.FlyCameraPlugin.html)
    pub pitch: f32,
    /// The current pitch of the FlyCamera in degrees. This value is always up-to-date, enforced by [FlyCameraPlugin](struct.FlyCameraPlugin.html)
    pub yaw: f32,
    /// The current velocity of the FlyCamera. This value is always up-to-date, enforced by [FlyCameraPlugin](struct.FlyCameraPlugin.html)
    pub velocity: Vec3,
    /// Key used to move forward. Defaults to <kbd>W</kbd>
    pub key_forward: KeyCode,
    /// Key used to move backward. Defaults to <kbd>S</kbd>
    pub key_backward: KeyCode,
    /// Key used to move left. Defaults to <kbd>A</kbd>
    pub key_left: KeyCode,
    /// Key used to move right. Defaults to <kbd>D</kbd>
    pub key_right: KeyCode,
    /// Key used to move up. Defaults to <kbd>Space</kbd>
    pub key_up: KeyCode,
    /// Key used to move forward. Defaults to <kbd>LShift</kbd>
    pub key_down: KeyCode,
    /// If `false`, disable keyboard control of the camera. Defaults to `true`
    pub enabled: bool,

    pub mouse_pressed: Option<MouseButton>,
}

impl FlyCamera {
    pub fn refresh_transform(&mut self, transform: &Transform) {
        let (z, x, y) = transform.rotation.to_euler(EulerRot::ZXY);
        self.pitch = -x.to_degrees();
        self.yaw = y.to_degrees();
    }
}

impl Default for FlyCamera {
    fn default() -> Self {
        Self {
            accel: 1.5,
            max_speed: 0.5,
            sensitivity: 3.0,
            friction: 1.0,
            pitch: 0.0,
            yaw: 0.0,
            velocity: Vec3::ZERO,
            key_forward: KeyCode::W,
            key_backward: KeyCode::S,
            key_left: KeyCode::A,
            key_right: KeyCode::D,
            key_up: KeyCode::Space,
            key_down: KeyCode::LShift,
            enabled: true,
            mouse_pressed: None,
        }
    }
}

fn forward_vector(rotation: &Quat) -> Vec3 {
    rotation.mul_vec3(Vec3::Z).normalize()
}

fn forward_walk_vector(rotation: &Quat) -> Vec3 {
    let f = forward_vector(rotation);
    let f_flattened = Vec3::new(f.x, 0.0, f.z).normalize();
    f_flattened
}

fn strafe_vector(rotation: &Quat) -> Vec3 {
    // Rotate it 90 degrees to get the strafe direction
    Quat::from_rotation_y(90.0f32.to_radians())
        .mul_vec3(forward_walk_vector(rotation))
        .normalize()
}

fn camera_movement_system(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut FlyCamera, &mut Transform)>,
) {
    for (mut options, mut transform) in query.iter_mut() {
        options.refresh_transform(&transform);

        let (axis_h, axis_v, axis_float) = if options.enabled {
            (
                movement_axis(&keyboard_input, options.key_right, options.key_left),
                movement_axis(
                    &keyboard_input,
                    options.key_backward,
                    options.key_forward,
                ),
                movement_axis(&keyboard_input, options.key_up, options.key_down),
            )
        } else {
            (0.0, 0.0, 0.0)
        };

        let rotation = transform.rotation;
        let accel: Vec3 = (strafe_vector(&rotation) * axis_h)
            + (forward_walk_vector(&rotation) * axis_v);
        //+ (Vec3::Y * axis_float);
        let accel: Vec3 = if accel.length() != 0.0 {
            accel.normalize() * options.accel
        } else {
            Vec3::ZERO
        };

        let friction: Vec3 = if options.velocity.length() != 0.0 {
            options.velocity.normalize() * -1.0 * options.friction
        } else {
            Vec3::ZERO
        };

        options.velocity += accel * time.delta_seconds();

        // clamp within max speed
        if options.velocity.length() > options.max_speed {
            options.velocity = options.velocity.normalize() * options.max_speed;
        }

        let delta_friction = friction * time.delta_seconds();

        options.velocity = if (options.velocity + delta_friction).signum()
            != options.velocity.signum()
        {
            Vec3::ZERO
        } else {
            options.velocity + delta_friction
        };

        transform.translation += options.velocity;
    }
}

fn mouse_wheel_system(
    time: Res<Time>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut query: Query<(&mut FlyCamera, &mut Transform)>,
)
{
    let mut x = 0f32;
    for event in mouse_wheel_events.iter() {
        x -= event.y;
    }
    if x == 0f32
    {
        return;
    }

    x = 5f32 * x * time.delta_seconds();

    for (mut options, mut transform) in query.iter_mut() {
        if !options.enabled {
            continue;
        }
        let f = transform.rotation.mul_vec3(Vec3::Z).normalize();
        transform.translation += f * x;
    }
}

fn mouse_motion_system(
    time: Res<Time>,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    mut mouse_motion_event_reader: EventReader<MouseMotion>,
    mut query: Query<(&mut FlyCamera, &mut Transform)>,
) {
    let mut delta: Vec2 = Vec2::ZERO;

    for event in mouse_motion_event_reader.iter() {
        delta += event.delta;
    }

    if delta.is_nan() {
        return;
    }

    for (mut options, mut transform) in query.iter_mut() {
        if !options.enabled {
            continue;
        }

        for event in mouse_button_input_events.iter() {
            if event.state.is_pressed() {
                options.mouse_pressed = Some(event.button);
            } else {
                if let Some(mp) = options.mouse_pressed {
                    if mp == event.button {
                        options.mouse_pressed = None;
                    }
                }
            }
        }

        if let Some(button) = options.mouse_pressed {
            match button {
                MouseButton::Left => {
                    options.yaw -= delta.x * options.sensitivity * time.delta_seconds();
                    options.pitch += delta.y * options.sensitivity * time.delta_seconds();

                    options.pitch = options.pitch.clamp(-89.0, 89.9);
                    // println!("pitch: {}, yaw: {}", options.pitch, options.yaw);

                    let yaw_radians = options.yaw.to_radians();
                    let pitch_radians = options.pitch.to_radians();

                    transform.rotation = Quat::from_axis_angle(Vec3::Y, yaw_radians)
                        * Quat::from_axis_angle(-Vec3::X, pitch_radians);
                }
                MouseButton::Middle => {
                    let d = delta * options.sensitivity * time.delta_seconds();
                    transform.translation = transform.translation + Vec3::new(-d.x, d.y, 0.0);
                }
                _ => {}
            }
        }
    }
}

pub struct FlyCameraPlugin;

impl Plugin for FlyCameraPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_system(camera_movement_system.system())
            .add_system(mouse_motion_system.system())
            .add_system(mouse_wheel_system.system());
    }
}
