use std::sync::{Arc, Mutex};

use ash::vk::{self, ImageAspectFlags};
use gpu_alloc::{GpuAllocator, MemoryBlock, Request, UsageFlags};
use gpu_alloc_ash::AshMemoryDevice;

use super::{
	buffer::Buffer,
	command_buffer::{self, CommandBufferBuilder},
	device::{self, Device},
};

pub struct Image {
	pub image: vk::Image,
	pub image_view: vk::ImageView,
	pub mip_image_views: Vec<vk::ImageView>,
	pub image_sampler: Option<vk::Sampler>,
	pub block: Option<MemoryBlock<vk::DeviceMemory>>,
	pub extent: vk::Extent3D,
	pub memory_requierments: vk::MemoryRequirements,
	pub initial_layout: vk::ImageLayout,
	pub final_layout: vk::ImageLayout,
	pub current_layout: vk::ImageLayout,
	pub format: vk::Format,
	pub subresource_range: vk::ImageSubresourceRange,
	device: Arc<ash::Device>,
	allocator: Arc<Mutex<GpuAllocator<vk::DeviceMemory>>>,
}

impl Drop for Image {
	fn drop(&mut self) {
		unsafe {
			if let Some(sampler) = self.image_sampler {
				self.device.destroy_sampler(sampler, None);
			}
			for i in 0..self.subresource_range.level_count {
				self.device
					.destroy_image_view(self.mip_image_views[i as usize], None);
			}
			self.device.destroy_image_view(self.image_view, None);
			let block = std::mem::take(&mut self.block).unwrap();
			self.allocator
				.lock()
				.expect("Failed to lock the Allocator's Mutex in an image drop.")
				.dealloc(AshMemoryDevice::wrap(&self.device), block);
			self.device.destroy_image(self.image, None);
		}
	}
}

impl Image {
	#![allow(dead_code)]
	#![allow(clippy::too_many_arguments)]

	pub fn new(
		device: &Device,
		flags: vk::ImageCreateFlags,
		image_type: vk::ImageType,
		format: vk::Format,
		extent: vk::Extent3D,
		mip_levels: u32,
		array_layers: u32,
		tiling: vk::ImageTiling,
		usage: vk::ImageUsageFlags,
		queue_family_index: u32,
		initial_layout: vk::ImageLayout,
		final_layout: vk::ImageLayout,
		view_type: vk::ImageViewType,
		image_aspect: vk::ImageAspectFlags,
		allocation_type: UsageFlags,
	) -> Image {
		let image_create_info = vk::ImageCreateInfo::builder()
			.flags(flags)
			.image_type(image_type)
			.format(format)
			.extent(extent)
			.mip_levels(mip_levels)
			.array_layers(array_layers)
			.samples(vk::SampleCountFlags::TYPE_1)
			.tiling(tiling)
			.usage(usage)
			.sharing_mode(vk::SharingMode::EXCLUSIVE)
			.queue_family_indices(&[queue_family_index])
			.initial_layout(initial_layout)
			.build();

		let image = unsafe {
			device
				.device
				.create_image(&image_create_info, None)
				.expect("Failed to create an Image.")
		};

		let subresource_range = vk::ImageSubresourceRange::builder()
			.aspect_mask(image_aspect)
			.base_mip_level(0)
			.level_count(mip_levels)
			.base_array_layer(0)
			.layer_count(array_layers)
			.build();

		let image_view_create_info = vk::ImageViewCreateInfo::builder()
			.flags(vk::ImageViewCreateFlags::empty())
			.image(image)
			.view_type(view_type)
			.format(format)
			.subresource_range(subresource_range)
			.build();

		let memory_requierments = unsafe { device.device.get_image_memory_requirements(image) };

		let block = unsafe {
			device
				.allocator
				.lock()
				.expect("Failed to lock the allocator's mutex in and image's new.")
				.alloc(
					AshMemoryDevice::wrap(&device.device),
					Request {
						size: memory_requierments.size,
						align_mask: memory_requierments.alignment,
						usage: allocation_type,
						memory_types: !0,
					},
				)
				.expect("Failed to allocate an image.")
		};

		let (image_view, mip_image_views) = unsafe {
			device
				.device
				.bind_image_memory(image, *block.memory(), 0)
				.expect("Failed to bind image memory.");

			let image_view = device
				.device
				.create_image_view(&image_view_create_info, None)
				.expect("Failed to create an image view.");

			let mut mip_image_views = Vec::with_capacity(mip_levels as usize);
			// mip_image_views.push(image_view);
			for i in 0..mip_levels {
				let subresource_range = vk::ImageSubresourceRange::builder()
					.aspect_mask(image_aspect)
					.base_mip_level(i)
					.level_count(mip_levels - i)
					.base_array_layer(0)
					.layer_count(array_layers)
					.build();

				let image_view_create_info = vk::ImageViewCreateInfo::builder()
					.flags(vk::ImageViewCreateFlags::empty())
					.image(image)
					.view_type(view_type)
					.format(format)
					.subresource_range(subresource_range)
					.build();
				mip_image_views.push(
					device
						.device
						.create_image_view(&image_view_create_info, None)
						.expect("Failed to create an image view."),
				);
			}
			(image_view, mip_image_views)
		};

		Image {
			image,
			image_view,
			mip_image_views,
			image_sampler: Default::default(),
			block: Some(block),
			extent,
			memory_requierments,
			initial_layout,
			final_layout,
			current_layout: initial_layout,
			format,
			subresource_range,
			device: device.device.clone(),
			allocator: device.allocator.clone(),
		}
	}

	pub fn set_sampler(
		&mut self,
		min_filter: vk::Filter,
		mag_filter: vk::Filter,
		mipmap_mode: vk::SamplerMipmapMode,
		address_mode_u: vk::SamplerAddressMode,
		address_mode_v: vk::SamplerAddressMode,
		address_mode_w: vk::SamplerAddressMode,
		mip_lod_bias: f32,
		anisotropy_enable: bool,
		max_anisotropy: f32,
		compare_enable: bool,
		compare_op: vk::CompareOp,
		min_lod: f32,
		max_lod: f32,
		border_color: vk::BorderColor,
	) {
		let sampler_create_info = vk::SamplerCreateInfo::builder()
			.min_filter(min_filter)
			.mag_filter(mag_filter)
			.mipmap_mode(mipmap_mode)
			.address_mode_u(address_mode_u)
			.address_mode_v(address_mode_v)
			.address_mode_w(address_mode_w)
			.mip_lod_bias(mip_lod_bias)
			.anisotropy_enable(anisotropy_enable)
			.max_anisotropy(max_anisotropy)
			.compare_enable(compare_enable)
			.compare_op(compare_op)
			.min_lod(min_lod)
			.max_lod(max_lod)
			.border_color(border_color)
			.unnormalized_coordinates(false)
			.build();

		let image_sampler = unsafe {
			self.device
				.create_sampler(&sampler_create_info, None)
				.expect("Failed to create an image sampler.")
		};

		self.image_sampler = Some(image_sampler);
	}

	fn change_image_layout(
		device: &Device,
		image: &mut Image,
		command_buffer: &vk::CommandBuffer,
		old_layout: vk::ImageLayout,
		new_layout: vk::ImageLayout,
	) {
		let image_memory_barrier = vk::ImageMemoryBarrier::builder()
			.src_access_mask(vk::AccessFlags::empty())
			.dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
			.old_layout(old_layout)
			.new_layout(new_layout)
			.src_queue_family_index(device.queue_family_index)
			.dst_queue_family_index(device.queue_family_index)
			.subresource_range(image.subresource_range)
			.image(image.image)
			.build();

		unsafe {
			device.device.cmd_pipeline_barrier(
				*command_buffer,
				vk::PipelineStageFlags::TOP_OF_PIPE,
				vk::PipelineStageFlags::TRANSFER,
				vk::DependencyFlags::empty(),
				&[],
				&[],
				&[image_memory_barrier],
			);
		};
		image.current_layout = new_layout;
	}

	fn change_vk_image_layout(
		device: &Device,
		image: vk::Image,
		subresource_range: vk::ImageSubresourceRange,
		command_buffer: &vk::CommandBuffer,
		old_layout: vk::ImageLayout,
		new_layout: vk::ImageLayout,
	) {
		let image_memory_barrier = vk::ImageMemoryBarrier::builder()
			.src_access_mask(vk::AccessFlags::empty())
			.dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
			.old_layout(old_layout)
			.new_layout(new_layout)
			.src_queue_family_index(device.queue_family_index)
			.dst_queue_family_index(device.queue_family_index)
			.subresource_range(subresource_range)
			.image(image)
			.build();

		unsafe {
			device.device.cmd_pipeline_barrier(
				*command_buffer,
				vk::PipelineStageFlags::TOP_OF_PIPE,
				vk::PipelineStageFlags::TRANSFER,
				vk::DependencyFlags::empty(),
				&[],
				&[],
				&[image_memory_barrier],
			);
		};
	}

	pub fn change_layout(
		&mut self,
		device: &Device,
		command_builder: &command_buffer::CommandBufferBuilder,
		old_layout: vk::ImageLayout,
		new_layout: vk::ImageLayout,
	) {
		let command_buffer = command_builder.build();
		Image::change_image_layout(device, self, &command_buffer, old_layout, new_layout);
		unsafe {
			self.device
				.end_command_buffer(command_buffer)
				.expect("Failed to stop a command buffer.");
		};

		let submit_info = [vk::SubmitInfo::builder()
			.command_buffers(&[command_buffer])
			.build()];
		unsafe {
			self.device
				.queue_submit(device.transfer_queue, &submit_info, vk::Fence::null())
				.expect("Failed to submit to transfer queue.");
			self.device
				.queue_wait_idle(device.transfer_queue)
				.expect("Failed to wait queue idle");

			self.device
				.free_command_buffers(command_builder.command_pool.command_pool, &[command_buffer]);
		};
	}

	pub fn write<T>(&mut self, data: &[T]) {
		unsafe {
			let (_, bytes, _) = data.align_to::<u8>();

			self.block
				.as_mut()
				.unwrap()
				.write_bytes(AshMemoryDevice::wrap(&self.device), 0, bytes)
				.expect("Failed to write to a buffer.");
		};
	}

	pub fn write_to_vram<T>(
		&mut self,
		device: &Device,
		command_builder: &CommandBufferBuilder,
		data: Vec<T>,
	) {
		let mut staging_buffer = Buffer::new(
			device,
			vk::BufferCreateFlags::empty(),
			self.memory_requierments.size,
			vk::BufferUsageFlags::TRANSFER_SRC,
			vk::SharingMode::EXCLUSIVE,
			UsageFlags::UPLOAD,
		);
		staging_buffer.write(0, data);

		let copy_region = [vk::BufferImageCopy {
			buffer_offset: 0,
			buffer_row_length: 0,
			buffer_image_height: 0,
			image_subresource: vk::ImageSubresourceLayers::builder()
				.aspect_mask(self.subresource_range.aspect_mask)
				.base_array_layer(self.subresource_range.base_array_layer)
				.layer_count(self.subresource_range.layer_count)
				.mip_level(self.subresource_range.base_mip_level)
				.build(),
			image_offset: vk::Offset3D::builder().build(),
			image_extent: self.extent,
		}];

		let command_buffer = command_builder.build();

		Image::change_image_layout(
			device,
			self,
			&command_buffer,
			self.initial_layout,
			vk::ImageLayout::TRANSFER_DST_OPTIMAL,
		);

		unsafe {
			self.device.cmd_copy_buffer_to_image(
				command_buffer,
				*staging_buffer.buffer,
				self.image,
				vk::ImageLayout::TRANSFER_DST_OPTIMAL,
				&copy_region,
			);
		};

		Image::change_image_layout(
			device,
			self,
			&command_buffer,
			vk::ImageLayout::TRANSFER_DST_OPTIMAL,
			self.final_layout,
		);

		unsafe {
			self.device
				.end_command_buffer(command_buffer)
				.expect("Failed to stop a command buffer.");
		};

		let submit_info = [vk::SubmitInfo::builder()
			.command_buffers(&[command_buffer])
			.build()];
		unsafe {
			self.device
				.queue_submit(device.transfer_queue, &submit_info, vk::Fence::null())
				.expect("Failed to submit to transfer queue.");
			self.device
				.queue_wait_idle(device.transfer_queue)
				.expect("Failed to wait queue idle");

			self.device
				.free_command_buffers(command_builder.command_pool.command_pool, &[command_buffer]);
		};
	}

	pub fn write_from_image(
		&mut self,
		device: &device::Device,
		command_builder: Option<&CommandBufferBuilder>,
		command_buffer: Option<&vk::CommandBuffer>,
		src_image: &vk::Image,
		src_image_aspect_mask: ImageAspectFlags,
		src_image_layout: vk::ImageLayout,
	) {
		let subresource_layer = vk::ImageSubresourceLayers::builder()
			.aspect_mask(self.subresource_range.aspect_mask)
			.base_array_layer(self.subresource_range.base_array_layer)
			.layer_count(self.subresource_range.layer_count)
			.mip_level(self.subresource_range.base_mip_level)
			.build();
		let region = vk::ImageCopy::builder()
			.dst_offset(vk::Offset3D::builder().build())
			.extent(vk::Extent3D::builder().build())
			.dst_subresource(subresource_layer)
			.src_offset(vk::Offset3D::builder().build())
			.src_subresource(
				vk::ImageSubresourceLayers::builder()
					.aspect_mask(src_image_aspect_mask)
					.base_array_layer(0)
					.layer_count(1)
					.mip_level(0)
					.build(),
			)
			.build();

		let subresource_range = vk::ImageSubresourceRange::builder()
			.aspect_mask(src_image_aspect_mask)
			.base_array_layer(0)
			.base_mip_level(0)
			.layer_count(1)
			.level_count(1)
			.build();

		let command_buffer = if command_buffer.is_some() {
			*command_buffer.unwrap()
		} else {
			command_builder.unwrap().build()
		};

		Image::change_vk_image_layout(
			device,
			*src_image,
			subresource_range,
			&command_buffer,
			src_image_layout,
			vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
		);
		let old_layout = self.current_layout;
		Image::change_image_layout(
			device,
			self,
			&command_buffer,
			old_layout,
			vk::ImageLayout::TRANSFER_DST_OPTIMAL,
		);
		unsafe {
			self.device.cmd_copy_image(
				command_buffer,
				*src_image,
				src_image_layout,
				self.image,
				self.current_layout,
				&[region],
			);
		};
		Image::change_vk_image_layout(
			device,
			*src_image,
			subresource_range,
			&command_buffer,
			vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
			src_image_layout,
		);
		Image::change_image_layout(
			device,
			self,
			&command_buffer,
			vk::ImageLayout::TRANSFER_DST_OPTIMAL,
			old_layout,
		);
	}

	pub fn read<T>(&mut self, data: &mut Vec<T>) {
		let image_len = self.memory_requierments.size as usize / std::mem::size_of::<T>();
		if data.capacity() < image_len {
			data.reserve(image_len - data.capacity())
		}
		unsafe {
			data.set_len(image_len);

			let (_, bytes, _) = data.align_to_mut::<u8>();
			self.block
				.as_mut()
				.unwrap()
				.read_bytes(AshMemoryDevice::wrap(&self.device), 0, bytes)
				.expect("Failed to read to a buffer.");
		};
	}

	pub fn read_from_vram<T>(
		&mut self,
		device: &Device,
		command_builder: &CommandBufferBuilder,
		data: &mut Vec<T>,
	) {
		let mut size = (std::mem::size_of::<T>() * data.len()) as u64;
		if size == 0 {
			size = (std::mem::size_of::<T>() * data.capacity()) as u64;
		}

		let mut staging_buffer = Buffer::new(
			device,
			vk::BufferCreateFlags::empty(),
			size,
			vk::BufferUsageFlags::TRANSFER_DST,
			vk::SharingMode::EXCLUSIVE,
			UsageFlags::DOWNLOAD,
		);
		let copy_region = [vk::BufferImageCopy {
			buffer_offset: 0,
			buffer_row_length: 0,
			buffer_image_height: 0,
			image_subresource: vk::ImageSubresourceLayers::builder()
				.aspect_mask(self.subresource_range.aspect_mask)
				.base_array_layer(self.subresource_range.base_array_layer)
				.layer_count(self.subresource_range.layer_count)
				.mip_level(self.subresource_range.base_mip_level)
				.build(),
			image_offset: vk::Offset3D::builder().build(),
			image_extent: self.extent,
		}];

		let command_buffer = command_builder.build();

		Image::change_image_layout(
			device,
			self,
			&command_buffer,
			self.initial_layout,
			vk::ImageLayout::TRANSFER_DST_OPTIMAL,
		);

		unsafe {
			self.device.cmd_copy_image_to_buffer(
				command_buffer,
				self.image,
				vk::ImageLayout::TRANSFER_DST_OPTIMAL,
				*staging_buffer.buffer,
				&copy_region,
			);
		};

		Image::change_image_layout(
			device,
			self,
			&command_buffer,
			vk::ImageLayout::TRANSFER_DST_OPTIMAL,
			self.final_layout,
		);

		unsafe {
			self.device
				.end_command_buffer(command_buffer)
				.expect("Failed to stop a command buffer.");
		};

		let submit_info = [vk::SubmitInfo::builder()
			.command_buffers(&[command_buffer])
			.build()];
		unsafe {
			self.device
				.queue_submit(device.transfer_queue, &submit_info, vk::Fence::null())
				.expect("Failed to submit to transfer queue.");
			self.device
				.queue_wait_idle(device.transfer_queue)
				.expect("Failed to wait queue idle");

			self.device
				.free_command_buffers(command_builder.command_pool.command_pool, &[command_buffer]);
		};
		staging_buffer.read(0, data);
	}
}
