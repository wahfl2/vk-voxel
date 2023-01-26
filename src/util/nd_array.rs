use std::ops::{IndexMut, Index};

use crate::util::more_vec::UsizeVec3;

use super::more_vec::UsizeVec2;

pub struct CubeArray<T, const N: usize> {
    /// Index using `[x][y][z]` or a `UsizeVec3`
    pub inner: [[[T; N]; N]; N],
}

impl<T, const N: usize> CubeArray<T, N> {
    
}

impl<T: Copy> CubeArray<T, 0> {
    pub fn fill_copy<const N: usize>(item: T) -> CubeArray<T, N> {
        CubeArray {
            inner: [[[item; N]; N]; N]
        }
    }
}

impl<T, const N: usize> From<[[[T; N]; N]; N]> for CubeArray<T, N> {
    fn from(value: [[[T; N]; N]; N]) -> Self {
        Self { inner: value }
    }
}

impl<T, const N: usize> Index<UsizeVec3> for CubeArray<T, N> {
    type Output = T;

    fn index(&self, index: UsizeVec3) -> &Self::Output {
        &self.inner[index]
    }
}

impl<T, const N: usize> IndexMut<UsizeVec3> for CubeArray<T, N> {
    fn index_mut(&mut self, index: UsizeVec3) -> &mut Self::Output {
        &mut self.inner[index]
    }
}

pub struct SquareArray<T, const N: usize> {
    pub inner: [[T; N]; N],
}

impl<T: Copy> SquareArray<T, 0> {
    pub fn fill_copy<const N: usize>(item: T) -> SquareArray<T, N> {
        SquareArray {
            inner: [[item; N]; N]
        }
    }
}

impl<T, const N: usize> From<[[T; N]; N]> for SquareArray<T, N> {
    fn from(value: [[T; N]; N]) -> Self {
        Self { inner: value }
    }
}

impl<T, const N: usize> Index<UsizeVec2> for SquareArray<T, N> {
    type Output = T;

    fn index(&self, index: UsizeVec2) -> &Self::Output {
        &self.inner[index]
    }
}

impl<T, const N: usize> IndexMut<UsizeVec2> for SquareArray<T, N> {
    fn index_mut(&mut self, index: UsizeVec2) -> &mut Self::Output {
        &mut self.inner[index]
    }
}