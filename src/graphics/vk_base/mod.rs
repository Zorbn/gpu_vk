pub mod command_data;
pub mod device_data;
pub mod instance_data;
pub mod surface_data;
pub mod swapchain_data;
pub mod sync_data;

use ash::extensions::ext;
use ash::vk;
use command_data::CommandData;
use device_data::DeviceData;
use instance_data::InstanceData;
use std::borrow::Cow;
use std::ffi::CStr;
use std::ops::Drop;
use std::rc;
use surface_data::SurfaceData;
use swapchain_data::SwapchainData;
use sync_data::SyncData;

#[cfg(any(target_os = "macos", target_os = "ios"))]
use ash::vk::{
    KhrGetPhysicalDeviceProperties2Fn, KhrPortabilityEnumerationFn, KhrPortabilitySubsetFn,
};

use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
    platform::run_return::EventLoopExtRunReturn,
};

// Simple offset_of macro akin to C++ offsetof.
#[macro_export]
macro_rules! offset_of {
    ($base:path, $field:ident) => {{
        #[allow(unused_unsafe)]
        unsafe {
            let b: $base = mem::zeroed();
            std::ptr::addr_of!(b.$field) as isize - std::ptr::addr_of!(b) as isize
        }
    }};
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number = callback_data.message_id_number;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    println!(
        "{message_severity:?}:\n{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n",
    );

    vk::FALSE
}

// NOTE: Multiple in-flight frames are currently unsupported:
// https://vulkan-tutorial.com/Drawing_a_triangle/Drawing/Frames_in_flight
pub struct VkBase {
    pub debug_utils_loader: ext::DebugUtils,
    pub debug_call_back: vk::DebugUtilsMessengerEXT,

    pub swapchain_data: SwapchainData,
    pub sync_data: SyncData,
    pub command_data: CommandData,
    pub surface_data: SurfaceData,
    pub device_data: rc::Rc<DeviceData>,
    pub instance_data: InstanceData,
}

impl VkBase {
    pub fn render_loop<F: FnMut()>(
        window: &winit::window::Window,
        event_loop: &mut EventLoop<()>,
        mut f: F,
    ) {
        event_loop.run_return(|event, _, control_flow| match event {
            Event::WindowEvent {
                event:
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    },
                ..
            } => control_flow.set_exit(),
            Event::MainEventsCleared
            | Event::WindowEvent {
                event: WindowEvent::Resized(..),
                ..
            } => {
                let window_size = window.inner_size();

                if window_size.width == 0 || window_size.height == 0 {
                    control_flow.set_wait();
                    return;
                }

                control_flow.set_poll();

                f();
            }
            _ => (),
        });
    }

    pub fn new(window: &winit::window::Window) -> Self {
        unsafe {
            let instance_data = InstanceData::new(window);
            let mut surface_data = SurfaceData::new(window, &instance_data);
            let device_data = rc::Rc::new(DeviceData::new(&instance_data, &surface_data));
            surface_data.update_surface_format(&device_data);
            let command_data = CommandData::new(device_data.clone());
            let sync_data = SyncData::new(device_data.clone());
            let swapchain_data = SwapchainData::new(
                device_data.clone(),
                &instance_data,
                &surface_data,
                &command_data,
                &sync_data,
            );

            let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                )
                .pfn_user_callback(Some(vulkan_debug_callback))
                .build();

            let debug_utils_loader =
                ext::DebugUtils::new(&instance_data.entry, &instance_data.instance);
            let debug_call_back = debug_utils_loader
                .create_debug_utils_messenger(&debug_info, None)
                .unwrap();

            VkBase {
                instance_data,
                device_data,
                debug_call_back,
                debug_utils_loader,
                surface_data,
                swapchain_data,
                command_data,
                sync_data,
            }
        }
    }

    pub fn resize(&mut self, window_width: u32, window_height: u32) {
        self.surface_data.resolution.width = window_width;
        self.surface_data.resolution.height = window_height;

        unsafe {
            self.swapchain_data.recreate(
                &self.instance_data,
                &self.surface_data,
                &self.command_data,
                &self.sync_data,
            );
        }
    }
}

impl Drop for VkBase {
    fn drop(&mut self) {
        unsafe {
            self.device_data.device.device_wait_idle().unwrap();

            self.swapchain_data.destroy();

            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_call_back, None);
        }
    }
}
