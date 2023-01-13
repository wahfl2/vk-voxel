use rustc_data_structures::stable_map::FxHashMap;
use ultraviolet::{Vec3, Vec2};
use winit::{event::{VirtualKeyCode, DeviceEvent, KeyboardInput, ElementState}, event_loop::EventLoopProxy};

#[derive(Clone, Debug)]
pub enum InputHandlerEvent {
    Movement(Vec3),
    MouseMovement(Vec2),
}

pub struct InputHandler {
    pub key_press_map: FxHashMap<VirtualKeyCode, bool>,
}

impl InputHandler {
    pub fn new() -> Self {
        Self { key_press_map: FxHashMap::default() }
    }

    pub fn handle_event(&mut self, event: DeviceEvent, proxy: &mut EventLoopProxy<InputHandlerEvent>) {
        if let DeviceEvent::Key(
            KeyboardInput { virtual_keycode: Some(key), state, .. }
        ) = event {
            self.key_press_map.insert(
                key, 
                match state {
                    ElementState::Pressed => true,
                    ElementState::Released => false,
                }
            );
        }

        if let DeviceEvent::MouseMotion { delta } = event {
            proxy.send_event(
                InputHandlerEvent::MouseMovement(Vec2::new(delta.0 as f32, delta.1 as f32))
            ).unwrap();
        }
    }

    pub fn is_pressed(&self, key: VirtualKeyCode) -> bool {
        match self.key_press_map.get(&key) {
            Some(state) => *state,
            None => false,
        }
    }
}