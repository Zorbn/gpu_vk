use super::*;

pub trait App {
    fn new(graphics: &mut Graphics) -> Self;
    fn update(&mut self, graphics: &mut Graphics, delta_time: f32);
    fn draw(&mut self, draw: &Draw);
}
