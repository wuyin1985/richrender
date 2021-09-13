use glam::Mat4;

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
            z_near: 0.1,
            z_far: 10.0,
        }
    }
}
