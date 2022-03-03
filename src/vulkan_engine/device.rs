use std::sync::{Arc, Mutex};
use std::{ffi::CStr, os::raw::c_char};

use ash::vk;
use gpu_alloc::{Config, GpuAllocator};
use gpu_alloc_ash::device_properties;

use super::instance::Instance;

struct QueueFamilyIndices {
	family_index: Option<u32>,
}

impl QueueFamilyIndices {
	pub fn is_complete(&self) -> bool {
		self.family_index.is_some()
	}
}

pub struct Device {
	pub physical_device: vk::PhysicalDevice,
	pub allocator: Arc<Mutex<GpuAllocator<vk::DeviceMemory>>>,
	pub queue_family_index: u32,
	pub graphic_queue: vk::Queue,
	pub compute_queue: vk::Queue,
	pub transfer_queue: vk::Queue,
	pub present_queue: vk::Queue,
	pub device: Arc<ash::Device>,
}

impl Drop for Device {
	fn drop(&mut self) {
		unsafe {
			self.allocator
				.lock()
				.expect("Failed to lock the allocator in drop.")
				.cleanup(gpu_alloc_ash::AshMemoryDevice::wrap(&self.device));
			self.device.destroy_device(None);
		};
	}
}

impl Device {
	#![allow(dead_code)]
	fn find_queue_family(
		instance: &ash::Instance,
		physical_device: vk::PhysicalDevice,
	) -> QueueFamilyIndices {
		let queue_families =
			unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

		let mut queue_family_indices = QueueFamilyIndices { family_index: None };

		for (index, queue_family) in queue_families.iter().enumerate() {
			if queue_family.queue_count > 0
				&& queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS)
				&& queue_family.queue_flags.contains(vk::QueueFlags::TRANSFER)
				&& queue_family.queue_flags.contains(vk::QueueFlags::COMPUTE)
			{
				queue_family_indices.family_index = Some(index as u32);
			}
			if queue_family_indices.is_complete() {
				break;
			}
		}

		queue_family_indices
	}

	fn is_physical_device_suitable(
		instance: &ash::Instance,
		physical_device: vk::PhysicalDevice,
	) -> QueueFamilyIndices {
		#[cfg(debug_assertions)]
		{
			let device_properties =
				unsafe { instance.get_physical_device_properties(physical_device) };
			// let device_features = unsafe { instance.get_physical_device_features(physical_device) };

			let device_queue_families =
				unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

			let device_type = match device_properties.device_type {
				vk::PhysicalDeviceType::CPU => "Cpu",
				vk::PhysicalDeviceType::INTEGRATED_GPU => "Integrated GPU",
				vk::PhysicalDeviceType::DISCRETE_GPU => "Discrete GPU",
				vk::PhysicalDeviceType::VIRTUAL_GPU => "Virtual GPU",
				vk::PhysicalDeviceType::OTHER => "Unknown",
				_ => panic!(),
			};

			let device_name = unsafe {
				CStr::from_ptr(device_properties.device_name.as_ptr())
					.to_str()
					.expect("Failed to convert vulkan raw string.")
					.to_owned()
			};

			println!(
				"\tDevice Name: {}, id: {}, type: {}",
				device_name, device_properties.device_id, device_type
			);

			let major_version = vk::api_version_major(device_properties.api_version);
			let minor_version = vk::api_version_minor(device_properties.api_version);
			let patch_version = vk::api_version_patch(device_properties.api_version);

			println!(
				"\tAPI Version: {}.{}.{}",
				major_version, minor_version, patch_version
			);

			println!("\tSupport Queue Family: {}", device_queue_families.len());
			println!("\t\tQueue Count | Graphics, Compute, Transfer, Sparse Binding");

			for queue_family in device_queue_families.iter() {
				let is_graphics_support =
					if queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
						"support"
					} else {
						"unsupport"
					};
				let is_compute_support =
					if queue_family.queue_flags.contains(vk::QueueFlags::COMPUTE) {
						"support"
					} else {
						"unsupport"
					};
				let is_transfer_support =
					if queue_family.queue_flags.contains(vk::QueueFlags::TRANSFER) {
						"support"
					} else {
						"unsupport"
					};
				let is_sparse_support = if queue_family
					.queue_flags
					.contains(vk::QueueFlags::SPARSE_BINDING)
				{
					"support"
				} else {
					"unsupport"
				};

				println!(
					"\t\t{}\t    | {},  {},  {},  {}",
					queue_family.queue_count,
					is_graphics_support,
					is_compute_support,
					is_transfer_support,
					is_sparse_support
				);
			}
		}

		// #[cfg(debug_assertions)]
		// println!(
		// 	"\tGeometry Shader support: {}",
		// 	if device_features.geometry_shader == 1 {
		// 		"Support"
		// 	} else {
		// 		"Unsupport"
		// 	}
		// );

		Device::find_queue_family(instance, physical_device)
	}

	fn pick_physical_device(instance: &ash::Instance) -> vk::PhysicalDevice {
		let physical_devices = unsafe {
			instance
				.enumerate_physical_devices()
				.expect("Failed to enumerate Physical Devices!")
		};

		#[cfg(debug_assertions)]
		println!(
			"{} devices (GPU) found with vulkan support.",
			physical_devices.len()
		);

		let mut result = None;
		for &physical_device in physical_devices.iter() {
			if Device::is_physical_device_suitable(instance, physical_device).is_complete()
				&& result.is_none()
			{
				result = Some(physical_device)
			}
		}
		#[cfg(debug_assertions)]
		println!("\n");

		match result {
			None => panic!("Failed to find a suitable GPU!"),
			Some(physical_device) => physical_device,
		}
	}

	fn required_extension_names() -> Vec<*const i8> {
		[ash::extensions::khr::Swapchain::name().as_ptr()].to_vec()
	}

	fn create_logical_device(
		instance: &ash::Instance,
		physical_device: vk::PhysicalDevice,
	) -> (ash::Device, u32, vk::Queue, vk::Queue, vk::Queue, vk::Queue) {
		let family_index = Device::find_queue_family(instance, physical_device)
			.family_index
			.expect("No queue family index.");

		let queue_priorities = [1.0f32, 1.0f32, 1.0f32, 1.0f32];

		let queue_info = [vk::DeviceQueueCreateInfo::builder()
			.queue_family_index(family_index)
			.queue_priorities(&queue_priorities)
			.build()];

		let physical_device_features = vk::PhysicalDeviceFeatures::builder()
			.dual_src_blend(true)
			.build();
		let mut physical_device_vulkan_12_features = vk::PhysicalDeviceVulkan12Features::builder()
			.buffer_device_address(true)
			.build();

		let mut physical_device_features_2 = vk::PhysicalDeviceFeatures2::builder()
			.features(physical_device_features)
			.push_next(&mut physical_device_vulkan_12_features)
			.build();

		let device_extension = Device::required_extension_names();
		let _layer_names = unsafe {
			[CStr::from_bytes_with_nul_unchecked(
				b"VK_LAYER_KHRONOS_validation\0",
			)]
		};

		#[cfg(debug_assertions)]
		let enable_layer_names: Vec<*const c_char> = _layer_names
			.iter()
			.map(|raw_name| raw_name.as_ptr())
			.collect();
		#[cfg(not(debug_assertions))]
		let enable_layer_names: Vec<*const c_char> = Vec::new();

		let device_create_info = vk::DeviceCreateInfo::builder()
			.queue_create_infos(&queue_info)
			.enabled_extension_names(&device_extension)
			.enabled_layer_names(&enable_layer_names)
			.push_next(&mut physical_device_features_2);

		let device = unsafe {
			instance
				.create_device(physical_device, &device_create_info, None)
				.expect("Failed to create the logical Device!")
		};
		let queue_family_props = unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
		let queue_family_props = queue_family_props[family_index as usize];

		let (graphic_queue, compute_queue, transfer_queue, present_queue) = unsafe {
			let graphic = device.get_device_queue(family_index, std::cmp::min(0, queue_family_props.queue_count));
			let compute = device.get_device_queue(family_index, std::cmp::min(1, queue_family_props.queue_count));
			let transfer = device.get_device_queue(family_index, std::cmp::min(2, queue_family_props.queue_count));
			let present = device.get_device_queue(family_index, std::cmp::min(3, queue_family_props.queue_count));
			(graphic, compute, transfer, present)
		};



		(
			device,
			family_index,
			graphic_queue,
			compute_queue,
			transfer_queue,
			present_queue,
		)
	}

	pub fn new(instance: &Instance) -> Device {
		let physical_device = Device::pick_physical_device(&instance.instance);

		let (
			device,
			queue_family_index,
			graphic_queue,
			compute_queue,
			transfer_queue,
			present_queue,
		) = Device::create_logical_device(&instance.instance, physical_device);

		let device = Arc::new(device);

		let config = Config::i_am_potato();

		let version = instance
			.entry
			.try_enumerate_instance_version()
			.expect("Failed to enumerate instance version")
			.unwrap_or_else(|| vk::make_api_version(0, 1, 3, 0));

		let properties = unsafe {
			device_properties(&instance.instance, version, physical_device)
				.expect("Failed to query the device properties for the allocator.")
		};

		// let mut formats = Vec::new();
		// formats.push(vk::Format::D32_SFLOAT);
		// formats.push(vk::Format::D32_SFLOAT_S8_UINT);
		// formats.push(vk::Format::D24_UNORM_S8_UINT);

		// for f in formats.iter() {
		// 	let a = unsafe {
		// 		instance
		// 			.instance
		// 			.get_physical_device_format_properties(physical_device, *f)
		// 	};
		// 	println!("format: {:?}, properties: {:?}", f, a);

		// }
		// println!("");
		// println!("");

		let allocator = GpuAllocator::<vk::DeviceMemory>::new(config, properties);

		let allocator = Arc::new(Mutex::new(allocator));

		Device {
			physical_device,
			allocator,
			queue_family_index,
			graphic_queue,
			compute_queue,
			transfer_queue,
			present_queue,
			device,
		}
	}
}
