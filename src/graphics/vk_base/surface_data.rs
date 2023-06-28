use ash::{extensions::khr, vk};

use super::{device_data::DeviceData, instance_data::InstanceData};

use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

pub struct SurfaceData {
    pub loader: khr::Surface,
    pub surface: vk::SurfaceKHR,
    pub format: Option<vk::SurfaceFormatKHR>,
    pub resolution: vk::Extent2D,
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
            loader: surface_loader,
            surface,
            format: None,
            resolution: vk::Extent2D {
                width: window_size.width,
                height: window_size.height,
            },
        }
    }

    pub unsafe fn update_surface_format(&mut self, device_data: &DeviceData) {
        let surface_format = self.loader
            .get_physical_device_surface_formats(device_data.physical_device, self.surface)
            .unwrap()[0];

        self.format = Some(surface_format);
    }
}

impl Drop for SurfaceData {
    fn drop(&mut self) {
        unsafe {
            self.loader.destroy_surface(self.surface, None);
        }
    }
}
