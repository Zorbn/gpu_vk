pub mod app;
pub mod sprite_batch;

mod mat4;
mod vk_base;
mod vk_resources;

use std::{default::Default, ffi::CStr, io::Cursor, mem, time};

use ash::util::*;
use ash::vk;

use vk_base::*;
use vk_resources::*;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

#[derive(Clone, Debug, Copy)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub _pad: f32,
}

pub struct Draw<'a> {
    device: &'a ash::Device,
    command_buffer: vk::CommandBuffer,
}

impl<'a> Draw<'a> {
    pub fn sprite_batch(&self, sprite_batch: &sprite_batch::SpriteBatch) {
        sprite_batch.draw(&self.device, self.command_buffer);
    }
}

pub struct Graphics {
    resources: Resources,

    window: Option<winit::window::Window>,
    event_loop: Option<EventLoop<()>>,

    needs_resize: bool,
}

pub struct Resources {
    render_pass: render_pass::RenderPass,
    viewports: [vk::Viewport; 1],
    scissors: [vk::Rect2D; 1],

    pipelines: Vec<vk::Pipeline>,
    pipeline_layout: vk::PipelineLayout,

    vertex_shader_module: vk::ShaderModule,
    fragment_shader_module: vk::ShaderModule,

    // Keep the uniform buffer alive with the rest of the resources.
    #[allow(dead_code)]
    uniform_color_buffer: buffer::Buffer,

    descriptor_sets: Vec<vk::DescriptorSet>,
    descriptor_set_layouts: [vk::DescriptorSetLayout; 1],
    descriptor_pool: vk::DescriptorPool,

    // The texture needs to stay alive as
    // long as its descriptor is being used.
    #[allow(dead_code)]
    texture: texture::Texture,

    projection_matrix: mat4::Mat4,
    projection_matrix_buffer: buffer::Buffer,
    projection_matrix_buffer_descriptor: vk::DescriptorBufferInfo,

    base: vk_base::VkBase,
}

impl Graphics {
    pub unsafe fn new(title: &str) -> Self {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(winit::dpi::LogicalSize::new(f64::from(640), f64::from(480)))
            .build(&event_loop)
            .unwrap();

        let base = VkBase::new(&window);

        let render_pass = render_pass::RenderPass::new(
            base.device_data.clone(),
            &base.surface_data,
            &base.swapchain_data,
            &window,
        );

        let texture = texture::Texture::new(
            "assets/rust.png",
            base.device_data.clone(),
            &base.command_data,
            &base.sync_data,
        );

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
        let descriptor_pool = base
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

        let descriptor_set_layouts = [base
            .device_data
            .device
            .create_descriptor_set_layout(&descriptor_info, None)
            .unwrap()];

        let descriptor_alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&descriptor_set_layouts)
            .build();
        let descriptor_sets = base
            .device_data
            .device
            .allocate_descriptor_sets(&descriptor_alloc_info)
            .unwrap();

        let uniform_color_buffer_data = Vector3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
            _pad: 0.0,
        };
        let uniform_color_buffer = buffer::Buffer::new(&[uniform_color_buffer_data], base.device_data.clone(), vk::BufferUsageFlags::UNIFORM_BUFFER);
        let uniform_color_buffer_descriptor = vk::DescriptorBufferInfo {
            buffer: uniform_color_buffer.vk_buffer(),
            offset: 0,
            range: mem::size_of_val(&uniform_color_buffer_data) as u64,
        };

        let write_descriptor_sets = [
            vk::WriteDescriptorSet {
                dst_set: descriptor_sets[0],
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                p_buffer_info: &uniform_color_buffer_descriptor,
                ..Default::default()
            },
            vk::WriteDescriptorSet {
                dst_set: descriptor_sets[0],
                dst_binding: 1,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                p_image_info: &texture.descriptor,
                ..Default::default()
            },
        ];
        base.device_data
            .device
            .update_descriptor_sets(&write_descriptor_sets, &[]);

        let mut vertex_spv_file = Cursor::new(&include_bytes!("../../shader/texture.vert.spv")[..]);
        let mut frag_spv_file = Cursor::new(&include_bytes!("../../shader/texture.frag.spv")[..]);

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

        let vertex_shader_module = base
            .device_data
            .device
            .create_shader_module(&vertex_shader_info, None)
            .expect("Failed to create vertex shader module");

        let fragment_shader_module = base
            .device_data
            .device
            .create_shader_module(&frag_shader_info, None)
            .expect("Failed to create fragment shader module");

        let layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&descriptor_set_layouts)
            .build();

        let pipeline_layout = base
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

        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: base.surface_data.resolution.width as f32,
            height: base.surface_data.resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];

        let scissors = [base.surface_data.resolution.into()];
        let viewport_state_info = vk::PipelineViewportStateCreateInfo::builder()
            .scissors(&scissors)
            .viewports(&viewports)
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
            sprite_batch::Vertex::get_info();
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
            .render_pass(render_pass.vk_render_pass)
            .build();

        let pipelines = base
            .device_data
            .device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[graphic_pipeline_infos], None)
            .unwrap();

        let projection_matrix = [0.0; 16];
        let projection_matrix_buffer = buffer::Buffer::new(
            &projection_matrix,
            base.device_data.clone(),
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        );
        let projection_matrix_buffer_descriptor = vk::DescriptorBufferInfo {
            buffer: projection_matrix_buffer.vk_buffer(),
            offset: 0,
            range: mem::size_of_val(&projection_matrix) as u64,
        };

        Self {
            resources: Resources {
                render_pass,
                viewports,
                scissors,

                pipelines,
                pipeline_layout,

                vertex_shader_module,
                fragment_shader_module,

                uniform_color_buffer,

                descriptor_sets,
                descriptor_set_layouts,
                descriptor_pool,

                texture,

                projection_matrix,
                projection_matrix_buffer,
                projection_matrix_buffer_descriptor,

                base,
            },

            window: Some(window),
            event_loop: Some(event_loop),

            needs_resize: true,
        }
    }

    pub unsafe fn run<T: app::App>(&mut self) {
        let window = self.window.take().expect("Tried to run an app without a window, running an app consumes the window it is run with");
        let mut event_loop = self.event_loop.take().expect("Tried to run an app without an event loop, running an app consumes the event loop it is run with");

        let mut app = T::new(&mut self.resources);
        let mut now = time::Instant::now();

        VkBase::render_loop(&window, &mut event_loop, || {
            let delta_time = now.elapsed().as_secs_f32();
            now = time::Instant::now();

            app.update(&mut self.resources, delta_time);

            if self.needs_resize {
                self.needs_resize = false;
                self.resources.base.device_data.device.device_wait_idle().unwrap();

                let window_size = window.inner_size();
                self.resources.base.resize(window_size.width, window_size.height);
                self.resources.render_pass.resize(&window, &self.resources.base.swapchain_data);

                // TODO: Bundle all of this stuff together into a single
                // uniform buffer struct that can be easily updated.
                mat4::orthographic_projection(
                    &mut self.resources.projection_matrix,
                    mat4::OrthographicProjectionInfo {
                        left: 0.0,
                        right: self.resources.base.surface_data.resolution.width as f32,
                        bottom: 0.0,
                        top: self.resources.base.surface_data.resolution.height as f32,
                        z_near: -mat4::Z_VIEW_DISTANCE,
                        z_far: mat4::Z_VIEW_DISTANCE,
                    },
                );
                self.resources.projection_matrix_buffer.set_data(&self.resources.projection_matrix);

                let write_descriptor_sets = [vk::WriteDescriptorSet {
                    dst_set: self.resources.descriptor_sets[0],
                    dst_binding: 2,
                    descriptor_count: 1,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    p_buffer_info: &self.resources.projection_matrix_buffer_descriptor,
                    ..Default::default()
                }];
                self.resources.base.device_data
                    .device
                    .update_descriptor_sets(&write_descriptor_sets, &[]);
            }

            let (present_index, _) = match self.resources.base.swapchain_data.loader.acquire_next_image(
                self.resources.base.swapchain_data.swapchain,
                std::u64::MAX,
                self.resources.base.sync_data.present_complete_semaphore,
                vk::Fence::null(),
            ) {
                Ok(values) => values,
                Err(_) => {
                    self.needs_resize = true;
                    return;
                }
            };

            let clear_values = [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 0.0],
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];

            self.resources.viewports[0].width = self.resources.base.surface_data.resolution.width as f32;
            self.resources.viewports[0].height = self.resources.base.surface_data.resolution.height as f32;
            self.resources.scissors[0].extent.width = self.resources.base.surface_data.resolution.width;
            self.resources.scissors[0].extent.height = self.resources.base.surface_data.resolution.height;

            self.resources.base.device_data.record_submit(
                self.resources.base.command_data.draw_buffer,
                self.resources.base.sync_data.draw_commands_reuse_fence,
                &[vk::PipelineStageFlags::BOTTOM_OF_PIPE],
                &[self.resources.base.sync_data.present_complete_semaphore],
                &[self.resources.base.sync_data.rendering_complete_semaphore],
                |device, command_buffer| {
                    self.resources.render_pass.begin(
                        device,
                        command_buffer,
                        &self.resources.base.surface_data,
                        present_index,
                        &clear_values,
                    );
                    device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.resources.pipeline_layout,
                        0,
                        &self.resources.descriptor_sets[..],
                        &[],
                    );
                    device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.resources.pipelines[0],
                    );
                    device.cmd_set_viewport(command_buffer, 0, &self.resources.viewports);
                    device.cmd_set_scissor(command_buffer, 0, &self.resources.scissors);
                    let draw = Draw{
                        device,
                        command_buffer,
                    };
                    app.draw(&draw);
                    self.resources.render_pass.end(device, command_buffer)
                },
            );
            let present_info = vk::PresentInfoKHR {
                wait_semaphore_count: 1,
                p_wait_semaphores: &self.resources.base.sync_data.rendering_complete_semaphore,
                swapchain_count: 1,
                p_swapchains: &self.resources.base.swapchain_data.swapchain,
                p_image_indices: &present_index,
                ..Default::default()
            };

            match self.resources.base
                .swapchain_data
                .loader
                .queue_present(self.resources.base.device_data.present_queue, &present_info)
            {
                Ok(_) => {}
                Err(_) => {
                    self.needs_resize = true;
                }
            }
        });
    }
}

impl Drop for Resources {
    fn drop(&mut self) {
        unsafe {
            self.base.device_data.device.device_wait_idle().unwrap();

            for pipeline in &self.pipelines {
                self.base.device_data.device.destroy_pipeline(*pipeline, None);
            }
            self.base.device_data
                .device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.base.device_data
                .device
                .destroy_shader_module(self.vertex_shader_module, None);
            self.base.device_data
                .device
                .destroy_shader_module(self.fragment_shader_module, None);
            for &descriptor_set_layout in self.descriptor_set_layouts.iter() {
                self.base.device_data
                    .device
                    .destroy_descriptor_set_layout(descriptor_set_layout, None);
            }
            self.base.device_data
                .device
                .destroy_descriptor_pool(self.descriptor_pool, None);
        }
    }
}
