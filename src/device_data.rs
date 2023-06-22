use ash::{extensions::khr, vk};

use crate::{instance_data::InstanceData, surface_data::SurfaceData};

pub struct DeviceData {
    pub device: ash::Device,
    pub pdevice: vk::PhysicalDevice, // TODO: Rename this
    pub queue_family_index: u32,
    pub memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub present_queue: vk::Queue,
}

impl DeviceData {
    pub unsafe fn new(
        instance_data: &InstanceData,
        surface_data: &SurfaceData,
    ) -> Self {
        let pdevices = instance_data
            .instance
            .enumerate_physical_devices()
            .expect("Physical device error");
        let (pdevice, queue_family_index) = pdevices
            .iter()
            .find_map(|pdevice| {
                instance_data
                    .instance
                    .get_physical_device_queue_family_properties(*pdevice)
                    .iter()
                    .enumerate()
                    .find_map(|(index, info)| {
                        let supports_graphic_and_surface =
                            info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                                && surface_data.surface_loader
                                    .get_physical_device_surface_support(
                                        *pdevice,
                                        index as u32,
                                        surface_data.surface,
                                    )
                                    .unwrap();
                        if supports_graphic_and_surface {
                            Some((*pdevice, index))
                        } else {
                            None
                        }
                    })
            })
            .expect("Couldn't find suitable device.");
        let memory_properties = instance_data
            .instance
            .get_physical_device_memory_properties(pdevice);
        let queue_family_index = queue_family_index as u32;

        let priorities = [1.0];

        let queue_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .queue_priorities(&priorities)
            .build();

        let device_extension_names_raw = [
            khr::Swapchain::name().as_ptr(),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            KhrPortabilitySubsetFn::NAME.as_ptr(),
        ];
        let features = vk::PhysicalDeviceFeatures {
            shader_clip_distance: 1,
            ..Default::default()
        };

        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(std::slice::from_ref(&queue_info))
            .enabled_extension_names(&device_extension_names_raw)
            .enabled_features(&features)
            .build();

        let device = instance_data
            .instance
            .create_device(pdevice, &device_create_info, None)
            .unwrap();

        let present_queue = device.get_device_queue(queue_family_index, 0);

        Self {
            device,
            pdevice, // TODO: Rename this
            queue_family_index,
            memory_properties,
            present_queue,
        }
    }

    pub fn record_submit<F: FnOnce(&ash::Device, vk::CommandBuffer)>(
        &self,
        command_buffer: vk::CommandBuffer,
        command_buffer_reuse_fence: vk::Fence,
        wait_mask: &[vk::PipelineStageFlags],
        wait_semaphores: &[vk::Semaphore],
        signal_semaphores: &[vk::Semaphore],
        f: F,
    ) {
        unsafe {
            self.device
                .wait_for_fences(&[command_buffer_reuse_fence], true, std::u64::MAX)
                .expect("Wait for fence failed.");

            self.device
                .reset_fences(&[command_buffer_reuse_fence])
                .expect("Reset fences failed.");

            self.device
                .reset_command_buffer(
                    command_buffer,
                    vk::CommandBufferResetFlags::RELEASE_RESOURCES,
                )
                .expect("Reset command buffer failed.");

            let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                .build();

            self.device
                .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                .expect("Begin commandbuffer");
            f(&self.device, command_buffer);
            self.device
                .end_command_buffer(command_buffer)
                .expect("End commandbuffer");

            let command_buffers = vec![command_buffer];

            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(wait_semaphores)
                .wait_dst_stage_mask(wait_mask)
                .command_buffers(&command_buffers)
                .signal_semaphores(signal_semaphores)
                .build();

            self.device
                .queue_submit(
                    self.present_queue,
                    &[submit_info],
                    command_buffer_reuse_fence,
                )
                .expect("queue submit failed.");
        }
    }

    pub fn find_memory_type_index(
        &self,
        memory_req: &vk::MemoryRequirements,
        flags: vk::MemoryPropertyFlags,
    ) -> Option<u32> {
        self.memory_properties.memory_types[..self.memory_properties.memory_type_count as _]
            .iter()
            .enumerate()
            .find(|(index, memory_type)| {
                (1 << index) & memory_req.memory_type_bits != 0
                    && memory_type.property_flags & flags == flags
            })
            .map(|(index, _memory_type)| index as _)
    }
}

impl Drop for DeviceData {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
        }
    }
}
