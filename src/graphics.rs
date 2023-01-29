use std::ops::Mul;

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
pub struct Matrix4 {
    pub data: [f32; 16],
}

impl Matrix4 {
    pub fn get_index(i: usize, j: usize) -> usize {
        i * 4 + j
    }

    pub fn get_value(&self, i: usize, j: usize) -> f32 {
        self.data[Matrix4::get_index(i, j)]
    }

    #[rustfmt::skip]
    pub fn identity() -> Matrix4 {
        return Matrix4 {
            data: [
                1_f32, 0_f32, 0_f32, 0_f32,
                0_f32, 1_f32, 0_f32, 0_f32,
                0_f32, 0_f32, 1_f32, 0_f32,
                0_f32, 0_f32, 0_f32, 1_f32,
            ],
        };
    }

    #[rustfmt::skip]
    pub fn rotation(angle: f32) -> Matrix4 {
        Matrix4 {
            data: [
                angle.cos(),    -angle.sin(),   0_f32, 0_f32,
                angle.sin(),     angle.cos(),   0_f32, 0_f32,
                0_f32,          0_f32,          1_f32, 0_f32,
                0_f32,          0_f32,          0_f32, 1_f32,
            ]
        }
    }

    #[rustfmt::skip]
    pub fn translation(x: f32, y: f32, z: f32) -> Matrix4 {
        Matrix4 {
            data: [
                1_f32, 0_f32, 0_f32, x,
                0_f32, 1_f32, 0_f32, y,
                0_f32, 0_f32, 1_f32, z,
                0_f32, 0_f32, 0_f32, 1_f32,
            ]
        }
    }

    pub fn transposed(&self) -> Matrix4 {
        let mut data = [0_f32; 16];

        for i in 0..4 {
            for j in 0..4 {
                data[Matrix4::get_index(i, j)] = self.get_value(j, i);
            }
        }

        Matrix4 {
            data,
        }
    }
}

impl Mul for Matrix4 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut values = [0_f32; 16];

        for i in 0..4 {
            for j in 0..4 {
                let mut sum = 0_f32;
                for k in 0..4 {
                    sum += self.get_value(i, k) * rhs.get_value(k, j);
                }
                values[Matrix4::get_index(i, j)] = sum;
            }
        }

        Matrix4 {
            data: values
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct Vertex {
    pub position: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
pub struct PushValues {
    projection: Matrix4,
    transformation: Matrix4,
    color: Color,
}

#[derive(Clone)]
pub struct DrawCommand {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub push_values: PushValues,
}

pub trait GraphicsBackend {
    fn request_swapchain_recreation(&mut self);
    fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32);
    fn present(&mut self, draw_commands: Vec<DrawCommand>);
}

pub struct Graphics<T: GraphicsBackend> {
    backend: T,
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    transformation_stack: Vec<Matrix4>,
    draw_commands: Vec<DrawCommand>,
    color: Color,
}

impl<T: GraphicsBackend> Graphics<T> {
    pub fn new(graphics_backend: T) -> Graphics<T> {
        let transformation_stack = vec![Matrix4::identity()];

        Graphics {
            backend: graphics_backend,
            vertices: Vec::new(),
            indices: Vec::new(),
            transformation_stack,
            draw_commands: Vec::new(),
            color: Color { r: 1_f32, g: 1_f32, b: 1_f32, a: 1_f32 },
        }
    }

    pub fn origin(&mut self) {
        self.flush_batched_draws();

        self.transformation_stack.pop();
        self.transformation_stack.push(Matrix4::identity());
    }

    pub fn translate(&mut self, x: f32, y: f32) {
        self.flush_batched_draws();

        let current = self.transformation_stack.pop().unwrap();
        self.transformation_stack.push(current * Matrix4::translation(x, y, 0_f32));
    }

    pub fn rotate(&mut self, r: f32) {
        self.flush_batched_draws();

        let current = self.transformation_stack.pop().unwrap();
        self.transformation_stack.push(current * Matrix4::rotation(r));
    }

    pub fn flush_batched_draws(&mut self) {
        if self.vertices.len() > 0 {
            let indices = self.indices.clone();
            let vertices = self.vertices.clone();
            let transformation = self.transformation_stack.last().unwrap().transposed();

            self.indices.clear();
            self.vertices.clear();

            let push_values = PushValues {
                projection: Matrix4::identity(),
                transformation,
                color: self.color.clone(),
            };

            let draw_command = DrawCommand {
                indices,
                vertices,
                push_values,
            };

            self.draw_commands.push(draw_command);
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
        self.flush_batched_draws();

        let draw_commands = self.draw_commands.clone();
        self.draw_commands.clear();

        self.backend.present(draw_commands);
    }
}
