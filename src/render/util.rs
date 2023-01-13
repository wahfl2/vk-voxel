use std::sync::Arc;

use guillotiere::euclid::{Size2D, UnknownUnit};
use ultraviolet::UVec2;
use vulkano::{swapchain::Surface, image::ImageDimensions};
use winit::window::Window;

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