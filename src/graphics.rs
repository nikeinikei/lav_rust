pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

pub trait GraphicsBackend {
    fn request_swapchain_recreation(&mut self);
    fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32);
    fn present(&mut self);
}

pub struct Graphics<T: GraphicsBackend> {
    backend: T,
}

impl <T: GraphicsBackend> Graphics<T> {
    pub fn new(graphics_backend: T) -> Graphics<T> {
        Graphics {
            backend: graphics_backend
        }
    }

    pub fn request_swapchain_recreation(&mut self) {
        self.backend.request_swapchain_recreation();
    }

    pub fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.backend.set_clear_color(r, g, b, a);
    }

    pub fn present(&mut self) {
        self.backend.present();
    }
}
