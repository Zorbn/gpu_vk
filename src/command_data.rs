use std::rc;

use ash::vk;

use crate::device_data::DeviceData;

pub struct CommandData {
    pub device_data: rc::Rc<DeviceData>,
    pub pool: vk::CommandPool,
    pub draw_command_buffer: vk::CommandBuffer,
    pub setup_command_buffer: vk::CommandBuffer,
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
