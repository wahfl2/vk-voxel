use std::f32::consts::{FRAC_PI_2, PI};

use ultraviolet::{Mat4, projection, Isometry3, Vec3, Rotor3, Vec2, Rotor2};
use vulkano::pipeline::graphics::viewport::Viewport;
use winit::event::VirtualKeyCode;

use crate::event_handler::InputHandler;

const RADIANS: f32 = PI / 180.0;

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

        let yaw_rot = Rotor3::from_rotation_xz(self.yaw);
        let pitch_rot = Rotor3::from_rotation_yz(self.pitch);

        let mut transform = Isometry3::new(self.pos, Rotor3::identity());
        transform.append_rotation(yaw_rot);
        transform.append_rotation(pitch_rot);

        proj * transform.into_homogeneous_matrix()
    }
}

pub struct CameraController {
    pub camera: Camera,
    pub sensitivity: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        Self { camera: Camera::default(), sensitivity: 0.002 }
    }
}

impl CameraController {
    pub fn new(camera: Camera, sensitivity: f32) -> Self {
        Self { camera, sensitivity }
    }

    pub fn turn(&mut self, delta: Vec2) {
        self.camera.yaw -= delta.x * self.sensitivity;
        if self.camera.yaw.abs() > 2.0 * PI {
            self.camera.yaw = self.camera.yaw % (2.0 * PI)
        }

        self.camera.pitch += delta.y * self.sensitivity;
        self.camera.pitch = self.camera.pitch.clamp(-FRAC_PI_2, FRAC_PI_2);
    }

    pub fn tick(&mut self, input: &InputHandler) {
        const MOVEMENT_SPEED: f32 = 0.75;

        // represents movement on the xz plane
        let mut movement = Vec2::zero();
        if input.is_pressed(VirtualKeyCode::W) {
            movement.y += 1.0;
        }
        if input.is_pressed(VirtualKeyCode::A) {
            movement.x += 1.0;
        }
        if input.is_pressed(VirtualKeyCode::S) {
            movement.y -= 1.0;
        }
        if input.is_pressed(VirtualKeyCode::D) {
            movement.x -= 1.0;
        }

        if movement != Vec2::zero() {
            movement = movement.normalized() * MOVEMENT_SPEED;
            movement.rotate_by(Rotor2::from_angle(-self.camera.yaw));
        }

        let mut vertical = 0.0;
        if input.is_pressed(VirtualKeyCode::Space) {
            vertical -= MOVEMENT_SPEED;
        }
        if input.is_pressed(VirtualKeyCode::LShift) {
            vertical += MOVEMENT_SPEED;
        }

        self.camera.pos += Vec3::new(movement.x, vertical, movement.y);
    }
}