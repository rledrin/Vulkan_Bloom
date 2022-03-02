use ash::extensions::khr;
use ash::vk;

use super::device::Device;
use super::instance::Instance;
use super::window::Window;

pub struct Surface {
	pub surface: vk::SurfaceKHR,
	pub surface_loader: khr::Surface,
	pub surface_format: vk::SurfaceFormatKHR,
	pub desired_format: vk::Format,
	pub surface_resolution: vk::Extent2D,
	pub pre_transform: vk::SurfaceTransformFlagsKHR,
	pub desired_image_count: u32,
}

impl Drop for Surface {
	fn drop(&mut self) {
		unsafe {
			self.surface_loader.destroy_surface(self.surface, None);
		};
	}
}

impl Surface {
	#![allow(dead_code)]
	pub fn new(instance: &Instance, window: &Window, device: &Device) -> Surface {
		let surface = unsafe {
			ash_window::create_surface(&instance.entry, &instance.instance, &window.window, None)
				.expect("Failed to create the vulkan surface.")
		};
		let surface_loader = khr::Surface::new(&instance.entry, &instance.instance);

		let surface_format = unsafe {
			surface_loader
				.get_physical_device_surface_formats(device.physical_device, surface)
				.expect("Failed to get the surface formats.")[0]
		};

		let surface_capabilities = unsafe {
			surface_loader
				.get_physical_device_surface_capabilities(device.physical_device, surface)
				.expect("Failed to get the surface capabilities.")
		};

		let surface_resolution = match surface_capabilities.current_extent.width {
			std::u32::MAX => vk::Extent2D {
				width: window.window_extent.width,
				height: window.window_extent.width,
			},
			_ => surface_capabilities.current_extent,
		};

		let mut desired_image_count = surface_capabilities.min_image_count + 1;
		if surface_capabilities.max_image_count > 0
			&& desired_image_count > surface_capabilities.max_image_count
		{
			desired_image_count = surface_capabilities.max_image_count;
		}

		let pre_transform = if surface_capabilities
			.supported_transforms
			.contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
		{
			vk::SurfaceTransformFlagsKHR::IDENTITY
		} else {
			surface_capabilities.current_transform
		};

		// let desired_format = surface_format.format;
		// let desired_format = vk::Format::A2B10G10R10_UNORM_PACK32;
		let desired_format = vk::Format::R16G16B16A16_SFLOAT;
		// let desired_format = vk::Format::R16G16B16A16_UNORM;

		Surface {
			surface,
			surface_loader,
			surface_format,
			desired_format,
			surface_resolution,
			pre_transform,
			desired_image_count,
		}
	}
}
