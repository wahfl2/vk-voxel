use event_handler::{InputHandlerEvent, InputHandler};
use render::{renderer::Renderer, util::RenderState, vertex::VertexRaw, fps_log::FpsLog};

use winit::{event_loop::{EventLoop, ControlFlow, EventLoopBuilder}, event::{Event, WindowEvent}};

pub mod render;
pub mod event_handler;

fn main() {
    let event_loop: EventLoop<InputHandlerEvent> = EventLoopBuilder::with_user_event().build();
    let mut renderer = Renderer::new(&event_loop);
    let mut input_handler = InputHandler::new();
    let mut fps_log = FpsLog::new();

    let vertices = [
        VertexRaw {
            position: [-0.5, -0.25, 0.0],
            color: [1.0, 0.0, 0.0, 1.0],
        },
        VertexRaw {
            position: [0.0, 0.5, 0.0],
            color: [0.0, 1.0, 0.0, 1.0],
        },
        VertexRaw {
            position: [0.25, -0.1, 0.0],
            color: [0.0, 0.0, 1.0, 1.0],
        },
    ];

    // renderer.add_vertices(&vertices);

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

            Event::DeviceEvent { event, .. } => {
                input_handler.handle_event(event);
            }

            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => window_resized = true,
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
            _ => ()
        }
    });
}
