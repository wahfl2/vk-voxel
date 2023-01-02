use render::{renderer::Renderer, util::RenderState, vertex::Vertex, fps_log::FpsLog};

use vulkano::{buffer::CpuAccessibleBuffer, memory::allocator::{GenericMemoryAllocator, GenericMemoryAllocatorCreateInfo}};
use winit::{event_loop::{EventLoop, ControlFlow}, event::{Event, WindowEvent}};

pub mod render;

fn main() {
    let event_loop = EventLoop::new();
    let mut renderer = Renderer::new(&event_loop);
    let mut fps_log = FpsLog::new();

    let vertices = [
        Vertex {
            position: [-0.5, -0.25],
        },
        Vertex {
            position: [0.0, 0.5],
        },
        Vertex {
            position: [0.25, -0.1],
        },
    ];

    renderer.overwrite_vbuffer(&vertices);

    let mut window_resized = false;
    let mut recreate_swapchain = false;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::MainEventsCleared => {
                fps_log.update();
                if window_resized || recreate_swapchain {
                    recreate_swapchain = false;
                    renderer.recreate_swapchain();

                    if window_resized {
                        window_resized = false;
                        renderer.recreate_pipeline();
                    }
                }

                match renderer.render() {
                    RenderState::OutOfDate | RenderState::Suboptimal => recreate_swapchain = true,
                    _ => ()
                }
            },

            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => window_resized = true,
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
            _ => ()
        }
    });
}
