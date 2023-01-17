use std::sync::Arc;

use guillotiere::euclid::{Size2D, UnknownUnit, Box2D};
use ultraviolet::{UVec2, Vec2};
use vulkano::{swapchain::Surface, image::ImageDimensions};
use winit::window::Window;

use super::mesh::quad::QuadUV;

pub trait GetWindow {
    fn get_window(&self) -> Option<Arc<Window>>;
}

impl GetWindow for Arc<Surface> {
    fn get_window(&self) -> Option<Arc<Window>> {
        match self.object().unwrap().clone().downcast::<Window>() {
            Ok(w) => Some(w),
            Err(_) => None,
        }
    }
}

pub enum RenderState {
    Ok,
    Suboptimal,
    OutOfDate,
}

pub trait BoxToUV {
    fn to_quad_uv(self, atlas_size: UVec2) -> QuadUV;
}

impl BoxToUV for Box2D<i32, UnknownUnit> {
    fn to_quad_uv(self, atlas_size: UVec2) -> QuadUV {
        let size = Vec2::from(atlas_size);

        QuadUV {
            min: Vec2::new(self.min.x as f32 / size.x, self.min.y as f32 / size.y),
            max: Vec2::new(self.max.x as f32 / size.x, self.max.y as f32 / size.y),
        }        
    }
}

pub trait VecConvenience {
    type Value;
    fn splat(v: Self::Value) -> Self;
    fn to_size_2d(self) -> Size2D<i32, UnknownUnit>;
    fn to_image_dimensions(self) -> ImageDimensions;
}

impl VecConvenience for UVec2 {
    type Value = u32;
    fn splat(v: Self::Value) -> Self {
        Self::new(v, v)
    }

    fn to_size_2d(self) -> Size2D<i32, UnknownUnit> {
        Size2D::new(self.x as i32, self.y as i32)
    }

    fn to_image_dimensions(self) -> ImageDimensions {
        ImageDimensions::Dim2d { width: self.x, height: self.y, array_layers: 1 }
    }
}

pub trait Reversed {
    fn reversed(self) -> Self;
}

impl<T, const N: usize> Reversed for [T; N] {
    fn reversed(mut self) -> Self {
        self.reverse();
        self
    }
}