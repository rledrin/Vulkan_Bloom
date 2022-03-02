extern crate ultraviolet as uv;

use std::mem::size_of;

use ash::vk;

mod bloom;
mod render;
mod vulkan_engine;

use gpu_alloc::UsageFlags;
use vulkan_engine::{buffer, descriptor, image, pipeline, push_constant, shader_module, window};
use winit::platform::run_return::EventLoopExtRunReturn;

fn main() {
	vulkan_engine::compile_shaders();
	let mut engine = vulkan_engine::VulkanEngine::new();

	let proj = uv::projection::perspective_vk(
		std::f32::consts::FRAC_PI_3,
		engine.surface.surface_resolution.width as f32
			/ engine.surface.surface_resolution.height as f32,
		0.1f32,
		1000f32,
	);

	let view = uv::Mat4::look_at(
		uv::Vec3::new(0.0, 0.0, -8.0),
		uv::Vec3::new(0.0, 0.0, 0.0),
		uv::Vec3::new(0.0, 1.0, 0.0),
	);

	let pv = proj * view;
	let model = uv::Mat4::from_scale(1.0f32);

	let mut pbr_param = vulkan_engine::PbrParameters {
		albedo: uv::Vec3::new(1.0, 0.0, 0.0),
		metallic: 0.0,
		roughness: 0.2,
		ao: 0.01,
		cam_pos: uv::Vec3::new(0.0, 0.0, -8.0),
		lights: [vulkan_engine::Light {
			light_position: uv::Vec3::new(-4.0, 5.0, -5.0),
			light_color: uv::Vec3::new(5.0, 5.0, 5.0),
			..Default::default()
		}],
		..Default::default()
	};

	let u_buffer_size = buffer::aligne_offset(size_of::<uv::Mat4>() as u64) * 2
		+ buffer::aligne_offset(size_of::<uv::Vec4>() as u64)
		+ buffer::aligne_offset(size_of::<vulkan_engine::PbrParameters>() as u64);

	let mut uniform_buffer = buffer::Buffer::new(
		&engine.device,
		vk::BufferCreateFlags::empty(),
		u_buffer_size,
		vk::BufferUsageFlags::UNIFORM_BUFFER
			| vk::BufferUsageFlags::TRANSFER_DST
			| vk::BufferUsageFlags::TRANSFER_SRC,
		vk::SharingMode::EXCLUSIVE,
		// UsageFlags::DOWNLOAD | UsageFlags::UPLOAD | UsageFlags::FAST_DEVICE_ACCESS,
		UsageFlags::DOWNLOAD | UsageFlags::UPLOAD | UsageFlags::HOST_ACCESS,
	);

	uniform_buffer.write(0, vec![pv]);
	uniform_buffer.write(size_of::<uv::Mat4>() as u64, vec![model]);
	uniform_buffer.write(
		buffer::aligne_offset((size_of::<uv::Mat4>() * 2 + size_of::<uv::Vec4>()) as u64),
		vec![pbr_param],
	);

	let mut uniform_descriptor = descriptor::DescriptorSet::new(
		&engine.device,
		[(vk::DescriptorType::UNIFORM_BUFFER, 2)].to_vec(),
		2,
		vec![
			vk::DescriptorSetLayoutBinding::builder()
				.binding(0)
				.descriptor_count(1)
				.descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
				.stage_flags(vk::ShaderStageFlags::VERTEX)
				.build(),
			vk::DescriptorSetLayoutBinding::builder()
				.binding(1)
				.descriptor_count(1)
				.descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
				.stage_flags(vk::ShaderStageFlags::VERTEX)
				.build(),
		],
	);
	uniform_descriptor.update_descriptor_set(
		0,
		0,
		Some(vec![vk::DescriptorBufferInfo::builder()
			.buffer(*uniform_buffer.buffer)
			.offset(0)
			.range(size_of::<uv::Mat4>() as u64)
			.build()]),
		None,
	);
	uniform_descriptor.update_descriptor_set(
		0,
		1,
		Some(vec![vk::DescriptorBufferInfo::builder()
			.buffer(*uniform_buffer.buffer)
			.offset(size_of::<uv::Mat4>() as u64)
			.range(size_of::<uv::Mat4>() as u64)
			.build()]),
		None,
	);

	uniform_descriptor.create_another_set(
		&engine.device,
		vec![vk::DescriptorSetLayoutBinding::builder()
			.binding(0)
			.descriptor_count(1)
			.descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
			.stage_flags(vk::ShaderStageFlags::FRAGMENT)
			.build()],
	);
	uniform_descriptor.update_descriptor_set(
		1,
		0,
		Some(vec![vk::DescriptorBufferInfo::builder()
			.buffer(*uniform_buffer.buffer)
			.offset(buffer::aligne_offset(
				(size_of::<uv::Mat4>() * 2 + size_of::<uv::Vec4>()) as u64,
			))
			.range(size_of::<vulkan_engine::PbrParameters>() as u64)
			.build()]),
		None,
	);

	engine.descriptors.push(uniform_descriptor);

	engine.build_basic_pipeline(
		0,
		"shaders/spv/vert.spv",
		"shaders/spv/frag.spv",
		vec![(0, 0), (0, 1)],
		None,
	);

	let arg = std::path::Path::new(&std::env::args().into_iter().next().unwrap())
		.parent()
		.unwrap()
		.parent()
		.unwrap()
		.parent()
		.unwrap()
		.to_str()
		.unwrap()
		.to_owned();

	let obj_path = if arg.is_empty() {
		// "obj/IcoSphere.obj".to_owned()
		// "obj/IcoSphere_hd_smooth.obj".to_owned()
		"obj/uv_sphere.obj".to_owned()
	} else {
		arg + "/obj/uv_sphere.obj"
	};

	// let vertex_data = vulkan_engine::generate_cube();
	let input = std::io::BufReader::new(std::fs::File::open(obj_path).unwrap());
	let vertices: obj::Obj<obj::Vertex, u32> = obj::load_obj(input).unwrap();
	let mut vertex_data = Vec::<vulkan_engine::Vertex>::with_capacity(vertices.indices.len());

	for vert in vertices.vertices.into_iter() {
		let position = vert.position;
		let normal = vert.normal;
		vertex_data.push(vulkan_engine::Vertex {
			position: uv::Vec3::new(position[0], position[1], position[2]),
			normal: uv::Vec3::new(normal[0], normal[1], normal[2]),
			..Default::default()
		});
	}
	let indices = vertices.indices;

	let index_count = indices.len() as u32;

	let mut index_buffer = buffer::Buffer::new(
		&engine.device,
		vk::BufferCreateFlags::empty(),
		(std::mem::size_of_val(&indices[0]) * indices.len()) as u64,
		vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
		vk::SharingMode::EXCLUSIVE,
		UsageFlags::FAST_DEVICE_ACCESS,
	);

	index_buffer.write_to_vram(&engine.device, &engine.command_builder, 0, indices);

	let mut vertex_buffer = buffer::Buffer::new(
		&engine.device,
		vk::BufferCreateFlags::empty(),
		// (std::mem::size_of::<vulkan_engine::Vertex>() * vertex_data.len()) as u64,
		// (std::mem::size_of::<f32>() * vertex_data.len()) as u64,
		(std::mem::size_of_val(&vertex_data[0]) * vertex_data.len()) as u64,
		vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
		vk::SharingMode::EXCLUSIVE,
		UsageFlags::FAST_DEVICE_ACCESS,
	);

	vertex_buffer.write_to_vram(&engine.device, &engine.command_builder, 0, vertex_data);

	let mut downsample_image = Vec::<image::Image>::with_capacity(3);

	let image_width = engine.surface.surface_resolution.width / 2;
	let image_height = engine.surface.surface_resolution.height / 2;

	for _ in 0..3 {
		let mut image = image::Image::new(
			&engine.device,
			vk::ImageCreateFlags::empty(),
			vk::ImageType::TYPE_2D,
			engine.surface.desired_format,
			vk::Extent3D::builder()
				.width(image_width)
				.height(image_height)
				.depth(1)
				.build(),
			bloom::BLOOM_MIP_COUNT as u32,
			1,
			vk::ImageTiling::OPTIMAL,
			vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
			engine.device.queue_family_index,
			vk::ImageLayout::UNDEFINED,
			vk::ImageLayout::GENERAL,
			vk::ImageViewType::TYPE_2D,
			vk::ImageAspectFlags::COLOR,
			UsageFlags::FAST_DEVICE_ACCESS,
		);
		image.set_sampler(
			vk::Filter::LINEAR,
			vk::Filter::LINEAR,
			vk::SamplerMipmapMode::LINEAR,
			vk::SamplerAddressMode::CLAMP_TO_EDGE,
			vk::SamplerAddressMode::CLAMP_TO_EDGE,
			vk::SamplerAddressMode::CLAMP_TO_EDGE,
			0.0,
			false,
			1.0,
			false,
			vk::CompareOp::ALWAYS,
			-1000.0,
			1000.0,
			vk::BorderColor::FLOAT_OPAQUE_BLACK,
		);
		image.change_layout(
			&engine.device,
			&engine.command_builder,
			image.initial_layout,
			image.final_layout,
		);
		downsample_image.push(image);
	}

	let image_descriptor = descriptor::DescriptorSet::new(
		&engine.device,
		vec![(vk::DescriptorType::UNIFORM_BUFFER, 1)],
		1,
		vec![
			vk::DescriptorSetLayoutBinding::builder()
				.binding(0)
				.descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
				.descriptor_count((3 * bloom::BLOOM_MIP_COUNT + 1) as u32)
				.stage_flags(vk::ShaderStageFlags::COMPUTE)
				.build(),
			vk::DescriptorSetLayoutBinding::builder()
				.binding(1)
				.descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
				.descriptor_count(4)
				.stage_flags(vk::ShaderStageFlags::COMPUTE)
				.build(),
			vk::DescriptorSetLayoutBinding::builder()
				.binding(2)
				.descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
				.descriptor_count(4)
				.stage_flags(vk::ShaderStageFlags::COMPUTE)
				.build(),
			vk::DescriptorSetLayoutBinding::builder()
				.binding(3)
				.descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
				.descriptor_count(1)
				.stage_flags(vk::ShaderStageFlags::COMPUTE)
				.build(),
		],
	);

	let bloom_threshold = 1.0f32;
	let bloom_knee = 0.2f32;

	uniform_buffer.write(
		(size_of::<uv::Mat4>() * 2) as u64,
		vec![uv::Vec4::new(
			bloom_threshold,
			bloom_threshold - bloom_knee,
			bloom_knee * 2.0f32,
			0.25f32 / bloom_knee,
		)],
	);

	image_descriptor.update_descriptor_set(
		0,
		3,
		Some(vec![vk::DescriptorBufferInfo::builder()
			.buffer(*uniform_buffer.buffer)
			.offset((size_of::<uv::Mat4>() * 2) as u64)
			.range(size_of::<uv::Vec4>() as u64)
			.build()]),
		None,
	);

	let mut bloom_data = bloom::BloomConstant {
		mode_lod_in_out_bloom: 0,
	};

	let compute_constant = push_constant::PushConstant::new(
		0,
		std::mem::size_of::<bloom::BloomConstant>() as u32,
		vk::ShaderStageFlags::COMPUTE,
		vec![bloom_data.clone()],
	);
	let compute_module =
		shader_module::ShaderModule::new(&engine.device, "shaders/spv/bloom.spv", "main");

	let compute_pipeline = pipeline::ComputePipeline::builder()
		.add_push_constant(&compute_constant)
		.add_descriptor_set(&image_descriptor, 0)
		.compute_module(&compute_module, vk::PipelineShaderStageCreateFlags::empty())
		.build(&engine.device);

	engine.descriptors.push(image_descriptor);
	engine.push_constants.push(compute_constant);
	engine.compute_pipelines.push(compute_pipeline);

	let mut imgui = imgui::Context::create();

	let mut renderer = imgui_rs_vulkan_renderer::Renderer::with_default_allocator(
		&engine.instance.instance,
		engine.device.physical_device,
		(*engine.device.device).clone(),
		engine.device.graphic_queue,
		engine.command_builder.command_pool.command_pool,
		engine.ui_renderpass.renderpass,
		&mut imgui,
		Some(imgui_rs_vulkan_renderer::Options {
			in_flight_frames: 1,
			..Default::default()
		}),
	)
	.unwrap();

	let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui); // step 1

	platform.attach_window(
		imgui.io_mut(),
		&engine.window.as_ref().unwrap().window,
		imgui_winit_support::HiDpiMode::Default,
	);

	let mut albedo_color = [0.0f32; 3];
	let mut emissive_color = [0.0f32; 3];

	let mut current_image = 0;
	let mut window: window::Window = unsafe { std::mem::transmute_copy(&engine.window) };
	let mut time = std::time::Instant::now();
	let mut delta_time = std::time::Duration::ZERO;

	window.event_loop.run_return(|event, _, control_flow| {
		*control_flow = winit::event_loop::ControlFlow::Poll;
		match event {
			winit::event::Event::WindowEvent { event, window_id } => match event {
				winit::event::WindowEvent::CloseRequested => {
					*control_flow = winit::event_loop::ControlFlow::Exit
				}
				winit::event::WindowEvent::KeyboardInput {
					input:
						winit::event::KeyboardInput {
							virtual_keycode: Some(winit::event::VirtualKeyCode::Escape),
							state: winit::event::ElementState::Pressed,
							..
						},
					..
				} => {
					*control_flow = winit::event_loop::ControlFlow::Exit;
				}
				winit::event::WindowEvent::Resized(logical_size) => {
					// println!("new size: {:?}", logical_size);
					if logical_size.width != 0 && logical_size.height != 0 {
						engine.new_extent = vk::Extent2D::builder()
							.width(logical_size.width)
							.height(logical_size.height)
							.build();
						engine.resized = true;
						engine.minimized = false;
					} else if logical_size.width == 0 && logical_size.height == 0 {
						engine.minimized = true;
					} else if engine.minimized == true
						&& logical_size.width != 0
						&& logical_size.height != 0
					{
						engine.minimized = false;
					}
					platform.handle_event::<winit::event::Event<()>>(
						imgui.io_mut(),
						&engine.window.as_ref().unwrap().window,
						&winit::event::Event::WindowEvent { event, window_id },
					);
				}
				_ => {
					platform.handle_event::<winit::event::Event<()>>(
						imgui.io_mut(),
						&engine.window.as_ref().unwrap().window,
						&winit::event::Event::WindowEvent { event, window_id },
					);
				}
			},
			winit::event::Event::NewEvents(_) => {
				let now = std::time::Instant::now();
				delta_time = now.duration_since(time);
				time = now;
				imgui.io_mut().update_delta_time(delta_time);

				if engine.resized == true && engine.old_extent == engine.new_extent {
					engine.window_resized(&mut current_image);
					engine.resized = false;
				} else if engine.resized == true && engine.old_extent != engine.new_extent {
					engine.old_extent = engine.new_extent;
				}
			}
			winit::event::Event::MainEventsCleared => {
				platform
					.prepare_frame(imgui.io_mut(), &engine.window.as_ref().unwrap().window)
					.expect("Failed to prepare frame");
				engine.window.as_ref().unwrap().window.request_redraw();
			}
			winit::event::Event::RedrawRequested(_) => {
				if engine.resized == false && engine.minimized == false {
					let ui = imgui.frame();
					// let mut opened = true;

					// ui.show_demo_window(&mut opened);
					albedo_color[0] = pbr_param.albedo.x;
					albedo_color[1] = pbr_param.albedo.y;
					albedo_color[2] = pbr_param.albedo.z;

					imgui::Window::new("Pbr parameters")
						.size([300.0, 100.0], imgui::Condition::FirstUseEver)
						.build(&ui, || {
							imgui::Slider::new("roughness", 0.0f32, 1.0f32)
								.build(&ui, &mut pbr_param.roughness);
							imgui::Slider::new("metallic", 0.0f32, 1.0f32)
								.build(&ui, &mut pbr_param.metallic);
							imgui::Slider::new("ao", 0.0f32, 1.0f32).build(&ui, &mut pbr_param.ao);
							imgui::ColorEdit::new(
								"albedo",
								imgui::EditableColor::Float3(&mut albedo_color),
							)
							.build(&ui);
							imgui::ColorEdit::new(
								"emissive",
								imgui::EditableColor::Float3(&mut emissive_color),
							)
							.build(&ui);
							// imgui::Textures::new().
							// imgui::Slider::new("emissive intensity", 0.0f32, 100.0f32).build(&ui, &mut val);
						})
						.expect("Failed to create the ui");

					pbr_param.albedo.x = albedo_color[0];
					pbr_param.albedo.y = albedo_color[1];
					pbr_param.albedo.z = albedo_color[2];

					uniform_buffer.write(
						(size_of::<uv::Mat4>() * 2 + size_of::<uv::Vec4>()) as u64,
						vec![pbr_param],
					);

					platform.prepare_render(&ui, &engine.window.as_ref().unwrap().window);
					let draw_data = ui.render();
					let mut tmp_current_image = current_image as usize;
					render::render_func(
						&engine,
						&vertex_buffer,
						&index_buffer,
						&mut tmp_current_image,
						index_count,
						&mut renderer,
						draw_data,
						&mut downsample_image,
						&mut bloom_data,
					);
					current_image = tmp_current_image as u32;
				} else if engine.minimized == true {
					std::thread::sleep(std::time::Duration::from_millis(10));
				}
			}
			// winit::event::Event::WindowEvent
			event => {
				platform.handle_event(
					imgui.io_mut(),
					&engine.window.as_ref().unwrap().window,
					&event,
				);
			}
		}
	});
}
