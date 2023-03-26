use std::{f32::consts::PI, time::Instant, hint::black_box, simd::{Simd, f32x4, f32x8}};

use rand_xoshiro::{Xoshiro128StarStar, rand_core::{SeedableRng, RngCore}};
use rayon::prelude::IntoParallelIterator;
use ultraviolet::{Mat4, projection, Isometry3, Vec3, Rotor3};
use vulkano::pipeline::graphics::viewport::Viewport;

use crate::{util::util::{EulerRot2, Aabb}, server::components::Hitbox};

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

    pub fn is_in_frustrum(&self, point: Vec3, aspect_ratio: f32) -> bool {
        let forward = Vec3::unit_z().rotated_by(self.rotation.get_rotor());
        let right = Vec3::unit_x().rotated_by(self.rotation.get_rotor());

        let v = point - self.pos;
        let pcz = v.dot(forward);

        if pcz < self.near || pcz > self.far {
            return false;
        }

        let pcx = v.dot(right);
        let pcy = v.dot(Vec3::unit_y());

        let frustrum_height = 2.0 * pcz * f32::tan(self.fov * 0.5);
        let frustrum_width = frustrum_height * aspect_ratio;
        let half_h = frustrum_height * 0.5;
        let half_w = frustrum_width * 0.5;

        if -half_w > pcx || pcx > half_w {
            return false;
        }

        if -half_h > pcy || pcy > half_h {
            return false;
        }

        true
    }
}

#[test]
fn frustrum_speed_test() {
    let mut rng = Xoshiro128StarStar::seed_from_u64(39847);
    const NUM_AABBS: u32 = 50_000;
    const NUM_TEST_VALUES: u32 = NUM_AABBS * 8;
    const NORMALIZE: f32 = 5.0 / u64::MAX as f32;

    let test_values = (0..NUM_TEST_VALUES).into_iter().map(|_| {
        Vec3::new(rng.next_u64() as f32 * NORMALIZE, rng.next_u64() as f32 * NORMALIZE, rng.next_u64() as f32 * NORMALIZE)
    }).collect::<Vec<_>>();

    let cam = Camera::default();
    let ratio = 16.0 / 9.0;
    let start = Instant::now();
    for v in test_values.into_iter() {
        black_box(cam.is_in_frustrum(v, ratio));
    }
    
    let duration = Instant::now().duration_since(start).as_secs_f64();
    println!("{NUM_TEST_VALUES} frustrum point tests done in {}ms", duration * 1_000.0);
    println!("{}ns per AABB", (duration / NUM_AABBS as f64) * 1_000_000_000.0);
}