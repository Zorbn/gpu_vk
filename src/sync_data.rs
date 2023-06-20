use std::rc;

use ash::vk;

use crate::device_data::DeviceData;

pub struct SyncData {
    pub device_data: rc::Rc<DeviceData>,

    pub present_complete_semaphore: vk::Semaphore,
    pub rendering_complete_semaphore: vk::Semaphore,

    pub draw_commands_reuse_fence: vk::Fence,
    pub setup_commands_reuse_fence: vk::Fence,
}

impl Drop for SyncData {
    fn drop(&mut self) {
        unsafe {
            self.device_data.device
                .destroy_semaphore(self.present_complete_semaphore, None);
            self.device_data.device
                .destroy_semaphore(self.rendering_complete_semaphore, None);
            self.device_data.device
                .destroy_fence(self.draw_commands_reuse_fence, None);
            self.device_data.device
                .destroy_fence(self.setup_commands_reuse_fence, None);
        }
    }
}