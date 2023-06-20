use ash::vk;

pub struct PhysicalDeviceData {
    pub pdevice: vk::PhysicalDevice, // TODO: Rename this
    pub device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub queue_family_index: u32,
    pub present_queue: vk::Queue,
}
