pub mod buffer;
pub mod command_buffer;
pub mod descriptor;
pub mod device;
pub mod fence;
pub mod image;
pub mod instance;
pub mod pipeline;
pub mod push_constant;
pub mod renderpass;
pub mod semaphore;
pub mod shader_module;
pub mod surface;
pub mod swapchain;
pub mod window;

use std::ops::Add;

use ash::vk;

#[derive(Default, Clone, Copy)]
#[repr(C)]
pub struct Vertex {
	pub position: uv::Vec3,
	pub normal: uv::Vec3,
	pub uv: uv::Vec2,
}

#[derive(Default, Clone, Copy)]
#[repr(C)]
pub struct Light {
	pub light_position: uv::Vec3,
	pub padding_1: u32,
	pub light_color: uv::Vec3,
	pub padding_2: u32,
}

#[derive(Default, Clone, Copy)]
#[repr(C)]
pub struct PbrParameters {
	pub albedo: uv::Vec3,
	pub metallic: f32,
	pub roughness: f32,
	pub ao: f32,
	pub padding_2: [u32; 2],
	pub cam_pos: uv::Vec3,
	pub padding_3: u32,
	pub lights: [Light; 1],
}

pub struct VulkanEngine {
	pub fences: fence::Fence,
	pub ui_fence: fence::Fence,
	pub render_finished_semaphore: semaphore::Semaphore,
	pub image_available_semaphore: semaphore::Semaphore,
	pub graphics_pipelines: Vec<pipeline::GraphicsPipeline>,
	pub compute_pipelines: Vec<pipeline::ComputePipeline>,
	pub push_constants: Vec<push_constant::PushConstant>,
	pub descriptors: Vec<descriptor::DescriptorSet>,
	pub command_builder: command_buffer::CommandBufferBuilder,
	pub swapchain: swapchain::Swapchain,
	pub renderpass: renderpass::RenderPass,
	pub ui_renderpass: renderpass::RenderPass,
	pub surface: surface::Surface,
	pub device: device::Device,
	pub instance: instance::Instance,
	pub window: Option<window::Window>,
	pub old_extent: vk::Extent2D,
	pub new_extent: vk::Extent2D,
	pub resized: bool,
	pub minimized: bool,
}

impl VulkanEngine {
	pub fn create_depth_image(
		instance: &instance::Instance,
		device: &device::Device,
		surface: &surface::Surface,
	) -> image::Image {
		let formats = vec![
			vk::Format::D32_SFLOAT,
			vk::Format::D32_SFLOAT_S8_UINT,
			vk::Format::D24_UNORM_S8_UINT,
			vk::Format::D16_UNORM_S8_UINT,
		];

		let mut selected_format = vk::Format::UNDEFINED;
		for f in formats.into_iter() {
			let a = unsafe {
				instance
					.instance
					.get_physical_device_format_properties(device.physical_device, f)
			};
			if a.optimal_tiling_features & vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT
				== vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT
			{
				selected_format = f;
				break;
			} else {
				selected_format = vk::Format::UNDEFINED;
			}
		}

		image::Image::new(
			device,
			vk::ImageCreateFlags::empty(),
			vk::ImageType::TYPE_2D,
			selected_format,
			vk::Extent3D::builder()
				.width(surface.surface_resolution.width)
				.height(surface.surface_resolution.height)
				.depth(1)
				.build(),
			1,
			1,
			vk::ImageTiling::OPTIMAL,
			vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
			device.queue_family_index,
			vk::ImageLayout::UNDEFINED,
			vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
			vk::ImageViewType::TYPE_2D,
			vk::ImageAspectFlags::DEPTH,
			gpu_alloc::UsageFlags::FAST_DEVICE_ACCESS,
		)
	}

	pub fn build_basic_pipeline(
		&mut self,
		subpass_index: u32,
		vert_shader: &str,
		frag_shader: &str,
		descriptor_set_indices: Vec<(usize, usize)>,
		push_constant_index: Option<usize>,
	) {
		let vertex_module = shader_module::ShaderModule::new(&self.device, vert_shader, "main");
		let fragment_module = shader_module::ShaderModule::new(&self.device, frag_shader, "main");

		let mut builder = pipeline::GraphicsPipeline::builder()
			.vertex_module_1(vertex_module)
			.fragment_module_2(fragment_module)
			.add_vertex_binding_3(
				0,
				std::mem::size_of::<Vertex>() as u32,
				vk::VertexInputRate::VERTEX,
			)
			.add_vertex_attribute_4(
				0,
				0,
				vk::Format::R32G32B32_SFLOAT,
				memoffset::offset_of!(Vertex, position) as u32,
			)
			.add_vertex_attribute_4(
				1,
				0,
				vk::Format::R32G32B32_SFLOAT,
				memoffset::offset_of!(Vertex, normal) as u32,
			)
			.add_vertex_attribute_4(
				2,
				0,
				vk::Format::R32G32_SFLOAT,
				memoffset::offset_of!(Vertex, uv) as u32,
			)
			.assembly_state_5(vk::PrimitiveTopology::TRIANGLE_LIST, false)
			.add_viewport_7(
				vk::Viewport::builder()
					.height(self.window.as_ref().unwrap().window_extent.height as f32)
					.width(self.window.as_ref().unwrap().window_extent.width as f32)
					.min_depth(0.0f32)
					.max_depth(1.0f32)
					.build(),
			)
			.add_scissor_8(
				vk::Rect2D::builder()
					.extent(self.window.as_ref().unwrap().window_extent)
					.build(),
			)
			.rasterization_state_9(
				false,
				false,
				vk::PolygonMode::FILL,
				vk::CullModeFlags::BACK,
				vk::FrontFace::COUNTER_CLOCKWISE,
				false,
				0.0,
				0.0,
				0.0,
				1.0,
			)
			.multisample_state_10(
				vk::SampleCountFlags::TYPE_1,
				false,
				0.0,
				&[vk::SampleMask::MAX],
				false,
				false,
			)
			.depth_stencil_state_11(
				true,
				true,
				vk::CompareOp::LESS,
				false,
				false,
				vk::StencilOpState::builder().build(),
				vk::StencilOpState::builder().build(),
				0.0,
				1.0,
			)
			.add_color_blend_attachments_12(
				false,
				vk::ColorComponentFlags::RGBA,
				vk::BlendFactor::ONE,
				vk::BlendFactor::ZERO,
				vk::BlendOp::ADD,
				vk::BlendFactor::ONE,
				vk::BlendFactor::ZERO,
				vk::BlendOp::ADD,
			)
			.color_blend_state_13(false, vk::LogicOp::COPY, [1.0f32; 4]);
		if !descriptor_set_indices.is_empty() {
			for (descriptor_vector_index, set_index) in descriptor_set_indices.into_iter() {
				builder = builder
					.add_descriptor_set_15(&self.descriptors[descriptor_vector_index], set_index);
			}
		}
		if let Some(index) = push_constant_index {
			builder = builder.add_push_constant_16(&self.push_constants[index]);
		}
		builder = builder.renderpass_17(&self.renderpass, subpass_index);
		let pipeline = builder.build(&self.device);

		self.graphics_pipelines.push(pipeline);
	}

	pub fn new() -> VulkanEngine {
		let window = window::Window::new(1080, 720, "Bloom");
		// let window = window::Window::new(1920, 1080, "Bloom");
		let instance = instance::Instance::new(&window);
		let device = device::Device::new(&instance);
		let surface = surface::Surface::new(&instance, &window, &device);
		let mut depth_stencil_image =
			VulkanEngine::create_depth_image(&instance, &device, &surface);
		let renderpass = renderpass::RenderPass::builder()
			.add_attachment(
				surface.desired_format,
				vk::SampleCountFlags::TYPE_1,
				vk::AttachmentLoadOp::CLEAR,
				vk::AttachmentStoreOp::STORE,
				vk::AttachmentLoadOp::DONT_CARE,
				vk::AttachmentStoreOp::DONT_CARE,
				vk::ImageLayout::UNDEFINED,
				vk::ImageLayout::PRESENT_SRC_KHR,
			)
			.add_attachment(
				depth_stencil_image.format,
				vk::SampleCountFlags::TYPE_1,
				vk::AttachmentLoadOp::CLEAR,
				vk::AttachmentStoreOp::STORE,
				vk::AttachmentLoadOp::DONT_CARE,
				vk::AttachmentStoreOp::DONT_CARE,
				vk::ImageLayout::UNDEFINED,
				vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
			)
			.add_subpasses(
				vk::PipelineBindPoint::GRAPHICS,
				vec![],
				1,
				Some(
					vk::AttachmentReference::builder()
						.attachment(1)
						.layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
						.build(),
				),
				vec![],
				vec![],
			)
			.add_dependencies(
				vk::SUBPASS_EXTERNAL,
				0,
				vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
					| vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
				vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
					| vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
				vk::AccessFlags::empty(),
				vk::AccessFlags::COLOR_ATTACHMENT_WRITE
					| vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
				vk::DependencyFlags::empty(),
			)
			.build(&device);

		let ui_renderpass = renderpass::RenderPass::builder()
			.add_attachment(
				surface.desired_format,
				vk::SampleCountFlags::TYPE_1,
				vk::AttachmentLoadOp::LOAD,
				vk::AttachmentStoreOp::STORE,
				vk::AttachmentLoadOp::DONT_CARE,
				vk::AttachmentStoreOp::DONT_CARE,
				vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
				vk::ImageLayout::PRESENT_SRC_KHR,
			)
			.add_subpasses(
				vk::PipelineBindPoint::GRAPHICS,
				vec![],
				1,
				None,
				vec![],
				vec![],
			)
			.add_dependencies(
				vk::SUBPASS_EXTERNAL,
				0,
				vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
					| vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
				vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
					| vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
				vk::AccessFlags::empty(),
				vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
				vk::DependencyFlags::empty(),
			)
			.build(&device);

		let command_builder = command_buffer::CommandBufferBuilder::primary(
			&device,
			command_buffer::CommandBufferUsage::OneTimeSubmit,
		);

		depth_stencil_image.change_layout(
			&device,
			&command_builder,
			depth_stencil_image.initial_layout,
			depth_stencil_image.final_layout,
		);

		let swapchain = swapchain::Swapchain::new(
			&instance,
			&surface,
			&device,
			Some(ash::vk::PresentModeKHR::FIFO),
			Some(
				vk::ImageUsageFlags::COLOR_ATTACHMENT
					| vk::ImageUsageFlags::STORAGE
					| vk::ImageUsageFlags::SAMPLED,
			),
			Some(depth_stencil_image),
			&renderpass,
			&ui_renderpass,
		);

		let descriptors = Vec::with_capacity(1);
		let push_constants = Vec::with_capacity(1);
		let graphics_pipelines = Vec::<pipeline::GraphicsPipeline>::with_capacity(1);

		let compute_pipelines = Vec::<pipeline::ComputePipeline>::with_capacity(1);

		let ui_fence = fence::Fence::new(&device, true, 1);
		let fences = fence::Fence::new(&device, false, swapchain.swapchain_framebuffers.len());

		let render_finished_semaphore = semaphore::Semaphore::new(&device, 1);
		let image_available_semaphore = semaphore::Semaphore::new(&device, 1);

		VulkanEngine {
			old_extent: surface.surface_resolution,
			new_extent: surface.surface_resolution,
			resized: false,
			minimized: false,
			render_finished_semaphore,
			image_available_semaphore,
			fences,
			ui_fence,
			graphics_pipelines,
			compute_pipelines,
			push_constants,
			descriptors,
			command_builder,
			swapchain,
			renderpass,
			ui_renderpass,
			surface,
			device,
			instance,
			window: Some(window),
		}
	}

	pub fn window_resized(&mut self, current_image: &mut u32) {
		*current_image = 0;
		self.surface.surface_resolution = self.new_extent;
		self.window.as_mut().unwrap().window_extent = self.new_extent;
		let depth_image =
			VulkanEngine::create_depth_image(&self.instance, &self.device, &self.surface);
		unsafe {
			self.device
				.device
				.device_wait_idle()
				.expect("Failed to wait for the device to be idle.");
		};
		self.swapchain.recreate(
			&self.surface,
			&self.renderpass,
			&self.ui_renderpass,
			Some(depth_image),
		);
		let mut pipeline_vec = Vec::with_capacity(self.graphics_pipelines.len());
		for i in (0..self.graphics_pipelines.len()).rev() {
			let pipeline = self.graphics_pipelines.remove(i);
			pipeline_vec.push(pipeline.recreate(&self.device, self.new_extent));
		}
		for (i, j) in (0..pipeline_vec.len()).zip((0..pipeline_vec.len()).rev()) {
			if i >= j {
				break;
			}
			pipeline_vec.swap(i, j);
		}
		self.graphics_pipelines = pipeline_vec;
	}
}

pub fn compile_shaders() {
	use std::fs;

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

	let compiler_path = if !arg.is_empty() {
		arg.clone().add("/glslangValidator.exe")
	} else {
		"./".to_string()
			.add(arg.clone().add("/glslangValidator.exe").as_str())
	};
	let shader_dir_path = if !arg.is_empty() {
		arg.add("/shaders/")
	} else {
		arg.add("shaders/")
	};
	let dir = std::path::Path::new(&shader_dir_path);
	if dir.is_dir() {
		for entry in fs::read_dir(dir).expect("Failed to open the shader dir.") {
			let path = entry
				.expect("Failed to get the path of a file in the shader dir.")
				.path();
			if path.is_file() {
				let mut new_file_name = std::path::Path::new(
					path.file_name()
						.expect("failed to get the filename of a shader"),
				)
				.to_path_buf();
				new_file_name.set_extension("spv");
				let output = std::process::Command::new(compiler_path.clone())
					.args([
						path.to_str().unwrap(),
						"-V100",
						"-o",
						&shader_dir_path
							.clone()
							.add("spv/")
							.add(new_file_name.to_str().unwrap()),
					])
					.output()
					.unwrap();
				if !output.stderr.is_empty() || !output.status.success() {
					panic!(
						"Failed to compile {:?}, error: {}",
						path,
						String::from_utf8(output.stdout.to_ascii_lowercase()).unwrap()
					)
				}
			}
		}
	}
}

// pub fn calculate_normals(vertices: &mut Vec<Vertex>) {
// 	let mut point_a;
// 	let mut point_b;
// 	let mut point_c;
// 	let mut side_ab;
// 	let mut side_ac;

// 	for index in (0..vertices.len()).step_by(3) {
// 		point_a = vertices[index].position;
// 		point_b = vertices[index + 1].position;
// 		point_c = vertices[index + 2].position;

// 		side_ab = point_b - point_a;
// 		side_ac = point_c - point_a;

// 		let mut tri_normal = side_ab.cross(side_ac);
// 		tri_normal.normalize();

// 		vertices[index].normal += tri_normal;
// 		vertices[index + 1].normal += tri_normal;
// 		vertices[index + 2].normal += tri_normal;
// 	}
// }

// pub fn generate_cube() -> Vec<Vertex> {
// 	let cube_data = vec![
// 		uv::Vec3::new(-0.5, -0.5, 0.5),
// 		uv::Vec3::new(0.5, -0.5, 0.5),
// 		uv::Vec3::new(0.5, 0.5, 0.5),
// 		uv::Vec3::new(-0.5, -0.5, 0.5),
// 		uv::Vec3::new(0.5, 0.5, 0.5),
// 		uv::Vec3::new(-0.5, 0.5, 0.5),
// 		uv::Vec3::new(0.5, -0.5, 0.5),
// 		uv::Vec3::new(0.5, -0.5, -0.5),
// 		uv::Vec3::new(0.5, 0.5, -0.5),
// 		uv::Vec3::new(0.5, -0.5, 0.5),
// 		uv::Vec3::new(0.5, 0.5, -0.5),
// 		uv::Vec3::new(0.5, 0.5, 0.5),
// 		uv::Vec3::new(0.5, -0.5, -0.5),
// 		uv::Vec3::new(-0.5, -0.5, -0.5),
// 		uv::Vec3::new(-0.5, 0.5, -0.5),
// 		uv::Vec3::new(0.5, -0.5, -0.5),
// 		uv::Vec3::new(-0.5, 0.5, -0.5),
// 		uv::Vec3::new(0.5, 0.5, -0.5),
// 		uv::Vec3::new(-0.5, -0.5, -0.5),
// 		uv::Vec3::new(-0.5, -0.5, 0.5),
// 		uv::Vec3::new(-0.5, 0.5, 0.5),
// 		uv::Vec3::new(-0.5, -0.5, -0.5),
// 		uv::Vec3::new(-0.5, 0.5, 0.5),
// 		uv::Vec3::new(-0.5, 0.5, -0.5),
// 		uv::Vec3::new(-0.5, 0.5, 0.5),
// 		uv::Vec3::new(0.5, 0.5, 0.5),
// 		uv::Vec3::new(0.5, 0.5, -0.5),
// 		uv::Vec3::new(-0.5, 0.5, 0.5),
// 		uv::Vec3::new(0.5, 0.5, -0.5),
// 		uv::Vec3::new(-0.5, 0.5, -0.5),
// 		uv::Vec3::new(-0.5, -0.5, -0.5),
// 		uv::Vec3::new(0.5, -0.5, -0.5),
// 		uv::Vec3::new(0.5, -0.5, 0.5),
// 		uv::Vec3::new(-0.5, -0.5, -0.5),
// 		uv::Vec3::new(0.5, -0.5, 0.5),
// 		uv::Vec3::new(-0.5, -0.5, 0.5),
// 	];

// 	let mut vertices = Vec::with_capacity(cube_data.len());
// 	for pos in cube_data.into_iter() {
// 		vertices.push(Vertex {
// 			position: pos,
// 			..Default::default()
// 		});
// 	}
// 	calculate_normals(&mut vertices);

// 	vertices
// }
