use std::{
    mem::{self, align_of},
    os::raw::c_void,
    rc,
};

use ash::{util::Align, vk};
use vk_mem::Alloc;

use crate::graphics::vk_base::*;

pub struct Buffer {
    device_data: rc::Rc<device_data::DeviceData>,
    vk_buffer: vk::Buffer,
    allocation: mem::ManuallyDrop<vk_mem::Allocation>,
    allocation_info: vk_mem::AllocationInfo,
    len: usize,
}

impl Buffer {
    pub unsafe fn new<T: std::marker::Copy>(
        data: &[T],
        device_data: rc::Rc<device_data::DeviceData>,
        usage: vk::BufferUsageFlags,
    ) -> Self {
        let allocation_create_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::Auto,
            // NOTE: Staging buffers aren't used, but could be for extra performance.
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
                | vk_mem::AllocationCreateFlags::MAPPED,
            ..Default::default()
        };
        let buffer_info = vk::BufferCreateInfo {
            size: std::mem::size_of_val(data) as u64,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let (vk_buffer, allocation) = device_data
            .allocator
            .create_buffer(&buffer_info, &allocation_create_info)
            .unwrap();
        let allocation_info = device_data
            .allocator
            .get_allocation_info(&allocation)
            .unwrap();

        let mut buffer = Self {
            device_data,
            vk_buffer,
            allocation: mem::ManuallyDrop::new(allocation),
            allocation_info,
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
        let data_ptr = self
            .device_data
            .allocator
            .map_memory(&mut self.allocation)
            .unwrap() as *mut c_void;
        let mut data_slice =
            Align::new(data_ptr, align_of::<T>() as u64, self.allocation_info.size);
        data_slice.copy_from_slice(data);
        self.device_data
            .allocator
            .unmap_memory(&mut self.allocation);
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.device_data.device.device_wait_idle().unwrap();

            self.device_data.allocator.destroy_buffer(
                self.vk_buffer,
                mem::ManuallyDrop::take(&mut self.allocation),
            );
        }
    }
}
