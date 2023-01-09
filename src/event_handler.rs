use rustc_data_structures::stable_map::FxHashMap;
use ultraviolet::{IVec2, Vec3};
use winit::event::{VirtualKeyCode, DeviceEvent, KeyboardInput, ElementState};

pub enum InputHandlerEvent {
    Movement(Vec3),
    Cursor(IVec2),
}

pub struct InputHandler {
    pub key_press_map: FxHashMap<VirtualKeyCode, bool>,
}

impl InputHandler {
    pub fn new() -> Self {
        Self { key_press_map: FxHashMap::default() }
    }

    pub fn handle_event(&mut self, event: DeviceEvent) {
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
    }

    pub fn is_pressed(&self, key: VirtualKeyCode) -> bool {
        match self.key_press_map.get(&key) {
            Some(state) => *state,
            None => false,
        }
    }
}