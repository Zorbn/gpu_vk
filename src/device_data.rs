use ash::vk;

pub struct DeviceData {
    pub instance: ash::Instance,
    pub device: ash::Device,
    pub pdevice: vk::PhysicalDevice, // TODO: Rename this
    pub memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub queue_family_index: u32,
    pub present_queue: vk::Queue,
}

impl DeviceData {
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
            self.instance.destroy_instance(None);
        }
    }
}