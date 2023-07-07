mod graphics;

use graphics::{*, texture};

use std::{rc};

struct App {
    time: f32,
    sprite_batch: sprite_batch::SpriteBatch,
    evil_sprite_batch: sprite_batch::SpriteBatch,
}

impl app::App for App {
    fn new(resources: &mut Resources) -> Self {
        let rust_texture = rc::Rc::new(texture::Texture::new(resources, "assets/rust.png"));
        let evil_rust_texture = rc::Rc::new(texture::Texture::new(resources, "assets/evil_rust.png"));

        Self {
            time: 0.0,
            sprite_batch: sprite_batch::SpriteBatch::new(resources, rust_texture),
            evil_sprite_batch: sprite_batch::SpriteBatch::new(resources, evil_rust_texture),
        }
    }

    fn update(&mut self, resources: &mut Resources, delta_time: f32) {
        self.time += delta_time;
        let sprite_position = self.time.sin() * 320.0 + 320.0;

        self.evil_sprite_batch.batch(&[
            sprite_batch::Sprite {
                x: sprite_position,
                y: 16.0,
                z: 1.0,
                width: 64.0,
                height: 32.0,
            },
            sprite_batch::Sprite {
                x: sprite_position + 16.0,
                y: 32.0,
                z: -1.0,
                width: 128.0,
                height: 64.0,
            },
        ]);

        if self.time as usize % 2 == 0 {
            self.sprite_batch.batch(&[
                sprite_batch::Sprite {
                    x: sprite_position,
                    y: 0.0,
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
        self.sprite_batch.draw(&draw);
        self.evil_sprite_batch.draw(&draw);
    }
}

fn main() {
    unsafe {
        let mut graphics = Graphics::new("GPU VK");
        graphics.run::<App>();
    }
}
