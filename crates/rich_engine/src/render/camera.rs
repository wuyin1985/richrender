use bevy::prelude::*;
use crate::{FlyCamera, RenderCamera};

pub enum CameraOpEvent {
    Focus(Transform, f32),
    ChangeTranslation(Vec3),
    ChangeRotation(Quat),
}

#[derive(Debug)]
pub struct Camera {
    pub fov: f32,
    pub aspect: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            fov: 45f32.to_radians(),
            aspect: 1280f32 / 720f32,
            z_near: 1.0,
            z_far: 96.0,
        }
    }
}

impl Camera {
    pub fn compute_world_2_ui_position(camera: &Camera, camera_transform: &Transform, target_world_pos: Vec3, window_size: Vec2) -> Vec3 {
        let proj = Mat4::perspective_rh(camera.fov, camera.aspect, camera.z_near, camera.z_far);
        let view = camera_transform.compute_matrix().inverse();
        let clip_pos = (proj * view).mul_vec4(Vec4::from((target_world_pos, 1f32)));
        let mut res = Vec3::new(clip_pos.x / clip_pos.w, clip_pos.y / clip_pos.w, clip_pos.w);
        res = res * 0.5f32 + Vec3::new(0.5f32, 0.5f32, 0.0);
        res.y = 1f32 - res.y;
        Vec3::new(window_size.x * (res.x), window_size.y * (res.y), res.z)
    }

    pub fn update_camera_op_event_system(
        render_camera: Res<RenderCamera>,
        mut camera_query: Query<(&Camera, &mut Transform)>,
        mut event_read: EventReader<CameraOpEvent>,
    )
    {
        if let Ok((camera, mut camera_transform)) = camera_query.get_mut(render_camera.camera) {
            let mut changed = false;
            for event in event_read.iter() {
                changed = true;
                match event {
                    CameraOpEvent::Focus(target_transform, distance) => {
                        let eye_pos = target_transform.translation + target_transform.rotation.mul_vec3(Vec3::Z).normalize() * (*distance);
                        let look_mat = Mat4::look_at_rh(eye_pos, target_transform.translation, Vec3::Y).inverse();
                        let (_, r, t) = look_mat.to_scale_rotation_translation();
                        camera_transform.translation = t;
                        camera_transform.rotation = r;
                    }
                    CameraOpEvent::ChangeTranslation(pos) => {
                        camera_transform.translation = *pos;
                    }
                    CameraOpEvent::ChangeRotation(rot) => {
                        camera_transform.rotation = *rot;
                    }
                }
            }
        }
    }
}
