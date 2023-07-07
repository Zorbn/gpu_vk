use std::rc;

use ash::vk;

use super::device_data::DeviceData;

pub struct SyncData {
    device_data: rc::Rc<DeviceData>,
    pub present_complete_semaphore: vk::Semaphore,
    pub rendering_complete_semaphore: vk::Semaphore,
    pub draw_commands_reuse_fence: vk::Fence,
    pub setup_commands_reuse_fence: vk::Fence,
}

impl SyncData {
    pub unsafe fn new(device_data: rc::Rc<DeviceData>) -> Self {
        let semaphore_create_info = vk::SemaphoreCreateInfo::default();

        let present_complete_semaphore = device_data
            .device
            .create_semaphore(&semaphore_create_info, None)
            .unwrap();
        let rendering_complete_semaphore = device_data
            .device
            .create_semaphore(&semaphore_create_info, None)
            .unwrap();

        let fence_create_info = vk::FenceCreateInfo::builder()
            .flags(vk::FenceCreateFlags::SIGNALED)
            .build();

        let draw_commands_reuse_fence = device_data
            .device
            .create_fence(&fence_create_info, None)
            .expect("Failed to create fence");
        let setup_commands_reuse_fence = device_data
            .device
            .create_fence(&fence_create_info, None)
            .expect("Failed to create fence");

        Self {
            device_data,
            present_complete_semaphore,
            rendering_complete_semaphore,
            draw_commands_reuse_fence,
            setup_commands_reuse_fence,
        }
    }
}

impl Drop for SyncData {
    fn drop(&mut self) {
        unsafe {
            self.device_data
                .device
                .destroy_semaphore(self.present_complete_semaphore, None);
            self.device_data
                .device
                .destroy_semaphore(self.rendering_complete_semaphore, None);
            self.device_data
                .device
                .destroy_fence(self.draw_commands_reuse_fence, None);
            self.device_data
                .device
                .destroy_fence(self.setup_commands_reuse_fence, None);
        }
    }
}
