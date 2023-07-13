mod graphics;

use graphics::{texture, *};

use winit::event::VirtualKeyCode;

use std::rc;

const PLAYER_SPEED: f32 = 500.0;

struct App {
    time: f32,
    sprite_batch: sprite_batch::SpriteBatch,
    evil_sprite_batch: sprite_batch::SpriteBatch,
    player_y: f32,
}

impl app::App for App {
    fn new(resources: &mut Resources) -> Self {
        let rust_texture = rc::Rc::new(texture::Texture::new(
            resources,
            "assets/rust.png",
            texture::Filter::Linear,
        ));
        let evil_rust_texture = rc::Rc::new(texture::Texture::new(
            resources,
            "assets/evil_rust.png",
            texture::Filter::Nearest,
        ));

        Self {
            time: 0.0,
            sprite_batch: sprite_batch::SpriteBatch::new(resources, rust_texture),
            evil_sprite_batch: sprite_batch::SpriteBatch::new(resources, evil_rust_texture),
            player_y: 0.0,
        }
    }

    fn update(&mut self, resources: &mut Resources, input: &mut Input, delta_time: f32) {
        self.time += delta_time;

        let mut player_direction = 0.0;

        if input.is_key_held(VirtualKeyCode::Up) {
            player_direction -= 1.0;
        }

        if input.is_key_held(VirtualKeyCode::Down) {
            player_direction += 1.0;
        }

        self.player_y += player_direction * delta_time * PLAYER_SPEED;

        let sprite_position = self.time.sin() * 320.0 + 320.0;

        self.evil_sprite_batch.batch(&[
            sprite_batch::Sprite {
                x: 0.0,
                y: sprite_position + 16.0,
                z: 1.0,
                width: 64.0,
                height: 32.0,
            },
            sprite_batch::Sprite {
                x: 16.0,
                y: sprite_position + 32.0,
                z: -1.0,
                width: 128.0,
                height: 64.0,
            },
        ]);

        if self.time as usize % 2 == 0 {
            self.sprite_batch.batch(&[
                sprite_batch::Sprite {
                    x: sprite_position,
                    y: self.player_y,
                    z: 1.0,
                    width: 64.0,
                    height: 32.0,
                },
                sprite_batch::Sprite {
                    x: sprite_position + 16.0,
                    y: 16.0,
                    z: -1.0,
                    width: 128.0,
                    height: 64.0,
                },
            ]);
        } else {
            self.sprite_batch.batch(&[]);
        }
    }

    fn draw(&mut self, draw: &Draw) {
        self.sprite_batch.draw(draw);
        self.evil_sprite_batch.draw(draw);
    }
}

fn main() {
    unsafe {
        let mut graphics = Graphics::new("GPU VK");
        graphics.run::<App>();
    }
}
