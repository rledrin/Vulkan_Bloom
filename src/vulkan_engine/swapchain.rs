use std::sync::Arc;

use ash::extensions::khr;
use ash::vk;

use super::device::Device;
use super::image::{self, Image};
use super::instance::Instance;
use super::renderpass::{self, RenderPass};
use super::surface::{self, Surface};

pub struct Swapchain {
	pub swapchain_loader: khr::Swapchain,
	pub swapchain: vk::SwapchainKHR,
	pub swapchain_extent: vk::Extent2D,
	pub swapchain_images: Vec<vk::Image>,
	pub swapchain_image_views: Vec<vk::ImageView>,
	pub swapchain_image_sampler: vk::Sampler,
	pub depth_stencil_image: Option<Image>,
	pub swapchain_framebuffers: Vec<vk::Framebuffer>,
	pub swapchain_ui_framebuffers: Vec<vk::Framebuffer>,
	pub max_image_in_flight: usize,
	swapchain_create_info: vk::SwapchainCreateInfoKHR,
	device: Arc<ash::Device>,
}

impl Drop for Swapchain {
	fn drop(&mut self) {
		unsafe {
			self.device
				.destroy_sampler(self.swapchain_image_sampler, None);
			for i in 0..self.swapchain_image_views.len() {
				self.device
					.destroy_image_view(self.swapchain_image_views[i], None);
				self.device
					.destroy_framebuffer(self.swapchain_framebuffers[i], None);
				self.device
					.destroy_framebuffer(self.swapchain_ui_framebuffers[i], None);
			}
			self.swapchain_loader
				.destroy_swapchain(self.swapchain, None);
		};
	}
}

impl Swapchain {
	#![allow(dead_code)]
	/// For the Option parameters: set to None for the default values
	///
	/// The default values:
	///
	/// present_mode: vk::PresentModeKHR::MAILBOX
	///
	/// image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT
	///
	/// sharing_mode: vk::SharingMode::EXCLUSIVE
	pub fn new(
		instance: &Instance,
		surface: &Surface,
		device: &Device,
		present_mode: Option<vk::PresentModeKHR>,
		image_usage: Option<vk::ImageUsageFlags>,
		depth_stencil_image: Option<Image>,
		renderpass: &RenderPass,
		ui_renderpass: &RenderPass,
	) -> Swapchain {
		let present_modes = unsafe {
			surface
				.surface_loader
				.get_physical_device_surface_present_modes(device.physical_device, surface.surface)
				.unwrap()
		};
		let chosen_present_mode = present_modes
			.iter()
			.cloned()
			.find(|&mode| mode == present_mode.unwrap_or(vk::PresentModeKHR::MAILBOX))
			.unwrap_or_else(|| panic!("Couldn't find {:?} as present mode.", present_mode));

		let swapchain_loader = khr::Swapchain::new(&instance.instance, &device.device);

		// println!("surface format: {:?}\n", surface.surface_format);
		// println!(
		// 	"swapchain format: {:?}, colorSpace: {:?}\n",
		// 	surface.desired_format, surface.surface_format.color_space
		// );

		// let format = surface.surface_format.format;
		let format = surface.desired_format;

		let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
			.surface(surface.surface)
			.min_image_count(surface.desired_image_count)
			.image_color_space(surface.surface_format.color_space)
			.image_format(format)
			.image_extent(surface.surface_resolution)
			.image_usage(image_usage.unwrap_or(vk::ImageUsageFlags::COLOR_ATTACHMENT))
			.image_sharing_mode(vk::SharingMode::EXCLUSIVE)
			.pre_transform(surface.pre_transform)
			.composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
			.present_mode(chosen_present_mode)
			.clipped(true)
			.image_array_layers(1)
			.build();

		let swapchain = unsafe {
			swapchain_loader
				.create_swapchain(&swapchain_create_info, None)
				.expect("Failed to create the Swapchain.")
		};

		let swapchain_images = unsafe {
			swapchain_loader
				.get_swapchain_images(swapchain)
				.expect("Failed to get Swapchain Images.")
		};

		let mut attachments = Vec::with_capacity(2);
		if depth_stencil_image.is_some() {
			attachments.resize(2, Default::default());
			attachments[1] = depth_stencil_image.as_ref().unwrap().image_view;
		} else {
			attachments.resize(1, Default::default());
		}

		let sampler_create_info = vk::SamplerCreateInfo::builder()
			.min_filter(vk::Filter::LINEAR)
			.mag_filter(vk::Filter::LINEAR)
			.mipmap_mode(vk::SamplerMipmapMode::LINEAR)
			.address_mode_u(vk::SamplerAddressMode::CLAMP_TO_BORDER)
			.address_mode_v(vk::SamplerAddressMode::CLAMP_TO_BORDER)
			.address_mode_w(vk::SamplerAddressMode::CLAMP_TO_BORDER)
			.mip_lod_bias(1.0)
			.anisotropy_enable(false)
			.max_anisotropy(1.0)
			.compare_enable(false)
			.compare_op(vk::CompareOp::ALWAYS)
			.min_lod(0.0)
			.max_lod(1.0)
			.border_color(vk::BorderColor::FLOAT_OPAQUE_BLACK)
			.unnormalized_coordinates(false)
			.build();

		let swapchain_image_sampler = unsafe {
			device
				.device
				.create_sampler(&sampler_create_info, None)
				.expect("Failed to create an image sampler(swapchain).")
		};

		let mut swapchain_image_views = Vec::with_capacity(swapchain_images.len());
		let mut swapchain_framebuffers = Vec::with_capacity(swapchain_images.len());
		let mut swapchain_ui_framebuffers = Vec::with_capacity(swapchain_images.len());
		for &image in swapchain_images.iter() {
			let image_view_create_info = vk::ImageViewCreateInfo::builder()
				.view_type(vk::ImageViewType::TYPE_2D)
				.format(format)
				.components(
					vk::ComponentMapping::builder()
						.r(vk::ComponentSwizzle::IDENTITY)
						.g(vk::ComponentSwizzle::IDENTITY)
						.b(vk::ComponentSwizzle::IDENTITY)
						.a(vk::ComponentSwizzle::IDENTITY)
						.build(),
				)
				.subresource_range(
					vk::ImageSubresourceRange::builder()
						.aspect_mask(vk::ImageAspectFlags::COLOR)
						.base_mip_level(0)
						.level_count(1)
						.base_array_layer(0)
						.layer_count(1)
						.build(),
				)
				.image(image)
				.build();
			let imageview = unsafe {
				device
					.device
					.create_image_view(&image_view_create_info, None)
					.expect("Failed to create Image View!")
			};
			attachments[0] = imageview;
			let framebuffer_create_info = vk::FramebufferCreateInfo::builder()
				.flags(vk::FramebufferCreateFlags::empty())
				.render_pass(renderpass.renderpass)
				.attachments(&attachments)
				.width(surface.surface_resolution.width)
				.height(surface.surface_resolution.height)
				.layers(1)
				.build();

			let ui_framebuffer_create_info = vk::FramebufferCreateInfo::builder()
				.flags(vk::FramebufferCreateFlags::empty())
				.render_pass(ui_renderpass.renderpass)
				.attachments(&[imageview])
				.width(surface.surface_resolution.width)
				.height(surface.surface_resolution.height)
				.layers(1)
				.build();

			let framebuffer = unsafe {
				device
					.device
					.create_framebuffer(&framebuffer_create_info, None)
					.expect("Failed to create a framebuffer.")
			};

			let ui_framebuffer = unsafe {
				device
					.device
					.create_framebuffer(&ui_framebuffer_create_info, None)
					.expect("Failed to create a framebuffer.")
			};
			swapchain_image_views.push(imageview);
			swapchain_framebuffers.push(framebuffer);
			swapchain_ui_framebuffers.push(ui_framebuffer);
		}

		let swapchain_extent = surface.surface_resolution;

		let max_image_in_flight = swapchain_framebuffers.len();

		Swapchain {
			swapchain_loader,
			swapchain,
			swapchain_extent,
			swapchain_images,
			swapchain_image_views,
			swapchain_image_sampler,
			depth_stencil_image,
			swapchain_framebuffers,
			swapchain_ui_framebuffers,
			max_image_in_flight,
			swapchain_create_info,
			device: device.device.clone(),
		}
	}

	pub fn recreate(
		&mut self,
		surface: &surface::Surface,
		renderpass: &renderpass::RenderPass,
		ui_renderpass: &renderpass::RenderPass,
		depth_image: Option<image::Image>,
	) {
		unsafe {
			self.device
				.destroy_sampler(self.swapchain_image_sampler, None);

			for i in 0..self.swapchain_image_views.len() {
				self.device
					.destroy_image_view(self.swapchain_image_views[i], None);
				self.device
					.destroy_framebuffer(self.swapchain_framebuffers[i], None);
				self.device
					.destroy_framebuffer(self.swapchain_ui_framebuffers[i], None);
			}
			self.swapchain_loader
				.destroy_swapchain(self.swapchain, None);
		}
		self.swapchain_framebuffers.clear();
		self.swapchain_ui_framebuffers.clear();
		self.swapchain_image_views.clear();

		self.swapchain_create_info.image_extent = surface.surface_resolution;
		self.swapchain = unsafe {
			self.swapchain_loader
				.create_swapchain(&self.swapchain_create_info, None)
				.expect("Failed to create the Swapchain.")
		};

		self.swapchain_images = unsafe {
			self.swapchain_loader
				.get_swapchain_images(self.swapchain)
				.expect("Failed to get Swapchain Images.")
		};

		let sampler_create_info = vk::SamplerCreateInfo::builder()
			.min_filter(vk::Filter::LINEAR)
			.mag_filter(vk::Filter::LINEAR)
			.mipmap_mode(vk::SamplerMipmapMode::LINEAR)
			.address_mode_u(vk::SamplerAddressMode::CLAMP_TO_BORDER)
			.address_mode_v(vk::SamplerAddressMode::CLAMP_TO_BORDER)
			.address_mode_w(vk::SamplerAddressMode::CLAMP_TO_BORDER)
			.mip_lod_bias(1.0)
			.anisotropy_enable(false)
			.max_anisotropy(1.0)
			.compare_enable(false)
			.compare_op(vk::CompareOp::ALWAYS)
			.min_lod(0.0)
			.max_lod(1.0)
			.border_color(vk::BorderColor::FLOAT_OPAQUE_BLACK)
			.unnormalized_coordinates(false)
			.build();

		self.swapchain_image_sampler = unsafe {
			self.device
				.create_sampler(&sampler_create_info, None)
				.expect("Failed to create an image sampler(swapchain).")
		};

		let mut attachments = Vec::with_capacity(2);
		if self.depth_stencil_image.is_some() {
			drop(self.depth_stencil_image.as_ref().unwrap());
			self.depth_stencil_image = depth_image;
			attachments.resize(2, Default::default());
			attachments[1] = self.depth_stencil_image.as_ref().unwrap().image_view;
		} else {
			attachments.resize(1, Default::default());
		}

		for (_, image) in self.swapchain_images.iter().enumerate() {
			let image_view_create_info = vk::ImageViewCreateInfo::builder()
				.view_type(vk::ImageViewType::TYPE_2D)
				.format(self.swapchain_create_info.image_format)
				.components(
					vk::ComponentMapping::builder()
						.r(vk::ComponentSwizzle::IDENTITY)
						.g(vk::ComponentSwizzle::IDENTITY)
						.b(vk::ComponentSwizzle::IDENTITY)
						.a(vk::ComponentSwizzle::IDENTITY)
						.build(),
				)
				.subresource_range(
					vk::ImageSubresourceRange::builder()
						.aspect_mask(vk::ImageAspectFlags::COLOR)
						.base_mip_level(0)
						.level_count(1)
						.base_array_layer(0)
						.layer_count(1)
						.build(),
				)
				.image(*image)
				.build();
			let imageview = unsafe {
				self.device
					.create_image_view(&image_view_create_info, None)
					.expect("Failed to create Image View!")
			};

			attachments[0] = imageview;
			let framebuffer_create_info = vk::FramebufferCreateInfo::builder()
				.flags(vk::FramebufferCreateFlags::empty())
				.render_pass(renderpass.renderpass)
				.attachments(&attachments)
				.width(surface.surface_resolution.width)
				.height(surface.surface_resolution.height)
				.layers(1)
				.build();

			let ui_framebuffer_create_info = vk::FramebufferCreateInfo::builder()
				.flags(vk::FramebufferCreateFlags::empty())
				.render_pass(ui_renderpass.renderpass)
				.attachments(&[imageview])
				.width(surface.surface_resolution.width)
				.height(surface.surface_resolution.height)
				.layers(1)
				.build();

			let framebuffer = unsafe {
				self.device
					.create_framebuffer(&framebuffer_create_info, None)
					.expect("Failed to create a framebuffer.")
			};
			let ui_framebuffer = unsafe {
				self.device
					.create_framebuffer(&ui_framebuffer_create_info, None)
					.expect("Failed to create a framebuffer.")
			};
			self.swapchain_image_views.push(imageview);
			self.swapchain_framebuffers.push(framebuffer);
			self.swapchain_ui_framebuffers.push(ui_framebuffer);
		}
		self.swapchain_extent = surface.surface_resolution;
	}
}
