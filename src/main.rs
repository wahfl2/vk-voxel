use event_handler::{InputHandlerEvent, InputHandler};
use render::{renderer::Renderer, util::RenderState, fps_log::FpsLog, camera::camera::CameraController, mesh::cube::UnitCube};

use ultraviolet::{Vec3, Vec2};
use winit::{event_loop::{EventLoop, ControlFlow, EventLoopBuilder}, event::{Event, WindowEvent, DeviceEvent}};

pub mod render;
pub mod util;
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

    let cube = UnitCube {
        center: Vec3::zero(),
        texture_idx: 2,
    };

    renderer.upload_chunk((0, 0).into(), cube);

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

            Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta }, .. } => {
                camera_controller.turn(Vec2::new(delta.0 as f32, delta.1 as f32));
            }

            Event::DeviceEvent { event, .. } => {
                input_handler.handle_event(event, &mut proxy);
            }

            Event::UserEvent(_ev) => {
                
            }

            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => window_resized = true,
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
            _ => ()
        }
    });
}