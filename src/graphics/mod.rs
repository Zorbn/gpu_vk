mod vk_base;
mod vk_resources;

use std::default::Default;
use std::ffi::CStr;
use std::io::Cursor;
use std::mem::{self, align_of};
use std::os::raw::c_void;

use ash::util::*;
use ash::vk;

use vk_base::*;
use vk_resources::*;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

#[derive(Clone, Debug, Copy)]
struct Vertex {
    pos: [f32; 4],
    uv: [f32; 2],
}

#[derive(Clone, Debug, Copy)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub _pad: f32,
}

pub unsafe fn run() {
    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Ash")
        .with_inner_size(winit::dpi::LogicalSize::new(f64::from(640), f64::from(480)))
        .build(&event_loop)
        .unwrap();

    let mut base = VkBase::new(&window);

    let mut render_pass = render_pass::RenderPass::new(
        base.device_data.clone(),
        &base.surface_data,
        &base.swapchain_data,
        &window,
    );

    let index_buffer_data = [0u32, 1, 2, 2, 3, 0];
    let index_buffer_info = vk::BufferCreateInfo {
        size: std::mem::size_of_val(&index_buffer_data) as u64,
        usage: vk::BufferUsageFlags::INDEX_BUFFER,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        ..Default::default()
    };
    let index_buffer = base
        .device_data
        .device
        .create_buffer(&index_buffer_info, None)
        .unwrap();
    let index_buffer_memory_req = base
        .device_data
        .device
        .get_buffer_memory_requirements(index_buffer);
    let index_buffer_memory_index = base
        .device_data
        .find_memory_type_index(
            &index_buffer_memory_req,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
        .expect("Unable to find suitable memory type for the index buffer");
    let index_allocate_info = vk::MemoryAllocateInfo {
        allocation_size: index_buffer_memory_req.size,
        memory_type_index: index_buffer_memory_index,
        ..Default::default()
    };
    let index_buffer_memory = base
        .device_data
        .device
        .allocate_memory(&index_allocate_info, None)
        .unwrap();
    let index_ptr: *mut c_void = base
        .device_data
        .device
        .map_memory(
            index_buffer_memory,
            0,
            index_buffer_memory_req.size,
            vk::MemoryMapFlags::empty(),
        )
        .unwrap();
    let mut index_slice = Align::new(
        index_ptr,
        align_of::<u32>() as u64,
        index_buffer_memory_req.size,
    );
    index_slice.copy_from_slice(&index_buffer_data);
    base.device_data.device.unmap_memory(index_buffer_memory);
    base.device_data
        .device
        .bind_buffer_memory(index_buffer, index_buffer_memory, 0)
        .unwrap();

    let vertices = [
        Vertex {
            pos: [-1.0, -1.0, 0.0, 1.0],
            uv: [0.0, 0.0],
        },
        Vertex {
            pos: [-1.0, 1.0, 0.0, 1.0],
            uv: [0.0, 1.0],
        },
        Vertex {
            pos: [1.0, 1.0, 0.0, 1.0],
            uv: [1.0, 1.0],
        },
        Vertex {
            pos: [1.0, -1.0, 0.0, 1.0],
            uv: [1.0, 0.0],
        },
    ];
    let vertex_input_buffer_info = vk::BufferCreateInfo {
        size: std::mem::size_of_val(&vertices) as u64,
        usage: vk::BufferUsageFlags::VERTEX_BUFFER,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        ..Default::default()
    };
    let vertex_input_buffer = base
        .device_data
        .device
        .create_buffer(&vertex_input_buffer_info, None)
        .unwrap();
    let vertex_input_buffer_memory_req = base
        .device_data
        .device
        .get_buffer_memory_requirements(vertex_input_buffer);
    let vertex_input_buffer_memory_index = base
        .device_data
        .find_memory_type_index(
            &vertex_input_buffer_memory_req,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
        .expect("Unable to find suitable memory type for the vertex buffer");

    let vertex_buffer_allocate_info = vk::MemoryAllocateInfo {
        allocation_size: vertex_input_buffer_memory_req.size,
        memory_type_index: vertex_input_buffer_memory_index,
        ..Default::default()
    };
    let vertex_input_buffer_memory = base
        .device_data
        .device
        .allocate_memory(&vertex_buffer_allocate_info, None)
        .unwrap();

    let vert_ptr = base
        .device_data
        .device
        .map_memory(
            vertex_input_buffer_memory,
            0,
            vertex_input_buffer_memory_req.size,
            vk::MemoryMapFlags::empty(),
        )
        .unwrap();
    let mut slice = Align::new(
        vert_ptr,
        align_of::<Vertex>() as u64,
        vertex_input_buffer_memory_req.size,
    );
    slice.copy_from_slice(&vertices);
    base.device_data
        .device
        .unmap_memory(vertex_input_buffer_memory);
    base.device_data
        .device
        .bind_buffer_memory(vertex_input_buffer, vertex_input_buffer_memory, 0)
        .unwrap();

    let uniform_color_buffer_data = Vector3 {
        x: 1.0,
        y: 1.0,
        z: 1.0,
        _pad: 0.0,
    };
    let uniform_color_buffer_info = vk::BufferCreateInfo {
        size: std::mem::size_of_val(&uniform_color_buffer_data) as u64,
        usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        ..Default::default()
    };
    let uniform_color_buffer = base
        .device_data
        .device
        .create_buffer(&uniform_color_buffer_info, None)
        .unwrap();
    let uniform_color_buffer_memory_req = base
        .device_data
        .device
        .get_buffer_memory_requirements(uniform_color_buffer);
    let uniform_color_buffer_memory_index = base
        .device_data
        .find_memory_type_index(
            &uniform_color_buffer_memory_req,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
        .expect("Unable to find suitable memory type for the vertex buffer");

    let uniform_color_buffer_allocate_info = vk::MemoryAllocateInfo {
        allocation_size: uniform_color_buffer_memory_req.size,
        memory_type_index: uniform_color_buffer_memory_index,
        ..Default::default()
    };
    let uniform_color_buffer_memory = base
        .device_data
        .device
        .allocate_memory(&uniform_color_buffer_allocate_info, None)
        .unwrap();
    let uniform_ptr = base
        .device_data
        .device
        .map_memory(
            uniform_color_buffer_memory,
            0,
            uniform_color_buffer_memory_req.size,
            vk::MemoryMapFlags::empty(),
        )
        .unwrap();
    let mut uniform_aligned_slice = Align::new(
        uniform_ptr,
        align_of::<Vector3>() as u64,
        uniform_color_buffer_memory_req.size,
    );
    uniform_aligned_slice.copy_from_slice(&[uniform_color_buffer_data]);
    base.device_data
        .device
        .unmap_memory(uniform_color_buffer_memory);
    base.device_data
        .device
        .bind_buffer_memory(uniform_color_buffer, uniform_color_buffer_memory, 0)
        .unwrap();

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
    let desc_layout_bindings = [
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
    ];
    let descriptor_info = vk::DescriptorSetLayoutCreateInfo::builder()
        .bindings(&desc_layout_bindings)
        .build();

    let desc_set_layouts = [base
        .device_data
        .device
        .create_descriptor_set_layout(&descriptor_info, None)
        .unwrap()];

    let desc_alloc_info = vk::DescriptorSetAllocateInfo::builder()
        .descriptor_pool(descriptor_pool)
        .set_layouts(&desc_set_layouts)
        .build();
    let descriptor_sets = base
        .device_data
        .device
        .allocate_descriptor_sets(&desc_alloc_info)
        .unwrap();

    let uniform_color_buffer_descriptor = vk::DescriptorBufferInfo {
        buffer: uniform_color_buffer,
        offset: 0,
        range: mem::size_of_val(&uniform_color_buffer_data) as u64,
    };

    let write_desc_sets = [
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
        .update_descriptor_sets(&write_desc_sets, &[]);

    let mut vertex_spv_file = Cursor::new(&include_bytes!("../../shader/texture/vert.spv")[..]);
    let mut frag_spv_file = Cursor::new(&include_bytes!("../../shader/texture/frag.spv")[..]);

    let vertex_code =
        read_spv(&mut vertex_spv_file).expect("Failed to read vertex shader spv file");
    let vertex_shader_info = vk::ShaderModuleCreateInfo::builder()
        .code(&vertex_code)
        .build();

    let frag_code = read_spv(&mut frag_spv_file).expect("Failed to read fragment shader spv file");
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
        .set_layouts(&desc_set_layouts)
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
    let vertex_input_binding_descriptions = [vk::VertexInputBindingDescription {
        binding: 0,
        stride: mem::size_of::<Vertex>() as u32,
        input_rate: vk::VertexInputRate::VERTEX,
    }];
    let vertex_input_attribute_descriptions = [
        vk::VertexInputAttributeDescription {
            location: 0,
            binding: 0,
            format: vk::Format::R32G32B32A32_SFLOAT,
            offset: crate::offset_of!(Vertex, pos) as u32,
        },
        vk::VertexInputAttributeDescription {
            location: 1,
            binding: 0,
            format: vk::Format::R32G32_SFLOAT,
            offset: crate::offset_of!(Vertex, uv) as u32,
        },
    ];
    let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_attribute_descriptions(&vertex_input_attribute_descriptions)
        .vertex_binding_descriptions(&vertex_input_binding_descriptions)
        .build();

    let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
        topology: vk::PrimitiveTopology::TRIANGLE_LIST,
        ..Default::default()
    };

    let mut viewports = [vk::Viewport {
        x: 0.0,
        y: 0.0,
        width: base.surface_data.resolution.width as f32,
        height: base.surface_data.resolution.height as f32,
        min_depth: 0.0,
        max_depth: 1.0,
    }];

    let mut scissors = [base.surface_data.resolution.into()];
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

    let graphics_pipelines = base
        .device_data
        .device
        .create_graphics_pipelines(vk::PipelineCache::null(), &[graphic_pipeline_infos], None)
        .unwrap();

    let graphic_pipeline = graphics_pipelines[0];

    VkBase::render_loop(&window, &mut event_loop, || {
        let (present_index, _) = match base.swapchain_data.loader.acquire_next_image(
            base.swapchain_data.swapchain,
            std::u64::MAX,
            base.sync_data.present_complete_semaphore,
            vk::Fence::null(),
        ) {
            Ok(values) => values,
            Err(_) => {
                // TODO: This is a duplicate of the same check at the bottom of the loop:
                base.device_data.device.device_wait_idle().unwrap();
                let window_size = window.inner_size();
                base.resize(window_size.width, window_size.height);
                render_pass.resize(&window, &base.swapchain_data);
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

        viewports[0].width = base.surface_data.resolution.width as f32;
        viewports[0].height = base.surface_data.resolution.height as f32;
        scissors[0].extent.width = base.surface_data.resolution.width;
        scissors[0].extent.height = base.surface_data.resolution.height;

        base.device_data.record_submit(
            base.command_data.draw_buffer,
            base.sync_data.draw_commands_reuse_fence,
            &[vk::PipelineStageFlags::BOTTOM_OF_PIPE],
            &[base.sync_data.present_complete_semaphore],
            &[base.sync_data.rendering_complete_semaphore],
            |device, draw_command_buffer| {
                render_pass.begin(
                    device,
                    draw_command_buffer,
                    &base.surface_data,
                    present_index,
                    &clear_values,
                );
                device.cmd_bind_descriptor_sets(
                    draw_command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline_layout,
                    0,
                    &descriptor_sets[..],
                    &[],
                );
                device.cmd_bind_pipeline(
                    draw_command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    graphic_pipeline,
                );
                device.cmd_set_viewport(draw_command_buffer, 0, &viewports);
                device.cmd_set_scissor(draw_command_buffer, 0, &scissors);
                device.cmd_bind_vertex_buffers(
                    draw_command_buffer,
                    0,
                    &[vertex_input_buffer],
                    &[0],
                );
                device.cmd_bind_index_buffer(
                    draw_command_buffer,
                    index_buffer,
                    0,
                    vk::IndexType::UINT32,
                );
                device.cmd_draw_indexed(
                    draw_command_buffer,
                    index_buffer_data.len() as u32,
                    1,
                    0,
                    0,
                    1,
                );
                render_pass.end(device, draw_command_buffer)
            },
        );
        let present_info = vk::PresentInfoKHR {
            wait_semaphore_count: 1,
            p_wait_semaphores: &base.sync_data.rendering_complete_semaphore,
            swapchain_count: 1,
            p_swapchains: &base.swapchain_data.swapchain,
            p_image_indices: &present_index,
            ..Default::default()
        };

        match base
            .swapchain_data
            .loader
            .queue_present(base.device_data.present_queue, &present_info)
        {
            Ok(_) => {}
            Err(_) => {
                base.device_data.device.device_wait_idle().unwrap();
                let window_size = window.inner_size();
                base.resize(window_size.width, window_size.height);
                render_pass.resize(&window, &base.swapchain_data);
            }
        }
    });

    base.device_data.device.device_wait_idle().unwrap();

    for pipeline in graphics_pipelines {
        base.device_data.device.destroy_pipeline(pipeline, None);
    }
    base.device_data
        .device
        .destroy_pipeline_layout(pipeline_layout, None);
    base.device_data
        .device
        .destroy_shader_module(vertex_shader_module, None);
    base.device_data
        .device
        .destroy_shader_module(fragment_shader_module, None);
    base.device_data
        .device
        .free_memory(index_buffer_memory, None);
    base.device_data.device.destroy_buffer(index_buffer, None);
    base.device_data
        .device
        .free_memory(uniform_color_buffer_memory, None);
    base.device_data
        .device
        .destroy_buffer(uniform_color_buffer, None);
    base.device_data
        .device
        .free_memory(vertex_input_buffer_memory, None);
    base.device_data
        .device
        .destroy_buffer(vertex_input_buffer, None);
    for &descriptor_set_layout in desc_set_layouts.iter() {
        base.device_data
            .device
            .destroy_descriptor_set_layout(descriptor_set_layout, None);
    }
    base.device_data
        .device
        .destroy_descriptor_pool(descriptor_pool, None);
}
