use ash::{extensions::khr, vk};

use crate::{device_data::DeviceData, instance_data::InstanceData};

use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

pub struct SurfaceData {
    pub surface_loader: khr::Surface,
    pub surface: vk::SurfaceKHR,
    pub surface_format: Option<vk::SurfaceFormatKHR>,
    pub surface_resolution: vk::Extent2D,
}

impl SurfaceData {
    pub unsafe fn new(window: &winit::window::Window, instance_data: &InstanceData) -> Self {
        let surface = ash_window::create_surface(
            &instance_data.entry,
            &instance_data.instance,
            window.raw_display_handle(),
            window.raw_window_handle(),
            None,
        )
        .unwrap();

        let surface_loader = khr::Surface::new(&instance_data.entry, &instance_data.instance);

        let window_size = window.inner_size();

        Self {
            surface_loader,
            surface,
            surface_format: None,
            surface_resolution: vk::Extent2D {
                width: window_size.width,
                height: window_size.height,
            },
        }
    }

    pub unsafe fn update_surface_format(&mut self, device_data: &DeviceData) {
        let surface_format = self.surface_loader
            .get_physical_device_surface_formats(device_data.pdevice, self.surface)
            .unwrap()[0];

        self.surface_format = Some(surface_format);
    }
}

impl Drop for SurfaceData {
    fn drop(&mut self) {
        unsafe {
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}
