use ash::vk;

use std::{fs, io, mem, rc};

use vk_mem::Alloc;

use crate::graphics::{vk_base::*, vk_resources::*, *};

pub enum Filter {
    Linear,
    Nearest,
}

pub struct Texture {
    pub descriptor: vk::DescriptorImageInfo,

    device_data: rc::Rc<device_data::DeviceData>,

    // Maintain the texture's image buffer until the texture is dropped.
    #[allow(dead_code)]
    image_buffer: buffer::Buffer,

    image_view: vk::ImageView,
    allocation: mem::ManuallyDrop<vk_mem::Allocation>,
    texture_image: vk::Image,
    sampler: vk::Sampler,
}

impl Texture {
    pub fn new(resources: &Resources, path: &str, filter: Filter) -> Self {
        let file = fs::File::open(path).unwrap_or_else(|_| panic!("Failed to load file {}", path));

        let image = image::load(io::BufReader::new(file), image::ImageFormat::Png)
            .unwrap()
            .to_rgba8();
        let (width, height) = image.dimensions();
        let image_extent = vk::Extent2D { width, height };
        let image_data = image.into_raw();

        unsafe {
            let image_buffer = buffer::Buffer::new(
                &image_data,
                resources.base.device_data.clone(),
                vk::BufferUsageFlags::TRANSFER_SRC,
            );
            let image_info = vk::ImageCreateInfo {
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

            let allocation_info = vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::Auto,
                ..Default::default()
            };

            let (texture_image, allocation) = resources
                .base
                .device_data
                .allocator
                .create_image(&image_info, &allocation_info)
                .unwrap();

            resources.base.device_data.record_submit(
                resources.base.command_data.setup_buffer,
                resources.base.sync_data.setup_commands_reuse_fence,
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
                        image_buffer.vk_buffer(),
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

            let sampler_filter = match filter {
                Filter::Linear => vk::Filter::LINEAR,
                Filter::Nearest => vk::Filter::NEAREST,
            };

            let sampler_info = vk::SamplerCreateInfo {
                mag_filter: sampler_filter,
                min_filter: sampler_filter,
                mipmap_mode: vk::SamplerMipmapMode::LINEAR,
                address_mode_u: vk::SamplerAddressMode::REPEAT,
                address_mode_v: vk::SamplerAddressMode::REPEAT,
                address_mode_w: vk::SamplerAddressMode::REPEAT,
                max_anisotropy: 1.0,
                border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
                compare_op: vk::CompareOp::NEVER,
                ..Default::default()
            };

            let sampler = resources
                .base
                .device_data
                .device
                .create_sampler(&sampler_info, None)
                .unwrap();

            let image_view_info = vk::ImageViewCreateInfo {
                view_type: vk::ImageViewType::TYPE_2D,
                format: image_info.format,
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

            let image_view = resources
                .base
                .device_data
                .device
                .create_image_view(&image_view_info, None)
                .unwrap();

            let descriptor = vk::DescriptorImageInfo {
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                image_view,
                sampler,
            };

            Self {
                device_data: resources.base.device_data.clone(),
                descriptor,
                image_buffer,
                image_view,
                allocation: mem::ManuallyDrop::new(allocation),
                texture_image,
                sampler,
            }
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            self.device_data
                .allocator
                .free_memory(mem::ManuallyDrop::take(&mut self.allocation));
            self.device_data
                .device
                .destroy_image_view(self.image_view, None);
            self.device_data
                .device
                .destroy_image(self.texture_image, None);
            self.device_data.device.destroy_sampler(self.sampler, None);
        }
    }
}
