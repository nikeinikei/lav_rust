use bytemuck::{Pod, Zeroable};

pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct Vertex {
    pub position: [f32; 2],
}

pub struct DrawCommand {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

pub trait GraphicsBackend {
    fn request_swapchain_recreation(&mut self);
    fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32);
    fn present(&mut self, draw_command: DrawCommand);
}

pub struct Graphics<T: GraphicsBackend> {
    backend: T,
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl<T: GraphicsBackend> Graphics<T> {
    pub fn new(graphics_backend: T) -> Graphics<T> {
        Graphics {
            backend: graphics_backend,
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn request_swapchain_recreation(&mut self) {
        self.backend.request_swapchain_recreation();
    }

    pub fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.backend.set_clear_color(r, g, b, a);
    }

    pub fn rectangle(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let start = self.vertices.len() as u32;

        self.vertices.push(Vertex { position: [x, y] });
        self.vertices.push(Vertex {
            position: [x, y + h],
        });
        self.vertices.push(Vertex {
            position: [x + w, y],
        });
        self.vertices.push(Vertex {
            position: [x + w, y + h],
        });

        self.indices.push(start);
        self.indices.push(start + 1);
        self.indices.push(start + 2);
        self.indices.push(start + 2);
        self.indices.push(start + 1);
        self.indices.push(start + 3);
    }

    pub fn present(&mut self) {
        let draw_command = DrawCommand {
            vertices: self.vertices.clone(),
            indices: self.indices.clone(),
        };

        self.backend.present(draw_command);

        self.vertices.clear();
        self.indices.clear();
    }
}
