use std::{mem::align_of, os::raw::c_void, rc};

use ash::{util::Align, vk};

use crate::graphics::vk_base::*;

pub struct Buffer {
    device_data: rc::Rc<device_data::DeviceData>,
    vk_buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    allocate_info: vk::MemoryAllocateInfo,
    len: usize,
}

// NOTE: Staging buffers aren't used, but could be for extra performance.
// NOTE: Each buffer allocates its own memory on the GPU.
impl Buffer {
    pub unsafe fn new<T: std::marker::Copy>(
        data: &[T],
        device_data: rc::Rc<device_data::DeviceData>,
        usage: vk::BufferUsageFlags,
    ) -> Self {
        let buffer_info = vk::BufferCreateInfo {
            size: std::mem::size_of_val(data) as u64,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let vk_buffer = device_data
            .device
            .create_buffer(&buffer_info, None)
            .unwrap();
        let buffer_memory_requirements =
            device_data.device.get_buffer_memory_requirements(vk_buffer);
        let buffer_memory_index = device_data
            .find_memory_type_index(
                &buffer_memory_requirements,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )
            .expect("Unable to find suitable memory type for buffer");
        let allocate_info = vk::MemoryAllocateInfo {
            allocation_size: buffer_memory_requirements.size,
            memory_type_index: buffer_memory_index,
            ..Default::default()
        };
        let memory = device_data
            .device
            .allocate_memory(&allocate_info, None)
            .unwrap();

        device_data
            .device
            .bind_buffer_memory(vk_buffer, memory, 0)
            .unwrap();

        let mut buffer = Self {
            device_data,
            vk_buffer,
            memory,
            allocate_info,
            len: data.len(),
        };

        buffer.set_data(data);

        buffer
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn vk_buffer(&self) -> vk::Buffer {
        self.vk_buffer
    }

    pub unsafe fn set_data<T: std::marker::Copy>(&mut self, data: &[T]) {
        let data_ptr: *mut c_void = self
            .device_data
            .device
            .map_memory(
                self.memory,
                0,
                self.allocate_info.allocation_size,
                vk::MemoryMapFlags::empty(),
            )
            .unwrap();
        let mut data_slice = Align::new(
            data_ptr,
            align_of::<u32>() as u64,
            self.allocate_info.allocation_size,
        );
        data_slice.copy_from_slice(data);
        self.device_data.device.unmap_memory(self.memory);
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.device_data.device.free_memory(self.memory, None);
            self.device_data.device.destroy_buffer(self.vk_buffer, None);
        }
    }
}
