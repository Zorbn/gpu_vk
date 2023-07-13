use std::{collections::HashSet, hash::Hash};

use winit::event::{ElementState, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent};

struct ButtonSet<T: Copy + Hash + Eq> {
    pressed_buttons: HashSet<T>,
    released_buttons: HashSet<T>,
    held_buttons: HashSet<T>,
}

impl<T: Copy + Hash + Eq> ButtonSet<T> {
    pub fn new() -> Self {
        Self {
            pressed_buttons: HashSet::new(),
            released_buttons: HashSet::new(),
            held_buttons: HashSet::new(),
        }
    }

    pub fn was_button_pressed(&self, button: T) -> bool {
        self.pressed_buttons.contains(&button)
    }

    pub fn was_button_released(&self, button: T) -> bool {
        self.released_buttons.contains(&button)
    }

    pub fn is_button_held(&self, button: T) -> bool {
        self.held_buttons.contains(&button)
    }

    pub fn button_state_changed(&mut self, button: T, state: ElementState) {
        match state {
            ElementState::Pressed => {
                if self.held_buttons.insert(button) {
                    self.pressed_buttons.insert(button);
                }
            }
            ElementState::Released => {
                self.released_buttons.insert(button);
                self.held_buttons.remove(&button);
            }
        }
    }

    pub fn update(&mut self) {
        self.pressed_buttons.clear();
        self.released_buttons.clear();
    }
}

// TODO (involves window): Re-add mouse locking, and also add a way to close the window from user-input.
pub struct Input {
    keys: ButtonSet<VirtualKeyCode>,
    mouse_buttons: ButtonSet<MouseButton>,
    mouse_delta_x: f32,
    mouse_delta_y: f32,
    mouse_x: f32,
    mouse_y: f32,
}

impl Input {
    pub fn new() -> Self {
        Self {
            keys: ButtonSet::new(),
            mouse_buttons: ButtonSet::new(),
            mouse_delta_x: 0.0,
            mouse_delta_y: 0.0,
            mouse_x: 0.0,
            mouse_y: 0.0,
        }
    }

    pub fn process_button(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => {
                self.key_state_changed(*keycode, *state);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.mouse_button_state_changed(*button, *state);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_x = position.x as f32;
                self.mouse_y = position.y as f32;
            }
            _ => return false,
        }

        true
    }

    pub fn process_mouse_motion(&mut self, delta_x: f32, delta_y: f32) {
        self.mouse_moved(delta_x, delta_y);
    }

    pub fn was_key_pressed(&self, keycode: VirtualKeyCode) -> bool {
        self.keys.was_button_pressed(keycode)
    }

    pub fn was_key_released(&self, keycode: VirtualKeyCode) -> bool {
        self.keys.was_button_released(keycode)
    }

    pub fn is_key_held(&self, keycode: VirtualKeyCode) -> bool {
        self.keys.is_button_held(keycode)
    }

    pub fn was_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons.was_button_pressed(button)
    }

    pub fn was_mouse_button_released(&self, button: MouseButton) -> bool {
        self.mouse_buttons.was_button_released(button)
    }

    pub fn is_mouse_button_held(&self, button: MouseButton) -> bool {
        self.mouse_buttons.is_button_held(button)
    }

    pub fn key_state_changed(&mut self, keycode: VirtualKeyCode, state: ElementState) {
        self.keys.button_state_changed(keycode, state);
    }

    pub fn mouse_button_state_changed(&mut self, button: MouseButton, state: ElementState) {
        self.mouse_buttons.button_state_changed(button, state);
    }

    pub fn mouse_moved(&mut self, delta_x: f32, delta_y: f32) {
        self.mouse_delta_x += delta_x;
        self.mouse_delta_y += delta_y;
    }

    pub fn mouse_x(&self) -> f32 {
        self.mouse_x
    }

    pub fn mouse_y(&self) -> f32 {
        self.mouse_y
    }

    pub fn mouse_delta_x(&mut self) -> f32 {
        self.mouse_delta_x
    }

    pub fn mouse_delta_y(&mut self) -> f32 {
        self.mouse_delta_y
    }

    pub fn update(&mut self) {
        self.keys.update();
        self.mouse_buttons.update();
        self.mouse_delta_x = 0.0;
        self.mouse_delta_y = 0.0;
    }
}
