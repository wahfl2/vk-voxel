#![feature(array_zip)]
#![feature(slice_as_chunks)]
#![feature(slice_flatten)]
#![feature(iter_collect_into)]
#![feature(portable_simd)]

use std::time::{Instant, Duration};

use event_handler::{InputHandler, UserEvent};
use mimalloc::MiMalloc;
use render::{renderer::Renderer, util::{RenderState, GetWindow}, fps_log::FpsLog};

use server::server::Server;
use ultraviolet::Vec2;
use crate::util::util::AdditionalSwizzles;
use winit::{event_loop::{EventLoop, ControlFlow, EventLoopBuilder}, event::{Event, WindowEvent}};
use world::{block_data::StaticBlockData, world_blocks::WorldBlocks};

pub mod render;
pub mod util;
pub mod world;
pub mod event_handler;
pub mod server;
pub mod physics;

pub const FRAME_TIME: f64 = 1.0 / 1000.0;

// #[global_allocator]
// pub static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    let event_loop: EventLoop<UserEvent> = EventLoopBuilder::with_user_event().build();
    let mut proxy = event_loop.create_proxy();

    let mut renderer = Renderer::new(&event_loop);
    let mut static_block_data = StaticBlockData::empty();
    static_block_data.init(&renderer.texture_atlas);
    let mut world_blocks = WorldBlocks::new(&static_block_data);
    let mut server = Server::new();
    server.init_single_player();

    let mut input_handler = InputHandler::new();
    let mut fps_log = FpsLog::new();

    let mut window_resized = false;
    let mut recreate_swapchain = false;
    let mut last_frame_start = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(_) => {
                let delta_time = fps_log.update();
                if window_resized || recreate_swapchain {
                    recreate_swapchain = false;
                    renderer.recreate_swapchain();

                    if window_resized {
                        window_resized = false;
                        renderer.recreate_pipeline();
                    }
                }

                let camera = server.get_camera();
                world_blocks.player_pos = -camera.pos.xz();
                world_blocks.frame_update(&static_block_data);
                server.tick(delta_time, &input_handler, &world_blocks, &static_block_data);
                renderer.cam_uniform = Some(camera.calculate_matrix(&renderer.viewport));

                input_handler.mouse_delta = Vec2::zero();

                match renderer.render(&mut world_blocks, &static_block_data, &camera) {
                    RenderState::OutOfDate | RenderState::Suboptimal => recreate_swapchain = true,
                    _ => ()
                }
            },

            Event::RedrawEventsCleared => {
                let next_render = last_frame_start + Duration::from_secs_f64(FRAME_TIME);
                proxy.send_event(UserEvent::RedrawAt(next_render)).unwrap();
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

#[test]
fn test() {
    let a = vec![1, 2, 3, 4];
    let b = vec![1, 2, 4, 5, 6];
    let c = vec![1, 2, 3, 4, 7, 8];

    let oisjdf = [a, b, c].into_iter().flatten().collect::<Vec<_>>();
    dbg!(oisjdf);
}