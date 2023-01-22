use ultraviolet::{Vec2, Vec3};

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