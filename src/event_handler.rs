use std::time::Instant;

use ahash::HashMap;
use ultraviolet::{Vec3, Vec2};
use winit::{event::{VirtualKeyCode, DeviceEvent, KeyboardInput, ElementState}, event_loop::EventLoopProxy};

#[derive(Clone, Debug)]
pub enum UserEvent {
    InputHandler(InputHandlerEvent),
    RedrawAt(Instant),
}

#[derive(Clone, Debug)]
pub enum InputHandlerEvent {
    Movement(Vec3),
    MouseMovement(Vec2),
}

pub struct InputHandler {
    pub key_press_map: HashMap<VirtualKeyCode, bool>,
    pub mouse_delta: Vec2,
}

impl InputHandler {
    pub fn new() -> Self {
        Self { 
            key_press_map: HashMap::default(),
            mouse_delta: Vec2::zero(),
        }
    }

    pub fn update(&mut self) {
        
    }

    pub fn handle_event(&mut self, event: DeviceEvent, proxy: &mut EventLoopProxy<UserEvent>) {
        match event {
            // Mouse movement
            DeviceEvent::MouseMotion { delta } => {
                let (dx, dy) = delta;
                self.mouse_delta += Vec2::new(dx as f32, dy as f32);
            }, 

            // Key press/release
            DeviceEvent::Key(
                KeyboardInput { virtual_keycode: Some(key), state, .. }
            ) => {
                self.key_press_map.insert(
                    key, 
                    match state {
                        ElementState::Pressed => true,
                        ElementState::Released => false,
                    }
                );
            }

            _ => ()
        }

        // if let DeviceEvent::MouseMotion { delta } = event {
        //     proxy.send_event(
        //         InputHandlerEvent::MouseMovement(Vec2::new(delta.0 as f32, delta.1 as f32))
        //     ).unwrap();
        // }
    }

    pub fn is_pressed(&self, key: VirtualKeyCode) -> bool {
        match self.key_press_map.get(&key) {
            Some(state) => *state,
            None => false,
        }
    }
}