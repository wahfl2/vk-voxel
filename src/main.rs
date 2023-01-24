use std::time::{Instant, Duration};

use event_handler::{InputHandler, UserEvent};
use render::{renderer::Renderer, util::{RenderState, GetWindow}, fps_log::FpsLog, camera::camera::CameraController};

use ultraviolet::{Vec2, Vec3};
use util::AdditionalSwizzles;
use winit::{event_loop::{EventLoop, ControlFlow, EventLoopBuilder}, event::{Event, WindowEvent, DeviceEvent}};
use world::{block_data::StaticBlockData, world::World};

pub mod render;
pub mod util;
pub mod world;
pub mod event_handler;

pub const FRAME_TIME: f64 = 1.0 / 60.0;

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    let event_loop: EventLoop<UserEvent> = EventLoopBuilder::with_user_event().build();
    let mut proxy = event_loop.create_proxy();

    let mut renderer = Renderer::new(&event_loop);
    let mut static_block_data = StaticBlockData::empty();
    static_block_data.init(&renderer.texture_atlas);
    let mut world = World::new();

    for _ in 0..100 {
        world.frame_update(&mut renderer, &static_block_data);
    }

    let mut input_handler = InputHandler::new();
    let mut camera_controller = CameraController::default();
    let mut fps_log = FpsLog::new();

    let mut window_resized = false;
    let mut recreate_swapchain = false;
    let mut last_frame_start = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(_) => {
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
                world.player_pos = camera_controller.camera.pos.xz();
                renderer.cam_uniform = Some(camera_controller.camera.calculate_matrix(&renderer.viewport));

                match renderer.render() {
                    RenderState::OutOfDate | RenderState::Suboptimal => recreate_swapchain = true,
                    _ => ()
                }
            },

            Event::RedrawEventsCleared => {
                let next_render = last_frame_start + Duration::from_secs_f64(FRAME_TIME);
                proxy.send_event(UserEvent::RedrawAt(next_render)).unwrap();
            }

            Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta }, .. } => {
                camera_controller.turn(Vec2::new(delta.0 as f32, delta.1 as f32));
            }

            Event::DeviceEvent { event, .. } => {
                input_handler.handle_event(event, &mut proxy);
            }

            Event::UserEvent(ev) => {
                match ev {
                    UserEvent::RedrawAt(instant) => {
                        if Instant::now() >= instant {
                            last_frame_start = Instant::now();
                            renderer.vk_surface.get_window().unwrap().request_redraw();
                        } else {
                            proxy.send_event(UserEvent::RedrawAt(instant)).unwrap();
                        }
                    }

                    _ => ()
                }
            }

            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => window_resized = true,
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
            _ => ()
        }
    });
}