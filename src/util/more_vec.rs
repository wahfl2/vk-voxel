use std::ops::{Index, IndexMut, Mul, Add};

use ultraviolet::{Vec3, Vec2, UVec3};

#[derive(Copy, Clone, Debug)]
pub struct UsizeVec3 {
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

impl UsizeVec3 {
    pub fn new(x: usize, y: usize, z: usize) -> Self {
        Self { x, y, z }
    }
    
    pub fn into_vec3(self) -> Vec3 {
        Vec3::new(self.x as f32, self.y as f32, self.z as f32)
    }
}

impl<T, const N: usize, const M: usize, const P: usize> Index<UsizeVec3> for [[[T; N]; M]; P] {
    type Output = T;

    fn index(&self, index: UsizeVec3) -> &Self::Output {
        &self[index.x][index.y][index.z]
    }
}

impl<T, const N: usize, const M: usize, const P: usize> IndexMut<UsizeVec3> for [[[T; N]; M]; P] {
    fn index_mut(&mut self, index: UsizeVec3) -> &mut Self::Output {
        &mut self[index.x][index.y][index.z]
    }
}

impl Mul<usize> for UsizeVec3 {
    type Output = UsizeVec3;

    fn mul(self, rhs: usize) -> Self::Output {
        (self.x * rhs, self.y * rhs, self.z * rhs).into()
    }
}

impl Add<UsizeVec3> for UsizeVec3 {
    type Output = Self;

    fn add(self, rhs: UsizeVec3) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl From<(usize, usize, usize)> for UsizeVec3 {
    fn from(value: (usize, usize, usize)) -> Self {
        Self::new(value.0, value.1, value.2)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct UsizeVec2 {
    pub x: usize,
    pub y: usize,
}

impl UsizeVec2 {
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
    
    pub fn into_vec2(self) -> Vec2 {
        Vec2::new(self.x as f32, self.y as f32)
    }
}

impl<T, const N: usize, const M: usize> Index<UsizeVec2> for [[T; N]; M] {
    type Output = T;

    fn index(&self, index: UsizeVec2) -> &Self::Output {
        &self[index.x][index.y]
    }
}

impl<T, const N: usize, const M: usize> IndexMut<UsizeVec2> for [[T; N]; M] {
    fn index_mut(&mut self, index: UsizeVec2) -> &mut Self::Output {
        &mut self[index.x][index.y]
    }
}

impl Mul<usize> for UsizeVec2 {
    type Output = UsizeVec2;

    fn mul(self, rhs: usize) -> Self::Output {
        (self.x * rhs, self.y * rhs).into()
    }
}

impl From<(usize, usize)> for UsizeVec2 {
    fn from(value: (usize, usize)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl From<UVec3> for UsizeVec3 {
    fn from(value: UVec3) -> Self {
        Self::new(value.x as usize, value.y as usize, value.z as usize)
    }
}