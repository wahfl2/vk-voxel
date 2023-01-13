use event_handler::{InputHandlerEvent, InputHandler};
use render::{renderer::Renderer, util::RenderState, vertex::VertexRaw, fps_log::FpsLog, camera::camera::CameraController};

use winit::{event_loop::{EventLoop, ControlFlow, EventLoopBuilder}, event::{Event, WindowEvent}};

pub mod render;
pub mod world;
pub mod event_handler;

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    let event_loop: EventLoop<InputHandlerEvent> = EventLoopBuilder::with_user_event().build();
    let mut proxy = event_loop.create_proxy();

    let mut renderer = Renderer::new(&event_loop);
    let mut input_handler = InputHandler::new();
    let mut camera_controller = CameraController::default();
    let mut fps_log = FpsLog::new();

    let vertices = [
        VertexRaw {
            position: [1.0, 0.0, 0.0],
            tex_coords: [1.0, 0.0],
        },
        VertexRaw {
            position: [0.0, 1.0, 0.0],
            tex_coords: [0.0, 1.0],
        },
        VertexRaw {
            position: [0.0, 0.0, 0.0],
            tex_coords: [0.0, 0.0],
        },

        VertexRaw {
            position: [1.0, 1.0, 0.0],
            tex_coords: [1.0, 1.0],
        },
        VertexRaw {
            position: [0.0, 1.0, 0.0],
            tex_coords: [0.0, 1.0],
        },
        VertexRaw {
            position: [1.0, 0.0, 0.0],
            tex_coords: [1.0, 0.0],
        },
    ];

    renderer.vertex_chunk_buffer.push_chunk_vertices((0, 0).into(), vertices.as_slice());

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

                camera_controller.tick(&input_handler);
                renderer.cam_uniform = Some(camera_controller.camera.calculate_matrix(&renderer.viewport));
                match renderer.render() {
                    RenderState::OutOfDate | RenderState::Suboptimal => recreate_swapchain = true,
                    _ => ()
                }
            },

            Event::DeviceEvent { event, .. } => {
                input_handler.handle_event(event, &mut proxy);
            }

            Event::UserEvent(ev) => {
                match ev {
                    InputHandlerEvent::MouseMovement(delta) => {
                        camera_controller.turn(delta);
                    },
                    _ => ()
                }
            }

            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => window_resized = true,
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
            _ => ()
        }
    });
}