use super::*;

pub trait App {
    fn new() -> Self;
    fn update(&mut self, delta_time: f32, sprite_batch: &mut sprite_batch::SpriteBatch);
}
