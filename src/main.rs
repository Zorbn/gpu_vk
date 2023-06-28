mod graphics;

fn main() {
    unsafe {
        let mut graphics = graphics::Graphics::new("GPU VK");
        graphics.run();
    }
}