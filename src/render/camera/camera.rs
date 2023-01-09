use ultraviolet::{Mat4, projection, Isometry3, Vec3, Rotor3};

use crate::event_handler::InputHandlerEvent;

pub struct Camera {
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self { 
            pos: Vec3::zero(),
            yaw: 0.0, 
            pitch: 0.0, 
            fov: 90.0, 
            near: 0.1, 
            far: 10000.0,
        }
    }
}

impl Camera {
    /// Returns the matrix representing view projection and this camera's transform.
    pub fn matrix(&self, aspect_ratio: f32) -> Mat4 {
        let proj = projection::perspective_vk(self.fov, aspect_ratio, self.near, self.far);
        let yaw_rot = Rotor3::from_rotation_xz(self.yaw);
        let pitch_rot = Rotor3::from_rotation_yz(self.pitch);
        let transform = Isometry3::new(self.pos, yaw_rot * pitch_rot);
        proj * transform.into_homogeneous_matrix()
    }
}

pub struct CameraController {
    pub camera: Camera,
}

impl Default for CameraController {
    fn default() -> Self {
        Self { camera: Camera::default() }
    }
}

impl CameraController {
    pub fn new(camera: Camera) -> Self {
        Self { camera }
    }

    pub fn handle_event(&mut self, event: InputHandlerEvent) {
        match event {
            InputHandlerEvent::Cursor(delta) => {
                self.camera.yaw += delta.x as f32;
                self.camera.pitch += delta.y as f32;
            }
            _ => ()
        }
    }
}