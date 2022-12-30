use std::sync::Arc;

use vulkano::swapchain::Surface;
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