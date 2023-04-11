use std::{ops::{Add, AddAssign}, f32::consts::PI};

use ultraviolet::{Vec2, Vec3, Rotor3, IVec3, IVec2, UVec3, UVec2};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub fn point_on_plane(&self, plane: f32, point: Vec2) -> Vec3 {
        match self {
            Self::X => Vec3::new(plane, point.x, point.y),
            Self::Y => Vec3::new(point.x, plane, point.y),
            Self::Z => Vec3::new(point.x, point.y, plane),
        }
    }

    pub fn unit_vec(&self) -> Vec3 {
        match self {
            Self::X => Vec3::unit_x(),
            Self::Y => Vec3::unit_y(),
            Self::Z => Vec3::unit_z(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd)]
pub enum Sign {
    Positive,
    Negative,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd)]
pub struct Facing {
    pub axis: Axis,
    pub sign: Sign,
}

impl Facing {
    pub const UP: Self = Self { axis: Axis::Y, sign: Sign::Positive };
    pub const DOWN: Self = Self { axis: Axis::Y, sign: Sign::Negative };
    pub const RIGHT: Self = Self { axis: Axis::X, sign: Sign::Positive };
    pub const LEFT: Self = Self { axis: Axis::X, sign: Sign::Negative };
    pub const FORWARD: Self = Self { axis: Axis::Z, sign: Sign::Positive };
    pub const BACK: Self = Self { axis: Axis::Z, sign: Sign::Negative };

    pub fn new(axis: Axis, sign: Sign) -> Self {
        Self { axis, sign }
    }

    pub fn to_num(&self) -> usize {
        match (self.sign, self.axis) {
            (Sign::Positive, Axis::X) => 0,
            (Sign::Negative, Axis::X) => 1,
            (Sign::Positive, Axis::Y) => 2,
            (Sign::Negative, Axis::Y) => 3,
            (Sign::Positive, Axis::Z) => 4,
            (Sign::Negative, Axis::Z) => 5,
        }
    }

    pub fn from_num(num: usize) -> Self {
        match num {
            0 => Self::new(Axis::X, Sign::Positive),
            1 => Self::new(Axis::X, Sign::Negative),
            2 => Self::new(Axis::Y, Sign::Positive),
            3 => Self::new(Axis::Y, Sign::Negative),
            4 => Self::new(Axis::Z, Sign::Positive),
            5 => Self::new(Axis::Z, Sign::Negative),
            _ => panic!("Out of bounds.")
        }
    }

    pub fn opposite(&self) -> Self {
        Self { 
            axis: self.axis, 
            sign: match self.sign {
                Sign::Positive => Sign::Negative,
                Sign::Negative => Sign::Positive,
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct EulerRot2 {
    pub yaw: f32,
    pub pitch: f32,
}

impl EulerRot2 {
    pub fn new(yaw: f32, pitch: f32) -> Self {
        Self { yaw, pitch }
    }

    pub fn get_rotor(&self) -> Rotor3 {
        Rotor3::from_rotation_xz(self.yaw) *
            Rotor3::from_rotation_yz(self.pitch
        )
    }

    pub fn get_reversed_rotor(&self) -> Rotor3 {
        Rotor3::from_rotation_xz(PI - self.yaw) *
            Rotor3::from_rotation_yz(-self.pitch)
    }
}

impl From<Vec2> for EulerRot2 {
    fn from(value: Vec2) -> Self {
        Self::new(value.x, value.y)
    }
}

impl Add<EulerRot2> for EulerRot2 {
    type Output = EulerRot2;

    fn add(self, rhs: EulerRot2) -> Self::Output {
        Self::new(self.yaw + rhs.yaw, self.pitch + rhs.pitch)
    }
}

impl AddAssign<EulerRot2> for EulerRot2 {
    fn add_assign(&mut self, rhs: EulerRot2) {
        *self = *self + rhs;
    }
}

pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn get_points(&self) -> [Vec3; 8] {
        let min = self.min;
        let max = self.max;
        [
            min,
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(min.x, max.y, min.z),
            max,
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(max.x, min.y, max.z),
        ]
    }
}

pub trait AdditionalSwizzles {
    type Out;

    fn xz(&self) -> Self::Out;
}

impl AdditionalSwizzles for Vec3 {
    type Out = Vec2;

    fn xz(&self) -> Self::Out {
        Self::Out::new(self.x, self.z)
    }
}

impl AdditionalSwizzles for IVec3 {
    type Out = IVec2;

    fn xz(&self) -> Self::Out {
        Self::Out::new(self.x, self.z)
    }
}

pub trait MoreCmp {
    fn all_greater_than(&self, rhs: &Self) -> bool;
    fn all_less_than(&self, rhs: &Self) -> bool;
    fn any_greater_than(&self, rhs: &Self) -> bool;
    fn any_less_than(&self, rhs: &Self) -> bool;
}

impl MoreCmp for Vec3 {
    fn all_greater_than(&self, rhs: &Self) -> bool {
        self.x > rhs.x && self.y > rhs.y && self.z > rhs.z
    }

    fn all_less_than(&self, rhs: &Self) -> bool {
        self.x < rhs.x && self.y < rhs.y && self.z < rhs.z
    }

    fn any_greater_than(&self, rhs: &Self) -> bool {
        self.x > rhs.x || self.y > rhs.y || self.z > rhs.z
    }

    fn any_less_than(&self, rhs: &Self) -> bool {
        self.x < rhs.x || self.y < rhs.y || self.z < rhs.z
    }
}

pub trait VecRounding {
    fn round(self) -> Self;
    fn floor(self) -> Self;
    fn ceil(self) -> Self;
}

impl VecRounding for Vec3 {
    fn round(self) -> Self {
        Self::new(self.x.round(), self.y.round(), self.z.round())
    }

    fn floor(self) -> Self {
        Self::new(self.x.floor(), self.y.floor(), self.z.floor())
    }

    fn ceil(self) -> Self {
        Self::new(self.x.ceil(), self.y.ceil(), self.z.ceil())
    }
}

pub trait Vec3Trunc {
    fn into_i(self) -> IVec3;
}

impl Vec3Trunc for Vec3 {
    fn into_i(self) -> IVec3 {
        IVec3::new(self.x as i32, self.y as i32, self.z as i32)
    }
}

pub trait VecAxisIndex {
    fn get(&self, axis: ndarray::Axis) -> f32;
    fn get_mut(&mut self, axis: ndarray::Axis) -> &mut f32;

    fn set(&mut self, axis: ndarray::Axis, value: f32);
}

impl VecAxisIndex for Vec3 {
    fn get(&self, axis: ndarray::Axis) -> f32 {
        match axis.0 {
            0 => self.x,
            1 => self.y,
            2 => self.z,
            d => panic!("Tried to get {d} axis of Vec3")
        }
    }

    fn get_mut(&mut self, axis: ndarray::Axis) -> &mut f32 {
        match axis.0 {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            d => panic!("Tried to get {d} axis of Vec3")
        }
    }

    fn set(&mut self, axis: ndarray::Axis, value: f32) {
        match axis.0 {
            0 => self.x = value,
            1 => self.y = value,
            2 => self.z = value,
            d => panic!("Tried to set {d} axis of Vec3")
        }
    }
}

pub trait MoreVecOps {
    fn powf(self, n: f32) -> Self;
    fn powi(self, n: i32) -> Self;
}

impl MoreVecOps for Vec3 {
    fn powf(self, n: f32) -> Self {
        Self::new(self.x.powf(n), self.y.powf(n), self.z.powf(n))
    }

    fn powi(self, n: i32) -> Self {
        Self::new(self.x.powi(n), self.y.powi(n), self.z.powi(n))
    }
}

pub trait UVecToSigned {
    type IVec;

    fn signed(self) -> Self::IVec;
}

impl UVecToSigned for UVec3 {
    type IVec = IVec3;

    fn signed(self) -> Self::IVec {
        IVec3::new(self.x as i32, self.y as i32, self.z as i32)
    }
}

impl UVecToSigned for UVec2 {
    type IVec = IVec2;

    fn signed(self) -> Self::IVec {
        IVec2::new(self.x as i32, self.y as i32)
    }
}

#[cfg(test)]
mod test {
    use std::{time::Instant, hint::black_box};

    use ahash::{HashSet, HashSetExt};
    use rand_xoshiro::{Xoshiro128StarStar, rand_core::{SeedableRng, RngCore}};
    use turborand::{rng::Rng, GenCore};

    use super::*;

    #[test]
    fn set_speed_test() {
        let mut rng = Xoshiro128StarStar::seed_from_u64(Rng::default().gen_u64());
        let mut set = HashSet::new();
        let mut vec = Vec::new();
        const DATA_POINTS: usize = 50_000;
    
        for _ in 0..DATA_POINTS {
            let random = IVec2::new((rng.next_u32() as i64 + i32::MIN as i64) as i32, (rng.next_u32() as i64 + i32::MIN as i64) as i32);
            set.insert(random);
            vec.push(random)
        }
        
        let mut reads = 0;
        let get = IVec2::new((rng.next_u32() as i64 + i32::MIN as i64) as i32, (rng.next_u32() as i64 + i32::MIN as i64) as i32);
        let start = Instant::now();
        while Instant::now().duration_since(start).as_secs_f32() < 1.0 {
            black_box(set.contains(&get));
            reads += 1;
        }
        println!("SET: {reads} reads in 1 second, {:.2}ns per read.", 1_000_000_000.0 / reads as f32);
    
        let mut reads = 0;
        let start = Instant::now();
        while Instant::now().duration_since(start).as_secs_f32() < 1.0 {
            black_box(vec.contains(&get));
            reads += 1;
        }
        println!("VEC: {reads} reads in 1 second, {:.2}ns per read.", 1_000_000_000.0 / reads as f32);
    }
}
