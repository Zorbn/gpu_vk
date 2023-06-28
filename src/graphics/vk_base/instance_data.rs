use std::{ffi, os::raw};

use ash::{vk, extensions::ext};

use raw_window_handle::{HasRawDisplayHandle};

pub struct InstanceData {
    pub instance: ash::Instance,
    pub entry: ash::Entry,
}

impl InstanceData {
    pub unsafe fn new(window: &winit::window::Window) -> Self {
        let entry = ash::Entry::linked();

        let app_name = ffi::CStr::from_bytes_with_nul_unchecked(b"gpu_vk\0");

        let app_info = vk::ApplicationInfo::builder()
            .application_name(app_name)
            .application_version(0)
            .engine_name(app_name)
            .engine_version(0)
            .api_version(vk::make_api_version(0, 1, 0, 0))
            .build();

        let create_flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            vk::InstanceCreateFlags::default()
        };

        let layer_names = [ffi::CStr::from_bytes_with_nul_unchecked(
            b"VK_LAYER_KHRONOS_validation\0",
        )];
        let layers_names_raw: Vec<*const raw::c_char> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let mut extension_names =
            ash_window::enumerate_required_extensions(window.raw_display_handle())
                .unwrap()
                .to_vec();
        extension_names.push(ext::DebugUtils::name().as_ptr());

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            extension_names.push(KhrPortabilityEnumerationFn::NAME.as_ptr());
            // Enabling this extension is a requirement when using `VK_KHR_portability_subset`
            extension_names.push(KhrGetPhysicalDeviceProperties2Fn::NAME.as_ptr());
        }

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&extension_names)
            .flags(create_flags)
            .build();

        let instance = entry
            .create_instance(&create_info, None)
            .expect("Failed to create instance");

        Self { instance, entry }
    }
}

impl Drop for InstanceData {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}