use ash::{extensions::khr, vk};

pub struct SurfaceData {
    pub surface_loader: khr::Surface,
    pub surface: vk::SurfaceKHR,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_resolution: vk::Extent2D,
}

impl Drop for SurfaceData {
    fn drop(&mut self) {
        unsafe {
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}
