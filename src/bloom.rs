extern crate ultraviolet as uv;

use crate::vulkan_engine::{self, image, push_constant};

use ash::vk;

pub const BLOOM_MIP_COUNT: usize = 7usize;
pub const MODE_PREFILTER: u32 = 0;
pub const MODE_DOWNSAMPLE: u32 = 1;
pub const MODE_UPSAMPLE_FIRST: u32 = 2;
pub const MODE_UPSAMPLE: u32 = 3;
pub const MODE_APPLY: u32 = 4;

#[derive(Default, Clone, Copy)]
pub struct BloomConstant {
	pub mode_lod_in_out_bloom: u32,
}

fn update_descriptor(
	engine: &vulkan_engine::VulkanEngine,
	current_image: usize,
	bloom_images: &mut Vec<image::Image>,
) {
	let mut output_image_descr_info =
		Vec::<vk::DescriptorImageInfo>::with_capacity(3 * BLOOM_MIP_COUNT + 1);
	for i in 0..3 {
		for j in 0..BLOOM_MIP_COUNT {
			output_image_descr_info.push(
				vk::DescriptorImageInfo::builder()
					.image_layout(vk::ImageLayout::GENERAL)
					.image_view(bloom_images[i].mip_image_views[j])
					.build(),
			);
		}
	}
	output_image_descr_info.push(
		vk::DescriptorImageInfo::builder()
			.image_layout(vk::ImageLayout::GENERAL)
			.image_view(engine.swapchain.swapchain_image_views[current_image])
			.build(),
	);

	let mut input_image_descr_info = Vec::<vk::DescriptorImageInfo>::with_capacity(4);
	for i in 0..3 {
		input_image_descr_info.push(
			vk::DescriptorImageInfo::builder()
				.image_layout(vk::ImageLayout::GENERAL)
				.image_view(bloom_images[i].image_view)
				.sampler(bloom_images[i].image_sampler.unwrap())
				.build(),
		);
	}
	input_image_descr_info.push(
		vk::DescriptorImageInfo::builder()
			.image_layout(vk::ImageLayout::GENERAL)
			.image_view(engine.swapchain.swapchain_image_views[current_image])
			.sampler(engine.swapchain.swapchain_image_sampler)
			.build(),
	);

	engine.descriptors[1].update_descriptor_set(0, 0, None, Some(output_image_descr_info));
	engine.descriptors[1].update_descriptor_set(0, 1, None, Some(input_image_descr_info.clone()));
	engine.descriptors[1].update_descriptor_set(0, 2, None, Some(input_image_descr_info));
}

fn get_mip_size(current_mip: usize, image: &vulkan_engine::image::Image) -> vk::Extent2D {
	let mut width = image.extent.width;
	let mut height = image.extent.height;
	for _ in 0..current_mip {
		width /= 2;
		height /= 2;
	}
	vk::Extent2D::builder().width(width).height(height).build()
}

unsafe fn dispach(
	engine: &vulkan_engine::VulkanEngine,
	command_buffer: &vk::CommandBuffer,
	image_size: vk::Extent2D,
	memory_barrier: vk::MemoryBarrier,
) {
	let mut group_x = image_size.width / 8;
	let mut group_y = image_size.height / 4;
	if image_size.width % 8 != 0 {
		group_x += 1;
	}
	if image_size.height % 4 != 0 {
		group_y += 1;
	}
	engine
		.device
		.device
		.cmd_dispatch(*command_buffer, group_x, group_y, 1);
	engine.device.device.cmd_pipeline_barrier(
		*command_buffer,
		vk::PipelineStageFlags::COMPUTE_SHADER,
		vk::PipelineStageFlags::COMPUTE_SHADER,
		vk::DependencyFlags::empty(),
		&[memory_barrier],
		&[],
		&[],
	);
}

pub fn bloom(
	engine: &vulkan_engine::VulkanEngine,
	command_buffer: &mut vk::CommandBuffer,
	current_image: usize,
	bloom_images: &mut Vec<image::Image>,
	bloom_data: &mut BloomConstant,
) {
	let mut push = push_constant::PushConstant::new(
		0,
		std::mem::size_of::<BloomConstant>() as u32,
		vk::ShaderStageFlags::COMPUTE,
		vec![bloom_data.clone()],
	);

	update_descriptor(engine, current_image, bloom_images);

	let subresource_range = vk::ImageSubresourceRange::builder()
		.aspect_mask(vk::ImageAspectFlags::COLOR)
		.layer_count(1)
		.level_count(1)
		.build();
	let image_memory_barrier = vk::ImageMemoryBarrier::builder()
		.src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
		.dst_access_mask(vk::AccessFlags::MEMORY_WRITE | vk::AccessFlags::MEMORY_READ)
		.subresource_range(subresource_range)
		.image(engine.swapchain.swapchain_images[current_image])
		.src_queue_family_index(engine.device.queue_family_index)
		.dst_queue_family_index(engine.device.queue_family_index)
		.old_layout(vk::ImageLayout::PRESENT_SRC_KHR)
		.new_layout(vk::ImageLayout::GENERAL)
		.build();

	let memory_barrier = vk::MemoryBarrier::builder()
		.dst_access_mask(vk::AccessFlags::MEMORY_WRITE)
		.src_access_mask(vk::AccessFlags::MEMORY_READ)
		.build();

	// sync graphic --> compute + change layout
	unsafe {
		engine.device.device.cmd_pipeline_barrier(
			*command_buffer,
			vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
			vk::PipelineStageFlags::COMPUTE_SHADER,
			vk::DependencyFlags::empty(),
			&[],
			&[],
			&[image_memory_barrier],
		);
	};

	//preFilter
	bloom_data.mode_lod_in_out_bloom = MODE_PREFILTER << 28 | 0 << 21 | 3 << 14 | 0 << 7 | 0;
	push.set_data(vec![bloom_data.clone()]);
	unsafe {
		engine.device.device.cmd_bind_pipeline(
			*command_buffer,
			vk::PipelineBindPoint::COMPUTE,
			engine.compute_pipelines[0].pipeline,
		);
		engine.device.device.cmd_push_constants(
			*command_buffer,
			engine.compute_pipelines[0].pipeline_layout,
			vk::ShaderStageFlags::COMPUTE,
			0,
			&push.data,
		);
		engine.device.device.cmd_bind_descriptor_sets(
			*command_buffer,
			vk::PipelineBindPoint::COMPUTE,
			engine.compute_pipelines[0].pipeline_layout,
			0,
			&engine.descriptors[1].descriptor_set,
			&[],
		);
		let image_extent = vk::Extent2D::builder()
			.width(bloom_images[0].extent.width)
			.height(bloom_images[0].extent.height)
			.build();
		dispach(engine, command_buffer, image_extent, memory_barrier);
	};

	//DownSample
	for i in 1..BLOOM_MIP_COUNT {
		let mip_size = get_mip_size(i, &bloom_images[0]);
		unsafe {
			// Ping
			bloom_data.mode_lod_in_out_bloom = MODE_DOWNSAMPLE << 28
				| ((i - 1) as u32) << 21
				| 0 << 14 | ((1 * BLOOM_MIP_COUNT + i) as u32) << 7
				| 0;
			push.set_data(vec![bloom_data.clone()]);
			engine.device.device.cmd_push_constants(
				*command_buffer,
				engine.compute_pipelines[0].pipeline_layout,
				vk::ShaderStageFlags::COMPUTE,
				0,
				&push.data,
			);
			dispach(engine, command_buffer, mip_size, memory_barrier);

			// Pong
			bloom_data.mode_lod_in_out_bloom = MODE_DOWNSAMPLE << 28
				| (i as u32) << 21
				| 1 << 14 | ((0 * BLOOM_MIP_COUNT + i) as u32) << 7
				| 0;
			push.set_data(vec![bloom_data.clone()]);
			engine.device.device.cmd_push_constants(
				*command_buffer,
				engine.compute_pipelines[0].pipeline_layout,
				vk::ShaderStageFlags::COMPUTE,
				0,
				&push.data,
			);
			dispach(engine, command_buffer, mip_size, memory_barrier);
		};
	}

	// First Upsample
	unsafe {
		bloom_data.mode_lod_in_out_bloom = MODE_UPSAMPLE_FIRST << 28
			| ((BLOOM_MIP_COUNT - 2) as u32) << 21
			| 0 << 14 | ((3 * BLOOM_MIP_COUNT - 1) as u32) << 7
			| 0;
		push.set_data(vec![bloom_data.clone()]);
		engine.device.device.cmd_push_constants(
			*command_buffer,
			engine.compute_pipelines[0].pipeline_layout,
			vk::ShaderStageFlags::COMPUTE,
			0,
			&push.data,
		);

		let mip_size = get_mip_size(BLOOM_MIP_COUNT - 1, &bloom_images[2]);
		dispach(engine, command_buffer, mip_size, memory_barrier);
	}

	//Upsample
	for i in (0..=BLOOM_MIP_COUNT - 2).rev() {
		unsafe {
			let mip_size = get_mip_size(i, &bloom_images[2]);
			bloom_data.mode_lod_in_out_bloom = MODE_UPSAMPLE << 28
				| (i as u32) << 21
				| 0 << 14 | ((2 * BLOOM_MIP_COUNT + i) as u32) << 7
				| 2;
			push.set_data(vec![bloom_data.clone()]);
			engine.device.device.cmd_push_constants(
				*command_buffer,
				engine.compute_pipelines[0].pipeline_layout,
				vk::ShaderStageFlags::COMPUTE,
				0,
				&push.data,
			);
			dispach(engine, command_buffer, mip_size, memory_barrier);
		};
	}

	// Apply the bloom to the render texture
	unsafe {
		let mip_size = engine.surface.surface_resolution;
		bloom_data.mode_lod_in_out_bloom =
			MODE_APPLY << 28 | 0 << 21 | 3 << 14 | (3 * BLOOM_MIP_COUNT as u32) << 7 | 2;
		push.set_data(vec![bloom_data.clone()]);
		engine.device.device.cmd_push_constants(
			*command_buffer,
			engine.compute_pipelines[0].pipeline_layout,
			vk::ShaderStageFlags::COMPUTE,
			0,
			&push.data,
		);
		dispach(engine, command_buffer, mip_size, memory_barrier);
	};

	// Change layout back to present
	let image_memory_barrier = vk::ImageMemoryBarrier::builder()
		.src_access_mask(vk::AccessFlags::MEMORY_WRITE | vk::AccessFlags::MEMORY_READ)
		.dst_access_mask(vk::AccessFlags::MEMORY_READ)
		.subresource_range(subresource_range)
		.image(engine.swapchain.swapchain_images[current_image])
		.src_queue_family_index(engine.device.queue_family_index)
		.dst_queue_family_index(engine.device.queue_family_index)
		.old_layout(vk::ImageLayout::GENERAL)
		.new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
		.build();

	unsafe {
		engine.device.device.cmd_pipeline_barrier(
			*command_buffer,
			vk::PipelineStageFlags::COMPUTE_SHADER,
			vk::PipelineStageFlags::BOTTOM_OF_PIPE,
			vk::DependencyFlags::empty(),
			&[],
			&[],
			&[image_memory_barrier],
		);
	};
}
