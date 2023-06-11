#![feature(slice_as_chunks)]
#![feature(slice_flatten)]
#![feature(portable_simd)]
#![feature(associated_const_equality)]
#![feature(fn_traits)]

use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use event_handler::{InputHandler, UserEvent};
use mimalloc::MiMalloc;
use render::{
    fps_log::FpsLog,
    renderer::Renderer,
    util::{GetWindow, RenderState},
};

use crate::util::util::AdditionalSwizzles;
use server::server::Server;
use ultraviolet::Vec2;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
};
use world::{block_data::StaticBlockData, world_blocks::WorldBlocks};

pub mod event_handler;
pub mod physics;
pub mod render;
pub mod server;
pub mod util;
pub mod world;

pub const FRAME_TIME: f64 = 0.0;

#[global_allocator]
pub static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    let event_loop: EventLoop<UserEvent> = EventLoopBuilder::with_user_event().build();
    let mut proxy = event_loop.create_proxy();

    let texture_atlas = Renderer::load_texture_folder_into_atlas("./resources");
    let mut static_block_data = StaticBlockData::empty();
    static_block_data.init(&texture_atlas);

    let static_block_data = Arc::new(static_block_data);

    let mut renderer = Renderer::new(&event_loop, texture_atlas, &static_block_data);
    let world_blocks = Arc::new(Mutex::new(WorldBlocks::new(&static_block_data)));

    thread::spawn({
        let world_blocks = world_blocks.clone();
        let static_block_data = static_block_data.clone();
        move || loop {
            let mut lock = world_blocks.lock().unwrap();
            lock.frame_update(&static_block_data);
            drop(lock);

            thread::sleep(Duration::from_micros(100));
        }
    });

    let mut server = Server::new();
    server.init_single_player();

    let mut input_handler = InputHandler::new();
    let mut fps_log = FpsLog::new();

    let mut window_resized = false;
    let mut recreate_swapchain = false;
    let mut last_frame_start = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        let world_blocks = world_blocks.clone();

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

                let camera = server.camera();

                let mut world_blocks_lock = world_blocks.lock().unwrap();
                world_blocks_lock.player_pos = -camera.pos.xz();
                server.tick(
                    delta_time,
                    &input_handler,
                    &world_blocks_lock,
                    &static_block_data,
                );
                drop(world_blocks_lock);

                let camera = server.camera();
                renderer.cam_uniform = Some(camera.calculate_matrix());

                input_handler.mouse_delta = Vec2::zero();

                match renderer.render(world_blocks.clone(), &static_block_data) {
                    RenderState::OutOfDate | RenderState::Suboptimal => recreate_swapchain = true,
                    _ => (),
                }
            }

            Event::RedrawEventsCleared => {
                let next_render = last_frame_start + Duration::from_secs_f64(FRAME_TIME);
                proxy.send_event(UserEvent::RedrawAt(next_render)).unwrap();
            }

            Event::DeviceEvent { event, .. } => {
                input_handler.handle_event(event, &mut proxy);
            }

            Event::UserEvent(ev) => {
                // This `match` has only two possible returns. Thus, an `if let` would be better here.
                // I am also getting a warning for possibly being able to collapse this into the match statement.
                if let UserEvent::RedrawAt(instant) = ev {
                    if Instant::now() >= instant {
                        last_frame_start = Instant::now();
                        renderer.vk_surface.get_window().unwrap().request_redraw();
                    } else {
                        proxy.send_event(UserEvent::RedrawAt(instant)).unwrap();
                    }
                }
            }

            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => window_resized = true,
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => (),
        }
    });
}

#[cfg(test)]
mod test {
    use std::{hint::black_box, println, time::Instant};

    use rand_xoshiro::{
        rand_core::{RngCore, SeedableRng},
        Xoshiro128PlusPlus,
    };
    use ultraviolet::IVec2;

    #[test]
    fn speed_test() {
        let mut rng = Xoshiro128PlusPlus::seed_from_u64(0);
        const NUMS: usize = 32;
        const ROWS: usize = 1024;

        let mut vec = Vec::with_capacity(ROWS);
        for _ in 0..ROWS {
            let mut row = Vec::with_capacity(NUMS);
            for _ in 0..NUMS {
                row.push(rng.next_u32());
            }
            vec.push(row);
        }

        let mut out = Vec::new();
        let start = Instant::now();
        for (row, y) in vec.into_iter().zip(-512..512) {
            for (num, j) in row.into_iter().zip(-16..16) {
                let j = j * 32;
                for k in 0..32 {
                    let shift = 31 - k;
                    if (num >> shift) & 1 > 0 {
                        let x = j + k;
                        out.push(IVec2::new(x, y));
                    }
                }
            }
        }
        let time_took = Instant::now() - start;
        black_box(out);

        println!("Took {}ms", time_took.as_secs_f32() * 1000.0);
    }
}
