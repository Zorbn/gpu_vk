use std::rc;

use crate::vk_base::{device_data::DeviceData, *};

use ash::vk;

pub struct RenderPass {
    device_data: rc::Rc<DeviceData>,
    pub vk_render_pass: vk::RenderPass,
    pub framebuffers: Vec<vk::Framebuffer>,
}

impl RenderPass {
    pub unsafe fn new(
        device_data: rc::Rc<device_data::DeviceData>,
        surface_data: &surface_data::SurfaceData,
        swapchain_data: &swapchain_data::SwapchainData,
        window: &winit::window::Window,
    ) -> Self {
        let surface_format = surface_data
            .format
            .expect("Surface format was uninitialized when creating render pass");

        let renderpass_attachments = [
            vk::AttachmentDescription {
                format: surface_format.format,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                ..Default::default()
            },
            vk::AttachmentDescription {
                format: vk::Format::D16_UNORM,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                initial_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                ..Default::default()
            },
        ];
        let color_attachment_refs = [vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];
        let depth_attachment_ref = vk::AttachmentReference {
            attachment: 1,
            layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };
        let dependencies = [vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
                | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ..Default::default()
        }];

        let subpass = vk::SubpassDescription::builder()
            .color_attachments(&color_attachment_refs)
            .depth_stencil_attachment(&depth_attachment_ref)
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .build();

        let renderpass_create_info = vk::RenderPassCreateInfo::builder()
            .attachments(&renderpass_attachments)
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(&dependencies)
            .build();

        let vk_render_pass = device_data
            .device
            .create_render_pass(&renderpass_create_info, None)
            .unwrap();

        let mut framebuffers = Vec::new();
        Self::new_framebuffers(
            &mut framebuffers,
            window,
            vk_render_pass,
            device_data.clone(),
            swapchain_data,
        );

        Self {
            device_data,
            vk_render_pass,
            framebuffers,
        }
    }

    pub unsafe fn resize(
        &mut self,
        window: &winit::window::Window,
        swapchain_data: &swapchain_data::SwapchainData,
    ) {
        Self::new_framebuffers(
            &mut self.framebuffers,
            window,
            self.vk_render_pass,
            self.device_data.clone(),
            swapchain_data,
        )
    }

    pub unsafe fn begin(
        &self,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        surface_data: &surface_data::SurfaceData,
        present_index: u32,
        clear_values: &[vk::ClearValue],
    ) {
        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.vk_render_pass)
            .framebuffer(self.framebuffers[present_index as usize])
            .render_area(surface_data.resolution.into())
            .clear_values(clear_values)
            .build();

        device.cmd_begin_render_pass(
            command_buffer,
            &render_pass_begin_info,
            vk::SubpassContents::INLINE,
        );
    }

    pub unsafe fn end(&self, device: &ash::Device, command_buffer: vk::CommandBuffer) {
        device.cmd_end_render_pass(command_buffer);
    }

    unsafe fn new_framebuffers(
        framebuffers: &mut Vec<vk::Framebuffer>,
        window: &winit::window::Window,
        render_pass: vk::RenderPass,
        device_data: rc::Rc<device_data::DeviceData>,
        swapchain_data: &swapchain_data::SwapchainData,
    ) {
        for framebuffer in framebuffers.iter() {
            device_data.device.destroy_framebuffer(*framebuffer, None);
        }

        framebuffers.clear();

        let window_size = window.inner_size();

        for present_image_view in &swapchain_data.present_image_views {
            let framebuffer_attachments = [*present_image_view, swapchain_data.depth_image_view];
            let frame_buffer_create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(render_pass)
                .attachments(&framebuffer_attachments)
                .width(window_size.width)
                .height(window_size.height)
                .layers(1)
                .build();

            framebuffers.push(
                device_data
                    .device
                    .create_framebuffer(&frame_buffer_create_info, None)
                    .unwrap(),
            );
        }
    }
}

impl Drop for RenderPass {
    fn drop(&mut self) {
        unsafe {
            for framebuffer in &self.framebuffers {
                self.device_data
                    .device
                    .destroy_framebuffer(*framebuffer, None);
            }
            self.device_data
                .device
                .destroy_render_pass(self.vk_render_pass, None);
        }
    }
}
