pub mod app;
pub mod sprite_batch;
pub mod texture;

mod mat4;
mod vk_base;
mod vk_resources;

use std::{default::Default, ffi::CStr, mem, time};

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
    resources: &'a Resources,
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

        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: base.surface_data.resolution.width as f32,
            height: base.surface_data.resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];

        let scissors = [base.surface_data.resolution.into()];

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
                self.resources
                    .base
                    .device_data
                    .device
                    .device_wait_idle()
                    .unwrap();

                let window_size = window.inner_size();
                self.resources
                    .base
                    .resize(window_size.width, window_size.height);
                self.resources
                    .render_pass
                    .resize(&window, &self.resources.base.swapchain_data);

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
                self.resources
                    .projection_matrix_buffer
                    .set_data(&self.resources.projection_matrix);
            }

            let (present_index, _) = match self
                .resources
                .base
                .swapchain_data
                .loader
                .acquire_next_image(
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

            self.resources.viewports[0].width =
                self.resources.base.surface_data.resolution.width as f32;
            self.resources.viewports[0].height =
                self.resources.base.surface_data.resolution.height as f32;
            self.resources.scissors[0].extent.width =
                self.resources.base.surface_data.resolution.width;
            self.resources.scissors[0].extent.height =
                self.resources.base.surface_data.resolution.height;

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

                    let draw = Draw {
                        device,
                        command_buffer,
                        resources: &self.resources,
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

            match self
                .resources
                .base
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
