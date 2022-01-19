use glam::Vec2;
use std::collections::HashMap;
use winit::dpi::PhysicalPosition;
use winit::event::WindowEvent;
use winit::event::{ElementState, VirtualKeyCode};

pub struct Input {
    key_states: HashMap<VirtualKeyCode, bool>,
    prev_key_states: HashMap<VirtualKeyCode, bool>,
    pub mouse_pos: PhysicalPosition<f64>,
    pub mouse_delta: Vec2,
    pub right_mouse_down: bool,
}

impl Default for Input {
    fn default() -> Input {
        Input {
            key_states: HashMap::new(),
            prev_key_states: HashMap::new(),
            mouse_pos: PhysicalPosition { x: 0.0, y: 0.0 },
            mouse_delta: Vec2::new(0.0, 0.0),
            right_mouse_down: false,
        }
    }
}

impl Input {
    pub fn update(&mut self, events: &[WindowEvent]) {
        self.prev_key_states = self.key_states.clone();
        let prev_mouse_pos = self.mouse_pos;

        for event in events {
            if let WindowEvent::KeyboardInput { input, .. } = event {
                if let Some(vk) = input.virtual_keycode {
                    if input.state == ElementState::Pressed {
                        self.key_states.entry(vk).or_insert(true);
                    } else {
                        self.key_states.remove(&vk);
                    }
                }
            }
            if let WindowEvent::CursorMoved { position, .. } = event {
                self.mouse_pos = *position;
            }
            if let WindowEvent::MouseInput { state, button, .. } = event {
                if *button == winit::event::MouseButton::Right && *state == ElementState::Pressed {
                    self.right_mouse_down = true;
                }
                if *button == winit::event::MouseButton::Right && *state == ElementState::Released {
                    self.right_mouse_down = false;
                }
            }
        }

        // Ignore big delta in the first call
        if prev_mouse_pos.x != 0.0 && prev_mouse_pos.y != 0.0 {
            self.mouse_delta = Vec2::new(
                (self.mouse_pos.x - prev_mouse_pos.x) as f32,
                (self.mouse_pos.y - prev_mouse_pos.y) as f32,
            );
        }
    }

    pub fn key_pressed(&self, key: VirtualKeyCode) -> bool {
        self.key_states.contains_key(&key) && !self.prev_key_states.contains_key(&key)
    }

    pub fn key_down(&self, key: VirtualKeyCode) -> bool {
        self.key_states.contains_key(&key)
    }

    pub fn right_mouse_down(&self) -> bool {
        self.right_mouse_down
    }
}
