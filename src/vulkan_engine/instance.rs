use std::ffi::c_void;
use std::ptr;

use ash::extensions::ext::DebugUtils;
use ash::vk;
use std::ffi::CStr;
use std::os::raw::c_char;

use super::window::Window;

unsafe extern "system" fn vulkan_debug_utils_callback(
	message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
	message_type: vk::DebugUtilsMessageTypeFlagsEXT,
	p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
	_p_user_data: *mut c_void,
) -> vk::Bool32 {
	let severity = match message_severity {
		vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "[Verbose]",
		vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "[Warning]",
		vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "[Error]",
		vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "[Info]",
		_ => "[Unknown]",
	};
	let types = match message_type {
		vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[General]",
		vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[Performance]",
		vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[Validation]",
		_ => "[Unknown]",
	};
	let message = CStr::from_ptr((*p_callback_data).p_message);
	println!("[Debug]{}{}{:?}", severity, types, message);

	vk::FALSE
}

pub struct Instance {
	pub entry: ash::Entry,
	pub instance: ash::Instance,
	_debug_utils_loader: ash::extensions::ext::DebugUtils,
	_debug_messager: vk::DebugUtilsMessengerEXT,
}

impl Drop for Instance {
	fn drop(&mut self) {
		unsafe {
			#[cfg(debug_assertions)]
			self._debug_utils_loader
				.destroy_debug_utils_messenger(self._debug_messager, None);
			self.instance.destroy_instance(None);
		}
	}
}

impl Instance {
	#![allow(dead_code)]
	fn required_extension_names(window: &Window) -> Vec<*const i8> {
		let surface_extensions = ash_window::enumerate_required_extensions(&window.window).unwrap();
		let mut extension_names_raw = surface_extensions
			.iter()
			.map(|ext| ext.as_ptr())
			.collect::<Vec<_>>();
		extension_names_raw.push(DebugUtils::name().as_ptr());
		extension_names_raw
	}

	unsafe fn create_instance(entry: &ash::Entry, window: &Window) -> ash::Instance {
		let app_name = CStr::from_bytes_with_nul_unchecked(b"Vulkan App\0");

		let _layer_names = [CStr::from_bytes_with_nul_unchecked(
			b"VK_LAYER_KHRONOS_validation\0",
		)];

		#[cfg(debug_assertions)]
		let layers_names_raw: Vec<*const c_char> = _layer_names
			.iter()
			.map(|raw_name| raw_name.as_ptr())
			.collect();
		#[cfg(not(debug_assertions))]
		let layers_names_raw: Vec<*const c_char> = Vec::new();

		let extension_names_raw = Instance::required_extension_names(window);

		let appinfo = vk::ApplicationInfo::builder()
			.application_name(app_name)
			.application_version(0)
			.engine_name(app_name)
			.engine_version(0)
			.api_version(vk::make_api_version(0, 1, 2, 198));

		let create_info = vk::InstanceCreateInfo::builder()
			.application_info(&appinfo)
			.enabled_layer_names(&layers_names_raw)
			.enabled_extension_names(&extension_names_raw);

		entry
			.create_instance(&create_info, None)
			.expect("Failed to create the ash::instance")
	}

	fn populate_debug_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
		vk::DebugUtilsMessengerCreateInfoEXT {
			s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
			p_next: ptr::null(),
			flags: vk::DebugUtilsMessengerCreateFlagsEXT::empty(),
			message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING |
				// vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE |
				// vk::DebugUtilsMessageSeverityFlagsEXT::INFO |
				vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
			message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
				| vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
				| vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
			pfn_user_callback: Some(vulkan_debug_utils_callback),
			// pfn_user_callback: None,
			p_user_data: ptr::null_mut(),
		}
	}

	fn setup_debug_utils(
		entry: &ash::Entry,
		instance: &ash::Instance,
	) -> (DebugUtils, vk::DebugUtilsMessengerEXT) {
		let debug_utils_loader = DebugUtils::new(entry, instance);

		// if !cfg!(debug_assertions) {
		// 	(debug_utils_loader, ash::vk::DebugUtilsMessengerEXT::null())
		// } else {
		// 	let messenger_ci = Instance::populate_debug_messenger_create_info();

		// 	let utils_messenger = unsafe {
		// 		debug_utils_loader
		// 			.create_debug_utils_messenger(&messenger_ci, None)
		// 			.expect("Debug Utils Callback")
		// 	};

		// 	(debug_utils_loader, utils_messenger)
		// }
		(debug_utils_loader, ash::vk::DebugUtilsMessengerEXT::null())
	}

	pub fn new(window: &Window) -> Instance {
		let (entry, instance, debug_utils_loader, debug_messager) = unsafe {
			let entry = ash::Entry::load()
				.expect("Failed to load vulkan functions, is Vulkan SDK installed ?");
			let instance = Instance::create_instance(&entry, window);
			let (debug_utils_loader, debug_messager) =
				Instance::setup_debug_utils(&entry, &instance);
			(entry, instance, debug_utils_loader, debug_messager)
		};
		Instance {
			entry,
			instance,
			_debug_utils_loader: debug_utils_loader,
			_debug_messager: debug_messager,
		}
	}
}
