use std::f32::consts::PI;

use ultraviolet::{Mat4, projection, Isometry3, Vec3, Rotor3};
use vulkano::pipeline::graphics::viewport::Viewport;

use crate::util::util::{EulerRot2, Aabb};

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

    pub fn calculate_frustrum(&self, aspect_ratio: f32) -> CalculatedFrustrum {
        let real_pos = self.pos;
        let rotor = self.rotation.get_reversed_rotor();

        let right = Vec3::unit_x().rotated_by(rotor);
        let up = Vec3::unit_y().rotated_by(rotor);
        let forward = Vec3::unit_z().rotated_by(rotor);

        let tan = f32::tan(self.fov * 0.5);
        let h_near_height = self.near * tan;
        let h_near_width = h_near_height * aspect_ratio;

        let near_center = real_pos + (forward * self.near);
        let far_center = real_pos + (forward * self.far);

        let near_right = near_center + (right * h_near_width);
        let near_left = near_center - (right * h_near_width);
        let near_up = near_center + (up * h_near_height);
        let near_down = near_center - (up * h_near_height);

        let n_right = (near_right - real_pos).normalized().cross(up);
        let n_left = (near_left - real_pos).normalized().cross(-up);
        let n_up = (near_up - real_pos).normalized().cross(-right);
        let n_down = (near_down - real_pos).normalized().cross(right);

        CalculatedFrustrum { planes: [
            Plane::new(near_center, forward),
            Plane::new(far_center, -forward),
            Plane::new(real_pos, -n_right),
            Plane::new(real_pos, -n_left),
            Plane::new(real_pos, -n_up),
            Plane::new(real_pos, -n_down),
        ]}
    }

    pub fn with_pos(&self, pos: Vec3) -> Self {
        Self {
            pos,
            ..self.clone()
        }
    }
}

#[test]
fn frustrum_test() {
    let frustrum = Camera {
        fov: 90.0,
        ..Default::default()
    }.calculate_frustrum(1.0);

    dbg!(frustrum);


}

#[derive(Debug, Clone, Copy)]
pub struct Plane {
    origin: Vec3,
    normal: Vec3,
}

impl Plane {
    fn new(origin: Vec3, normal: Vec3) -> Self {
        Self { origin, normal }
    }

    fn distance(&self, point: Vec3) -> f32 {
        self.normal.dot(point - self.origin)
    }
}

#[test]
fn distance_test() {
    assert_eq!(Plane::new(Vec3::new(0.0, 0.0, 0.0), Vec3::unit_y()).distance(Vec3::zero()), 0.0);
    assert_eq!(Plane::new(Vec3::new(0.0, 0.0, 0.0), Vec3::unit_y()).distance(Vec3::unit_y()), 1.0);
}

#[derive(Debug)]
pub struct CalculatedFrustrum {
    planes: [Plane; 6]
}

impl CalculatedFrustrum {
    pub fn should_render(&self, aabb: Aabb) -> bool {
        let planes = self.planes;
        for plane in planes.iter() {
            let n = plane.normal;

            let mut positive = aabb.min;
            let mut negative = aabb.max;
            if n.x >= 0.0 { positive.x = aabb.max.x }
            if n.y >= 0.0 { positive.y = aabb.max.y }
            if n.z >= 0.0 { positive.z = aabb.max.z }

            if n.x >= 0.0 { negative.x = aabb.min.x }
            if n.y >= 0.0 { negative.y = aabb.min.y }
            if n.z >= 0.0 { negative.z = aabb.min.z }

            if plane.distance(positive) < 0.0 {
                return false
            }
        }

        true
    }
}

#[cfg(test)]
mod test {
    use std::{time::Instant, hint::black_box};

    use rand_xoshiro::{Xoshiro128StarStar, rand_core::{SeedableRng, RngCore}};

    use super::*;

    #[test]
    fn frustrum_speed_test() {
        let mut rng = Xoshiro128StarStar::seed_from_u64(39847);
        const NUM_TEST_VALUES: u32 = 500_000;
        const NORMALIZE: f32 = 5.0 / u64::MAX as f32;

        let test_values = (0..NUM_TEST_VALUES).into_iter().map(|_| {
            Vec3::new(rng.next_u64() as f32 * NORMALIZE, rng.next_u64() as f32 * NORMALIZE, rng.next_u64() as f32 * NORMALIZE)
        }).map(|v| { Aabb::new(v - (5.0 * Vec3::one()), v) }).collect::<Vec<_>>();

        let cam = Camera::default();
        let ratio = 16.0 / 9.0;
        let start = Instant::now();
        let frustrum = cam.calculate_frustrum(ratio);
        for v in test_values.into_iter() {
            black_box(frustrum.should_render(v));
        }
        
        let duration = Instant::now().duration_since(start).as_secs_f64();
        println!("\n{NUM_TEST_VALUES} frustrum AABB tests done in {}ms", duration * 1_000.0);
        println!("{}ns per AABB", (duration / NUM_TEST_VALUES as f64) * 1_000_000_000.0);
    }
}