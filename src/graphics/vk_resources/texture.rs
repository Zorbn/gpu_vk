use ash::vk;
use std::{fs, io, rc};

use crate::graphics::vk_base::*;

pub struct Texture {
    pub descriptor: vk::DescriptorImageInfo,

    device_data: rc::Rc<device_data::DeviceData>,
    image_buffer_memory: vk::DeviceMemory,
    image_buffer: vk::Buffer,
    image_view: vk::ImageView,
    memory: vk::DeviceMemory,
    texture_image: vk::Image,
    sampler: vk::Sampler,
}

impl Texture {
    pub unsafe fn new(
        path: &str,
        device_data: rc::Rc<device_data::DeviceData>,
        command_data: &command_data::CommandData,
        sync_data: &sync_data::SyncData,
    ) -> Self {
        let file = fs::File::open(path).unwrap_or_else(|_| panic!("Failed to load file {}", path));
        let image = image::load(io::BufReader::new(file), image::ImageFormat::Png)
            .unwrap()
            .to_rgba8();
        let (width, height) = image.dimensions();
        let image_extent = vk::Extent2D { width, height };
        let image_data = image.into_raw();
        let image_buffer_info = vk::BufferCreateInfo {
            size: (std::mem::size_of::<u8>() * image_data.len()) as u64,
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let image_buffer = device_data
            .device
            .create_buffer(&image_buffer_info, None)
            .unwrap();
        let image_buffer_memory_req = device_data
            .device
            .get_buffer_memory_requirements(image_buffer);
        let image_buffer_memory_index = device_data
            .find_memory_type_index(
                &image_buffer_memory_req,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )
            .expect("Unable to find suitable memory type for the image buffer");

        let image_buffer_allocate_info = vk::MemoryAllocateInfo {
            allocation_size: image_buffer_memory_req.size,
            memory_type_index: image_buffer_memory_index,
            ..Default::default()
        };
        let image_buffer_memory = device_data
            .device
            .allocate_memory(&image_buffer_allocate_info, None)
            .unwrap();
        let image_ptr = device_data
            .device
            .map_memory(
                image_buffer_memory,
                0,
                image_buffer_memory_req.size,
                vk::MemoryMapFlags::empty(),
            )
            .unwrap();
        let mut image_slice = ash::util::Align::new(
            image_ptr,
            std::mem::align_of::<u8>() as u64,
            image_buffer_memory_req.size,
        );
        image_slice.copy_from_slice(&image_data);
        device_data.device.unmap_memory(image_buffer_memory);
        device_data
            .device
            .bind_buffer_memory(image_buffer, image_buffer_memory, 0)
            .unwrap();

        let create_info = vk::ImageCreateInfo {
            image_type: vk::ImageType::TYPE_2D,
            format: vk::Format::R8G8B8A8_UNORM,
            extent: image_extent.into(),
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let texture_image = device_data
            .device
            .create_image(&create_info, None)
            .unwrap();
        let memory_requirements = device_data
            .device
            .get_image_memory_requirements(texture_image);
        let memory_index = device_data
            .find_memory_type_index(&memory_requirements, vk::MemoryPropertyFlags::DEVICE_LOCAL)
            .expect("Unable to find suitable memory index for depth image");

        let allocate_info = vk::MemoryAllocateInfo {
            allocation_size: memory_requirements.size,
            memory_type_index: memory_index,
            ..Default::default()
        };
        let memory = device_data
            .device
            .allocate_memory(&allocate_info, None)
            .unwrap();
        device_data
            .device
            .bind_image_memory(texture_image, memory, 0)
            .expect("Unable to bind depth image memory");

        device_data.record_submit(
            command_data.setup_buffer,
            sync_data.setup_commands_reuse_fence,
            &[],
            &[],
            &[],
            |device, texture_command_buffer| {
                let texture_barrier = vk::ImageMemoryBarrier {
                    dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                    new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    image: texture_image,
                    subresource_range: vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        level_count: 1,
                        layer_count: 1,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                device.cmd_pipeline_barrier(
                    texture_command_buffer,
                    vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[texture_barrier],
                );
                let buffer_copy_regions = vk::BufferImageCopy::builder()
                    .image_subresource(
                        vk::ImageSubresourceLayers::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .layer_count(1)
                            .build(),
                    )
                    .image_extent(image_extent.into())
                    .build();

                device.cmd_copy_buffer_to_image(
                    texture_command_buffer,
                    image_buffer,
                    texture_image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[buffer_copy_regions],
                );
                let texture_barrier_end = vk::ImageMemoryBarrier {
                    src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                    dst_access_mask: vk::AccessFlags::SHADER_READ,
                    old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    image: texture_image,
                    subresource_range: vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        level_count: 1,
                        layer_count: 1,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                device.cmd_pipeline_barrier(
                    texture_command_buffer,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[texture_barrier_end],
                );
            },
        );

        let sampler_info = vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            address_mode_u: vk::SamplerAddressMode::MIRRORED_REPEAT,
            address_mode_v: vk::SamplerAddressMode::MIRRORED_REPEAT,
            address_mode_w: vk::SamplerAddressMode::MIRRORED_REPEAT,
            max_anisotropy: 1.0,
            border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
            compare_op: vk::CompareOp::NEVER,
            ..Default::default()
        };

        let sampler = device_data
            .device
            .create_sampler(&sampler_info, None)
            .unwrap();

        let image_view_info = vk::ImageViewCreateInfo {
            view_type: vk::ImageViewType::TYPE_2D,
            format: create_info.format,
            components: vk::ComponentMapping {
                r: vk::ComponentSwizzle::R,
                g: vk::ComponentSwizzle::G,
                b: vk::ComponentSwizzle::B,
                a: vk::ComponentSwizzle::A,
            },
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            },
            image: texture_image,
            ..Default::default()
        };

        let image_view = device_data
            .device
            .create_image_view(&image_view_info, None)
            .unwrap();

        let descriptor = vk::DescriptorImageInfo {
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            image_view,
            sampler,
        };

        Self {
            device_data,
            descriptor,
            image_buffer_memory,
            image_buffer,
            image_view,
            memory,
            texture_image,
            sampler,
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            self.device_data
                .device
                .free_memory(self.image_buffer_memory, None);
            self.device_data.device.destroy_buffer(self.image_buffer, None);
            self.device_data.device.free_memory(self.memory, None);
            self.device_data
                .device
                .destroy_image_view(self.image_view, None);
            self.device_data.device.destroy_image(self.texture_image, None);
            self.device_data.device.destroy_sampler(self.sampler, None);
        }
    }
}
