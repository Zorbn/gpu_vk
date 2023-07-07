mod graphics;

use graphics::*;

struct App {
    time: f32,
    sprite_batch: sprite_batch::SpriteBatch,
}

impl app::App for App {
    fn new(resources: &mut Resources) -> Self {
        Self {
            time: 0.0,
            sprite_batch: sprite_batch::SpriteBatch::new(resources),
        }
    }

    fn update(&mut self, resources: &mut Resources, delta_time: f32) {
        self.time += delta_time;
        let sprite_position = self.time.sin() * 320.0 + 320.0;

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
        draw.sprite_batch(&self.sprite_batch);
    }
}

fn main() {
    unsafe {
        let mut graphics = Graphics::new("GPU VK");
        graphics.run::<App>();
    }
}
