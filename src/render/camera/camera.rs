use std::f32::consts::{FRAC_PI_2, PI};

use ultraviolet::{Mat4, projection, Isometry3, Vec3, Rotor3, Vec2, Rotor2};
use vulkano::pipeline::graphics::viewport::Viewport;
use winit::event::VirtualKeyCode;

use crate::{event_handler::InputHandler, util::util::EulerRot2};

const RADIANS: f32 = PI / 180.0;

#[derive(Clone)]
pub struct Camera {
    pub pos: Vec3,
    pub rotation: EulerRot2,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self { 
            pos: Vec3::zero(),
            rotation: EulerRot2::new(0.0, 0.0),
            fov: 80.0, 
            near: 0.1, 
            far: 1000.0,
        }
    }
}

impl Camera {
    /// Returns the matrix representing view projection and this camera's transform.
    pub fn calculate_matrix(&self, viewport: &Viewport) -> Mat4 {
        let aspect_ratio = viewport.dimensions[0] / viewport.dimensions[1];
        let proj = projection::perspective_vk(self.fov * RADIANS, aspect_ratio, self.near, self.far);

        let yaw_rot = Rotor3::from_rotation_xz(self.rotation.yaw);
        let pitch_rot = Rotor3::from_rotation_yz(self.rotation.pitch);

        let real_pos = -self.pos;
        let mut transform = Isometry3::new(real_pos, Rotor3::identity());
        transform.append_rotation(yaw_rot);
        transform.append_rotation(pitch_rot);

        proj * transform.into_homogeneous_matrix()
    }

    pub fn with_pos(&self, pos: Vec3) -> Self {
        Self {
            pos,
            ..self.clone()
        }
    }
}