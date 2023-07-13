use super::*;

pub trait App {
    fn new(resources: &mut Resources) -> Self;
    fn update(&mut self, resources: &mut Resources, input: &mut Input, delta_time: f32);
    fn draw(&mut self, draw: &Draw);
}
