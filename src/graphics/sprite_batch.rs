use std::{rc, mem};

use ash::vk;

use super::{*, vk_base::*, vk_resources::*};

#[derive(Clone, Debug, Copy)]
pub struct Vertex {
    pos: [f32; 3],
    uv: [f32; 2],
}

impl Vertex {
    pub fn get_info() -> (
        [vk::VertexInputBindingDescription; 1],
        [vk::VertexInputAttributeDescription; 2],
    ) {
        let vertex_input_binding_descriptions = [vk::VertexInputBindingDescription {
            binding: 0,
            stride: mem::size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }];
        let vertex_input_attribute_descriptions = [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: crate::offset_of!(Vertex, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: crate::offset_of!(Vertex, uv) as u32,
            },
        ];

        (
            vertex_input_binding_descriptions,
            vertex_input_attribute_descriptions,
        )
    }
}

pub struct Sprite {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub width: f32,
    pub height: f32,
}

pub struct SpriteBatch {
    device_data: rc::Rc<device_data::DeviceData>,
    index_buffer: Option<buffer::Buffer>,
    vertex_buffer: Option<buffer::Buffer>,
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl SpriteBatch {
    pub fn new(resources: &Resources) -> Self {
        Self {
            device_data: resources.base.device_data.clone(),
            index_buffer: None,
            vertex_buffer: None,
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn batch(&mut self, sprites: &[Sprite]) {
        self.vertices.clear();
        self.indices.clear();

        for sprite in sprites {
            let vertex_count = self.vertices.len() as u32;

            self.vertices.push(Vertex {
                pos: [sprite.x, sprite.y, sprite.z],
                uv: [0.0, 0.0],
            });
            self.vertices.push(Vertex {
                pos: [sprite.x, sprite.y + sprite.height, sprite.z],
                uv: [0.0, 1.0],
            });
            self.vertices.push(Vertex {
                pos: [sprite.x + sprite.width, sprite.y + sprite.height, sprite.z],
                uv: [1.0, 1.0],
            });
            self.vertices.push(Vertex {
                pos: [sprite.x + sprite.width, sprite.y, sprite.z],
                uv: [1.0, 0.0],
            });

            self.indices.push(vertex_count);
            self.indices.push(vertex_count + 1);
            self.indices.push(vertex_count + 2);
            self.indices.push(vertex_count + 2);
            self.indices.push(vertex_count + 3);
            self.indices.push(vertex_count);
        }

        unsafe {
            self.device_data.device.device_wait_idle().unwrap();

            // Buffers can't be initialized with no data, so if either buffer
            // has no data (ie: because the sprites slice is empty) then return.
            if self.indices.is_empty() || self.vertices.is_empty() {
                self.index_buffer = None;
                self.vertex_buffer = None;
                return;
            }

            self.index_buffer = Some(buffer::Buffer::new(
                &self.indices,
                self.device_data.clone(),
                vk::BufferUsageFlags::INDEX_BUFFER,
            ));

            self.vertex_buffer = Some(buffer::Buffer::new(
                &self.vertices,
                self.device_data.clone(),
                vk::BufferUsageFlags::VERTEX_BUFFER,
            ));
        }
    }

    pub fn draw(&self, device: &ash::Device, command_buffer: vk::CommandBuffer) {
        if self.vertex_buffer.is_none() || self.index_buffer.is_none() {
            return;
        }

        let vertex_buffer = self.vertex_buffer.as_ref().unwrap();
        let index_buffer = self.index_buffer.as_ref().unwrap();

        unsafe {
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer.vk_buffer()], &[0]);
            device.cmd_bind_index_buffer(
                command_buffer,
                index_buffer.vk_buffer(),
                0,
                vk::IndexType::UINT32,
            );
            device.cmd_draw_indexed(command_buffer, index_buffer.len() as u32, 1, 0, 0, 1);
        }
    }
}
