use std::rc;

use ash::{extensions::khr, vk};

use crate::{
    command_data::CommandData, device_data::DeviceData, surface_data::SurfaceData,
    sync_data::SyncData,
};

// TODO: Remove redundant prefixes "swapchain_loader" -> "loader" because it's contained in "swapchain"data
pub struct SwapchainData {
    pub device_data: rc::Rc<DeviceData>,
    pub swapchain_loader: khr::Swapchain,
    pub swapchain: vk::SwapchainKHR,
    pub present_images: Vec<vk::Image>,
    pub present_image_views: Vec<vk::ImageView>,
    pub depth_image: vk::Image,
    pub depth_image_view: vk::ImageView,
    pub depth_image_memory: vk::DeviceMemory,
}

impl SwapchainData {
    pub unsafe fn new(
        device_data: rc::Rc<DeviceData>,
        surface_data: &SurfaceData,
        command_data: &CommandData,
        sync_data: &SyncData,
    ) -> Self {
        let surface_capabilities = surface_data
            .surface_loader
            .get_physical_device_surface_capabilities(
                device_data.pdevice,
                surface_data.surface,
            )
            .unwrap();
        let mut desired_image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count > 0
            && desired_image_count > surface_capabilities.max_image_count
        {
            desired_image_count = surface_capabilities.max_image_count;
        }

        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };
        let present_modes = surface_data
            .surface_loader
            .get_physical_device_surface_present_modes(
                device_data.pdevice,
                surface_data.surface,
            )
            .unwrap();
        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO);
        let swapchain_loader = khr::Swapchain::new(&device_data.instance, &device_data.device);

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface_data.surface)
            .min_image_count(desired_image_count)
            .image_color_space(surface_data.surface_format.color_space)
            .image_format(surface_data.surface_format.format)
            .image_extent(surface_data.surface_resolution)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1)
            .build();

        let swapchain = swapchain_loader
            .create_swapchain(&swapchain_create_info, None)
            .unwrap();

        let present_images = swapchain_loader.get_swapchain_images(swapchain).unwrap();
        let present_image_views: Vec<vk::ImageView> = present_images
            .iter()
            .map(|&image| {
                let create_view_info = vk::ImageViewCreateInfo::builder()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(surface_data.surface_format.format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::R,
                        g: vk::ComponentSwizzle::G,
                        b: vk::ComponentSwizzle::B,
                        a: vk::ComponentSwizzle::A,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(image)
                    .build();
                device_data.device.create_image_view(&create_view_info, None).unwrap()
            })
            .collect();
        let depth_image_create_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::D16_UNORM)
            .extent(surface_data.surface_resolution.into())
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .build();

        let depth_image = device_data.device.create_image(&depth_image_create_info, None).unwrap();
        let depth_image_memory_req = device_data.device.get_image_memory_requirements(depth_image);
        let depth_image_memory_index = device_data.find_memory_type_index(
            &depth_image_memory_req,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .expect("Unable to find suitable memory index for depth image.");

        let depth_image_allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(depth_image_memory_req.size)
            .memory_type_index(depth_image_memory_index)
            .build();

        let depth_image_memory = device_data.device
            .allocate_memory(&depth_image_allocate_info, None)
            .unwrap();

        device_data.device
            .bind_image_memory(depth_image, depth_image_memory, 0)
            .expect("Unable to bind depth image memory");

        device_data.record_submit(
            command_data.setup_command_buffer,
            sync_data.setup_commands_reuse_fence,
            &[],
            &[],
            &[],
            |device, setup_command_buffer| {
                let layout_transition_barriers = vk::ImageMemoryBarrier::builder()
                    .image(depth_image)
                    .dst_access_mask(
                        vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                            | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                    )
                    .new_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .subresource_range(
                        vk::ImageSubresourceRange::builder()
                            .aspect_mask(vk::ImageAspectFlags::DEPTH)
                            .layer_count(1)
                            .level_count(1)
                            .build(),
                    )
                    .build();

                device.cmd_pipeline_barrier(
                    setup_command_buffer,
                    vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                    vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[layout_transition_barriers],
                );
            },
        );

        let depth_image_view_info = vk::ImageViewCreateInfo::builder()
            .subresource_range(
                vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::DEPTH)
                    .level_count(1)
                    .layer_count(1)
                    .build(),
            )
            .image(depth_image)
            .format(depth_image_create_info.format)
            .view_type(vk::ImageViewType::TYPE_2D)
            .build();

        let depth_image_view = device_data.device
            .create_image_view(&depth_image_view_info, None)
            .unwrap();

        Self {
            device_data,
            swapchain_loader,
            swapchain,
            present_images,
            present_image_views,
            depth_image,
            depth_image_view,
            depth_image_memory,
        }
    }

    pub unsafe fn recreate(
        &mut self,
        surface_data: &SurfaceData,
        command_data: &CommandData,
        sync_data: &SyncData,
    ) {
        self.destroy();

        let new_swapchain_data = SwapchainData::new(
            self.device_data.clone(),
            surface_data,
            command_data,
            sync_data,
        );

        self.swapchain_loader = new_swapchain_data.swapchain_loader;
        self.swapchain = new_swapchain_data.swapchain;
        self.present_images = new_swapchain_data.present_images;
        self.present_image_views = new_swapchain_data.present_image_views;
        self.depth_image = new_swapchain_data.depth_image;
        self.depth_image_view = new_swapchain_data.depth_image_view;
        self.depth_image_memory = new_swapchain_data.depth_image_memory;
    }

    pub unsafe fn destroy(&mut self) {
        self.device_data.device.free_memory(self.depth_image_memory, None);
        self.device_data.device.destroy_image_view(self.depth_image_view, None);
        self.device_data.device.destroy_image(self.depth_image, None);
        for &image_view in self.present_image_views.iter() {
            self.device_data.device.destroy_image_view(image_view, None);
        }
        self.swapchain_loader
            .destroy_swapchain(self.swapchain, None);
    }
}
