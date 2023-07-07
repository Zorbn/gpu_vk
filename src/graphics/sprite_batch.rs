use std::{io, mem, rc};

use ash::vk;

use super::{texture, vk_base::*, vk_resources::*, *};

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

    vertex_shader_module: vk::ShaderModule,
    fragment_shader_module: vk::ShaderModule,

    pipelines: Vec<vk::Pipeline>,
    pipeline_layout: vk::PipelineLayout,

    descriptor_sets: Vec<vk::DescriptorSet>,
    descriptor_set_layouts: [vk::DescriptorSetLayout; 1],
    descriptor_pool: vk::DescriptorPool,

    texture: rc::Rc<texture::Texture>,
}

impl SpriteBatch {
    pub fn new(resources: &Resources, texture: rc::Rc<texture::Texture>) -> Self {
        unsafe {
            let descriptor_sizes = [
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: 1,
                },
            ];
            let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&descriptor_sizes)
                .max_sets(1)
                .build();
            let descriptor_pool = resources
                .base
                .device_data
                .device
                .create_descriptor_pool(&descriptor_pool_info, None)
                .unwrap();
            let descriptor_layout_bindings = [
                vk::DescriptorSetLayoutBinding {
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    stage_flags: vk::ShaderStageFlags::FRAGMENT,
                    ..Default::default()
                },
                vk::DescriptorSetLayoutBinding {
                    binding: 1,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: 1,
                    stage_flags: vk::ShaderStageFlags::FRAGMENT,
                    ..Default::default()
                },
                vk::DescriptorSetLayoutBinding {
                    binding: 2,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    stage_flags: vk::ShaderStageFlags::VERTEX,
                    ..Default::default()
                },
            ];
            let descriptor_info = vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&descriptor_layout_bindings)
                .build();

            let descriptor_set_layouts = [resources
                .base
                .device_data
                .device
                .create_descriptor_set_layout(&descriptor_info, None)
                .unwrap()];

            let descriptor_alloc_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&descriptor_set_layouts)
                .build();
            let descriptor_sets = resources
                .base
                .device_data
                .device
                .allocate_descriptor_sets(&descriptor_alloc_info)
                .unwrap();

            // TODO: Make shader a seperate resoure that gets passed in here?
            let mut vertex_spv_file =
                io::Cursor::new(&include_bytes!("../../shader/texture.vert.spv")[..]);
            let mut frag_spv_file =
                io::Cursor::new(&include_bytes!("../../shader/texture.frag.spv")[..]);

            let vertex_code =
                read_spv(&mut vertex_spv_file).expect("Failed to read vertex shader spv file");
            let vertex_shader_info = vk::ShaderModuleCreateInfo::builder()
                .code(&vertex_code)
                .build();

            let frag_code =
                read_spv(&mut frag_spv_file).expect("Failed to read fragment shader spv file");
            let frag_shader_info = vk::ShaderModuleCreateInfo::builder()
                .code(&frag_code)
                .build();

            let vertex_shader_module = resources
                .base
                .device_data
                .device
                .create_shader_module(&vertex_shader_info, None)
                .expect("Failed to create vertex shader module");

            let fragment_shader_module = resources
                .base
                .device_data
                .device
                .create_shader_module(&frag_shader_info, None)
                .expect("Failed to create fragment shader module");

            let layout_create_info = vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&descriptor_set_layouts)
                .build();

            let pipeline_layout = resources
                .base
                .device_data
                .device
                .create_pipeline_layout(&layout_create_info, None)
                .unwrap();

            let shader_entry_name = CStr::from_bytes_with_nul_unchecked(b"main\0");
            let shader_stage_create_infos = [
                vk::PipelineShaderStageCreateInfo {
                    module: vertex_shader_module,
                    p_name: shader_entry_name.as_ptr(),
                    stage: vk::ShaderStageFlags::VERTEX,
                    ..Default::default()
                },
                vk::PipelineShaderStageCreateInfo {
                    module: fragment_shader_module,
                    p_name: shader_entry_name.as_ptr(),
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    ..Default::default()
                },
            ];

            let viewport_state_info = vk::PipelineViewportStateCreateInfo::builder()
                .scissors(&resources.scissors)
                .viewports(&resources.viewports)
                .build();

            let rasterization_info = vk::PipelineRasterizationStateCreateInfo {
                front_face: vk::FrontFace::COUNTER_CLOCKWISE,
                line_width: 1.0,
                polygon_mode: vk::PolygonMode::FILL,
                ..Default::default()
            };

            let multisample_state_info = vk::PipelineMultisampleStateCreateInfo::builder()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1)
                .build();

            let noop_stencil_state = vk::StencilOpState {
                fail_op: vk::StencilOp::KEEP,
                pass_op: vk::StencilOp::KEEP,
                depth_fail_op: vk::StencilOp::KEEP,
                compare_op: vk::CompareOp::ALWAYS,
                ..Default::default()
            };
            let depth_state_info = vk::PipelineDepthStencilStateCreateInfo {
                depth_test_enable: 1,
                depth_write_enable: 1,
                depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
                front: noop_stencil_state,
                back: noop_stencil_state,
                max_depth_bounds: 1.0,
                ..Default::default()
            };

            let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
                blend_enable: 0,
                src_color_blend_factor: vk::BlendFactor::SRC_COLOR,
                dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_DST_COLOR,
                color_blend_op: vk::BlendOp::ADD,
                src_alpha_blend_factor: vk::BlendFactor::ZERO,
                dst_alpha_blend_factor: vk::BlendFactor::ZERO,
                alpha_blend_op: vk::BlendOp::ADD,
                color_write_mask: vk::ColorComponentFlags::RGBA,
            }];
            let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
                .logic_op(vk::LogicOp::CLEAR)
                .attachments(&color_blend_attachment_states)
                .build();

            let dynamic_state = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
            let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::builder()
                .dynamic_states(&dynamic_state)
                .build();

            let (vertex_input_binding_descriptions, vertex_input_attribute_descriptions) =
                Vertex::get_info();
            let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_attribute_descriptions(&vertex_input_attribute_descriptions)
                .vertex_binding_descriptions(&vertex_input_binding_descriptions)
                .build();

            let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
                topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                ..Default::default()
            };

            let graphic_pipeline_infos = vk::GraphicsPipelineCreateInfo::builder()
                .stages(&shader_stage_create_infos)
                .vertex_input_state(&vertex_input_state_info)
                .input_assembly_state(&vertex_input_assembly_state_info)
                .viewport_state(&viewport_state_info)
                .rasterization_state(&rasterization_info)
                .multisample_state(&multisample_state_info)
                .depth_stencil_state(&depth_state_info)
                .color_blend_state(&color_blend_state)
                .dynamic_state(&dynamic_state_info)
                .layout(pipeline_layout)
                .render_pass(resources.render_pass.vk_render_pass)
                .build();

            let pipelines = resources
                .base
                .device_data
                .device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[graphic_pipeline_infos],
                    None,
                )
                .unwrap();

            Self {
                device_data: resources.base.device_data.clone(),

                index_buffer: None,
                vertex_buffer: None,
                vertices: Vec::new(),
                indices: Vec::new(),

                vertex_shader_module,
                fragment_shader_module,

                pipelines,
                pipeline_layout,

                descriptor_sets,
                descriptor_set_layouts,
                descriptor_pool,

                texture,
            }
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

    pub fn draw(&self, draw: &Draw) {
        if self.vertex_buffer.is_none() || self.index_buffer.is_none() {
            return;
        }

        let vertex_buffer = self.vertex_buffer.as_ref().unwrap();
        let index_buffer = self.index_buffer.as_ref().unwrap();

        unsafe {
            let write_descriptor_sets = [
                vk::WriteDescriptorSet {
                    dst_set: self.descriptor_sets[0],
                    dst_binding: 1,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    p_image_info: &self.texture.descriptor,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    dst_set: self.descriptor_sets[0],
                    dst_binding: 2,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    p_buffer_info: &draw.resources.projection_matrix_buffer_descriptor,
                    ..Default::default()
                },
            ];

            draw.device
                .update_descriptor_sets(&write_descriptor_sets, &[]);

            draw.device.cmd_bind_descriptor_sets(
                draw.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &self.descriptor_sets[..],
                &[],
            );
            draw.device.cmd_bind_pipeline(
                draw.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipelines[0],
            );
            draw.device
                .cmd_set_viewport(draw.command_buffer, 0, &draw.resources.viewports);
            draw.device
                .cmd_set_scissor(draw.command_buffer, 0, &draw.resources.scissors);

            draw.device.cmd_bind_vertex_buffers(
                draw.command_buffer,
                0,
                &[vertex_buffer.vk_buffer()],
                &[0],
            );
            draw.device.cmd_bind_index_buffer(
                draw.command_buffer,
                index_buffer.vk_buffer(),
                0,
                vk::IndexType::UINT32,
            );
            draw.device.cmd_draw_indexed(
                draw.command_buffer,
                index_buffer.len() as u32,
                1,
                0,
                0,
                1,
            );
        }
    }
}

impl Drop for SpriteBatch {
    fn drop(&mut self) {
        unsafe {
            self.device_data.device.device_wait_idle().unwrap();

            for pipeline in &self.pipelines {
                self.device_data.device.destroy_pipeline(*pipeline, None);
            }
            self.device_data
                .device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device_data
                .device
                .destroy_shader_module(self.vertex_shader_module, None);
            self.device_data
                .device
                .destroy_shader_module(self.fragment_shader_module, None);
            for &descriptor_set_layout in self.descriptor_set_layouts.iter() {
                self.device_data
                    .device
                    .destroy_descriptor_set_layout(descriptor_set_layout, None);
            }
            self.device_data
                .device
                .destroy_descriptor_pool(self.descriptor_pool, None);
        }
    }
}
