extern crate ultraviolet as uv;

use ash::vk;

use crate::{
	bloom,
	vulkan_engine::{self, buffer, image},
};

pub fn render_func(
	engine: &vulkan_engine::VulkanEngine,
	vertex_buffer: &buffer::Buffer,
	index_buffer: &buffer::Buffer,
	current_image_save: &mut usize,
	index_count: u32,
	renderer: &mut imgui_rs_vulkan_renderer::Renderer,
	draw_data: &imgui::DrawData,
	bloom_images: &mut Vec<image::Image>,
	bloom_data: &mut bloom::BloomConstant,
) {
	let current_image = *current_image_save;
	unsafe {
		let tmp = engine
			.swapchain
			.swapchain_loader
			.acquire_next_image(
				engine.swapchain.swapchain,
				std::u64::MAX,
				engine.image_available_semaphore.semaphores[0],
				engine.fences.fences[current_image],
			)
			.expect("Failed to acquire the next swapchain image");
		*current_image_save = tmp.0 as usize;
	};

	let clear_value = [
		vk::ClearValue {
			color: vk::ClearColorValue {
				float32: [0.0, 0.0, 0.0, 0.0],
			},
		},
		vk::ClearValue {
			depth_stencil: vk::ClearDepthStencilValue {
				depth: 1.0,
				stencil: 1,
			},
		},
	];
	let render_area = vk::Rect2D::builder()
		.extent(engine.surface.surface_resolution)
		.build();

	let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
		.render_pass(engine.renderpass.renderpass)
		.framebuffer(engine.swapchain.swapchain_framebuffers[current_image])
		.render_area(render_area)
		.clear_values(&clear_value)
		.build();

	let ui_render_pass_begin_info = vk::RenderPassBeginInfo::builder()
		.render_pass(engine.ui_renderpass.renderpass)
		.framebuffer(engine.swapchain.swapchain_ui_framebuffers[current_image])
		.render_area(render_area)
		.clear_values(&clear_value)
		.build();

	let mut command_buffer = engine.command_builder.build();
	unsafe {
		engine.device.device.cmd_begin_render_pass(
			command_buffer,
			&render_pass_begin_info,
			vk::SubpassContents::INLINE,
		);
		engine.device.device.cmd_bind_pipeline(
			command_buffer,
			vk::PipelineBindPoint::GRAPHICS,
			engine.graphics_pipelines[0].pipeline,
		);
		engine.device.device.cmd_bind_descriptor_sets(
			command_buffer,
			vk::PipelineBindPoint::GRAPHICS,
			engine.graphics_pipelines[0].pipeline_layout,
			0,
			&engine.descriptors[0].descriptor_set,
			&[],
		);
		engine.device.device.cmd_bind_vertex_buffers(
			command_buffer,
			0,
			&[*vertex_buffer.buffer],
			&[0],
		);

		engine.device.device.cmd_bind_index_buffer(
			command_buffer,
			*index_buffer.buffer,
			0,
			vk::IndexType::UINT32,
		);

		engine
			.device
			.device
			.cmd_draw_indexed(command_buffer, index_count, 1, 0, 0, 0);

		// renderer.cmd_draw(command_buffer, draw_data).expect("Failed to draw the ui.");

		engine.device.device.cmd_end_render_pass(command_buffer);

		//BLOOM BEGIN

		bloom::bloom(
			engine,
			&mut command_buffer,
			*current_image_save,
			bloom_images,
			bloom_data,
		);

		//BLOOM END

		engine.device.device.cmd_begin_render_pass(
			command_buffer,
			&ui_render_pass_begin_info,
			vk::SubpassContents::INLINE,
		);

		renderer
			.cmd_draw(command_buffer, draw_data)
			.expect("Failed to draw the ui.");

		engine.device.device.cmd_end_render_pass(command_buffer);

		engine
			.device
			.device
			.end_command_buffer(command_buffer)
			.expect("Failed to end a command Buffer.");
		engine
			.device
			.device
			.wait_for_fences(&[engine.fences.fences[current_image]], true, std::u64::MAX)
			.expect("Failed to wait for fences.");

		let submit_info = vk::SubmitInfo::builder()
			.command_buffers(&[command_buffer])
			.wait_semaphores(&engine.image_available_semaphore.semaphores)
			.signal_semaphores(&engine.render_finished_semaphore.semaphores)
			.wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
			.build();

		engine
			.device
			.device
			.reset_fences(&engine.fences.fences)
			.expect("Failed to reset fences.");
		engine
			.device
			.device
			.queue_submit(
				engine.device.graphic_queue,
				&[submit_info],
				engine.fences.fences[current_image],
			)
			.expect("Failed to submit a command buffer to the graphics queue.");
		let present_info = vk::PresentInfoKHR::builder()
			.swapchains(&[engine.swapchain.swapchain])
			.wait_semaphores(&engine.render_finished_semaphore.semaphores)
			.image_indices(&[current_image as u32])
			.build();
		engine
			.swapchain
			.swapchain_loader
			.queue_present(engine.device.present_queue, &present_info)
			.expect("Failed to present an image to the present queue.");

		engine
			.device
			.device
			.wait_for_fences(&[engine.fences.fences[current_image]], true, std::u64::MAX)
			.expect("Failed to wait for fences.");
		engine
			.device
			.device
			.reset_fences(&engine.fences.fences)
			.expect("Failed to reset fences.");
		engine.device.device.device_wait_idle().unwrap();
		engine.device.device.free_command_buffers(
			engine.command_builder.command_pool.command_pool,
			&[command_buffer],
		)
	};
	*current_image_save = (*current_image_save + 1) % engine.swapchain.max_image_in_flight;
}
