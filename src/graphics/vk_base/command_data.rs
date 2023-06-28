use std::rc;

use ash::vk;

use super::device_data::DeviceData;

pub struct CommandData {
    device_data: rc::Rc<DeviceData>,
    pub pool: vk::CommandPool,
    pub draw_buffer: vk::CommandBuffer,
    pub setup_buffer: vk::CommandBuffer,
}

impl CommandData {
    pub unsafe fn new(device_data: rc::Rc<DeviceData>) -> Self {
        let pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(device_data.queue_family_index)
            .build();

        let pool = device_data.device.create_command_pool(&pool_create_info, None).unwrap();

        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(2)
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .build();

        let command_buffers = device_data.device
            .allocate_command_buffers(&command_buffer_allocate_info)
            .unwrap();
        let setup_command_buffer = command_buffers[0];
        let draw_command_buffer = command_buffers[1];

        Self {
            device_data,
            pool,
            draw_buffer: draw_command_buffer,
            setup_buffer: setup_command_buffer,
        }
    }
}

impl Drop for CommandData {
    fn drop(&mut self) {
        unsafe {
            self.device_data
                .device
                .destroy_command_pool(self.pool, None);
        }
    }
}
