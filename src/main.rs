use vk::renderer::Renderer;
use vulkano_win::VkSurfaceBuild;
use winit::{event_loop::{EventLoop, ControlFlow}, window::WindowBuilder, event::{Event, WindowEvent}};

pub mod vk;

fn main() {
    
    let event_loop = EventLoop::new();
    let renderer = Renderer::new(&event_loop);

    event_loop.run(|event, _, control_flow| {
        match event {


            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
            _ => ()
        }
    });
}
