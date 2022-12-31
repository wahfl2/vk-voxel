use std::sync::Arc;

use vulkano::{swapchain::{Surface, PresentFuture, SwapchainAcquireFuture}, sync::{FenceSignalFuture, JoinFuture, GpuFuture}, command_buffer::CommandBufferExecFuture};
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

pub type ExecuteFence = FenceSignalFuture<PresentFuture<CommandBufferExecFuture<JoinFuture<Box<dyn GpuFuture>, SwapchainAcquireFuture>>>>;