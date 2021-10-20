use bevy::prelude::*;

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
}
