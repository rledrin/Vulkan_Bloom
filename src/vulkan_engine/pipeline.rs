use std::sync::Arc;

use ash::vk;

use super::descriptor::DescriptorSet;
use super::device::Device;
use super::push_constant::PushConstant;
use super::renderpass;
use super::shader_module;

// #[derive(Clone)]
pub struct GraphicsPipeline {
	pub pipeline: vk::Pipeline,
	pub pipeline_layout: vk::PipelineLayout,
	builder: Option<GraphicsPipelineBuilder>,
	device: Arc<ash::Device>,
}

impl Drop for GraphicsPipeline {
	fn drop(&mut self) {
		unsafe {
			self.device.destroy_pipeline(self.pipeline, None);
			self.device
				.destroy_pipeline_layout(self.pipeline_layout, None);
		};
	}
}

impl GraphicsPipeline {
	pub fn builder() -> GraphicsPipelineBuilder {
		GraphicsPipelineBuilder {
			..Default::default()
		}
	}

	pub fn recreate(mut self, device: &Device, extent: vk::Extent2D) -> GraphicsPipeline {
		let viewport = vk::Viewport::builder()
			.width(extent.width as f32)
			.height(extent.height as f32)
			.min_depth(0.0)
			.max_depth(1.0)
			.build();
		let scissor = vk::Rect2D::builder()
			.extent(
				vk::Extent2D::builder()
					.width(extent.width)
					.height(extent.height)
					.build(),
			)
			.build();

		self.builder.as_mut().unwrap().viewports[0] = viewport;
		self.builder.as_mut().unwrap().scissors[0] = scissor;

		let builder = std::mem::take(&mut self.builder);

		let pipeline = builder.unwrap().build(device);

		pipeline
	}
}

#[derive(Default)]
pub struct GraphicsPipelineBuilder {
	flags: vk::PipelineCreateFlags,
	vertex_module: Option<shader_module::ShaderModule>,
	fragment_module: Option<shader_module::ShaderModule>,
	vertex_binding_description_create_info: Vec<vk::VertexInputBindingDescription>,
	vertex_attribute_descriptions_create_info: Vec<vk::VertexInputAttributeDescription>,
	assembly_state_create_info: vk::PipelineInputAssemblyStateCreateInfo,
	tessellation_state_create_info: vk::PipelineTessellationStateCreateInfo,
	viewports: Vec<vk::Viewport>,
	scissors: Vec<vk::Rect2D>,
	rasterization_state_create_info: vk::PipelineRasterizationStateCreateInfo,
	multisample_state_create_info: vk::PipelineMultisampleStateCreateInfo,
	depth_stencil_state_create_info: vk::PipelineDepthStencilStateCreateInfo,
	color_blend_attachments: Vec<vk::PipelineColorBlendAttachmentState>,
	color_blend_state_create_info: vk::PipelineColorBlendStateCreateInfo,
	dynamic_states: Vec<vk::DynamicState>,
	renderpass: vk::RenderPass,
	subpass_index: u32,
	descriptor_sets: Vec<vk::DescriptorSetLayout>,
	push_constants: Vec<vk::PushConstantRange>,
	base_pipeline: vk::Pipeline,
	base_pipeline_index: i32,
}

impl GraphicsPipelineBuilder {
	#![allow(dead_code)]
	pub fn vertex_module_1(mut self, vertex_module: shader_module::ShaderModule) -> Self {
		self.vertex_module = Some(vertex_module);
		self
	}

	pub fn fragment_module_2(mut self, fragment_module: shader_module::ShaderModule) -> Self {
		// self.pipeline_shader_stage_create_info.push(
		// 	vk::PipelineShaderStageCreateInfo::builder()
		// 		.stage(vk::ShaderStageFlags::FRAGMENT)
		// 		.module(fragment_module.shader_module)
		// 		.name(fragment_module.entry_point.as_c_str())
		// 		.build(),
		// );
		self.fragment_module = Some(fragment_module);
		self
	}

	pub fn add_vertex_binding_3(
		mut self,
		binding: u32,
		stride: u32,
		inpute_rate: vk::VertexInputRate,
	) -> Self {
		self.vertex_binding_description_create_info.push(
			vk::VertexInputBindingDescription::builder()
				.binding(binding)
				.stride(stride)
				.input_rate(inpute_rate)
				.build(),
		);
		self
	}

	pub fn add_vertex_attribute_4(
		mut self,
		location: u32,
		binding: u32,
		format: vk::Format,
		offset: u32,
	) -> Self {
		self.vertex_attribute_descriptions_create_info.push(
			vk::VertexInputAttributeDescription::builder()
				.location(location)
				.binding(binding)
				.format(format)
				.offset(offset)
				.build(),
		);
		self
	}

	pub fn assembly_state_5(
		mut self,
		topology: vk::PrimitiveTopology,
		primitive_restart_enable: bool,
	) -> Self {
		self.assembly_state_create_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
			.topology(topology)
			.primitive_restart_enable(primitive_restart_enable)
			.build();
		self
	}

	pub fn tessellation_state_6(mut self, patch_control_points: u32) -> Self {
		self.tessellation_state_create_info = vk::PipelineTessellationStateCreateInfo::builder()
			.patch_control_points(patch_control_points)
			.build();
		self
	}

	pub fn add_viewport_7(mut self, viewport: vk::Viewport) -> Self {
		self.viewports.push(viewport);
		self
	}

	pub fn add_scissor_8(mut self, scissor: vk::Rect2D) -> Self {
		self.scissors.push(scissor);
		self
	}

	#[allow(clippy::too_many_arguments)]
	pub fn rasterization_state_9(
		mut self,
		depth_clamp_enable: bool,
		rasterizer_discard_enable: bool,
		polygon_mode: vk::PolygonMode,
		cull_mode: vk::CullModeFlags,
		front_face: vk::FrontFace,
		depth_bias_enable: bool,
		depth_bias_constant_factor: f32,
		depth_bias_clamp: f32,
		depth_bias_slope_factor: f32,
		line_width: f32,
	) -> Self {
		self.rasterization_state_create_info = vk::PipelineRasterizationStateCreateInfo::builder()
			.depth_clamp_enable(depth_clamp_enable)
			.rasterizer_discard_enable(rasterizer_discard_enable)
			.polygon_mode(polygon_mode)
			.cull_mode(cull_mode)
			.front_face(front_face)
			.depth_bias_enable(depth_bias_enable)
			.depth_bias_constant_factor(depth_bias_constant_factor)
			.depth_bias_clamp(depth_bias_clamp)
			.depth_bias_slope_factor(depth_bias_slope_factor)
			.line_width(line_width)
			.build();
		self
	}

	pub fn multisample_state_10(
		mut self,
		rasterization_samples: vk::SampleCountFlags,
		sample_shading_enable: bool,
		min_sample_shading: f32,
		sample_mask: &[vk::SampleMask],
		alpha_to_coverage_enable: bool,
		alpha_to_one_enable: bool,
	) -> Self {
		self.multisample_state_create_info = vk::PipelineMultisampleStateCreateInfo::builder()
			.rasterization_samples(rasterization_samples)
			.sample_shading_enable(sample_shading_enable)
			.min_sample_shading(min_sample_shading)
			.sample_mask(sample_mask)
			.alpha_to_coverage_enable(alpha_to_coverage_enable)
			.alpha_to_one_enable(alpha_to_one_enable)
			.build();
		self
	}

	#[allow(clippy::too_many_arguments)]
	pub fn depth_stencil_state_11(
		mut self,
		depth_test_enable: bool,
		depth_write_enable: bool,
		depth_compare_op: vk::CompareOp,
		depth_bounds_test_enable: bool,
		stencil_test_enable: bool,
		front: vk::StencilOpState,
		back: vk::StencilOpState,
		min_depth_bounds: f32,
		max_depth_bounds: f32,
	) -> Self {
		self.depth_stencil_state_create_info = vk::PipelineDepthStencilStateCreateInfo::builder()
			.depth_test_enable(depth_test_enable)
			.depth_write_enable(depth_write_enable)
			.depth_compare_op(depth_compare_op)
			.depth_bounds_test_enable(depth_bounds_test_enable)
			.stencil_test_enable(stencil_test_enable)
			.front(front)
			.back(back)
			.min_depth_bounds(min_depth_bounds)
			.max_depth_bounds(max_depth_bounds)
			.build();
		self
	}

	#[allow(clippy::too_many_arguments)]
	pub fn add_color_blend_attachments_12(
		mut self,
		blend_enable: bool,
		color_write_mask: vk::ColorComponentFlags,
		src_color_blend_factor: vk::BlendFactor,
		dst_color_blend_factor: vk::BlendFactor,
		color_blend_op: vk::BlendOp,
		src_alpha_blend_factor: vk::BlendFactor,
		dst_alpha_blend_factor: vk::BlendFactor,
		alpha_blend_op: vk::BlendOp,
	) -> Self {
		self.color_blend_attachments.push(
			vk::PipelineColorBlendAttachmentState::builder()
				.blend_enable(blend_enable)
				.color_write_mask(color_write_mask)
				.src_color_blend_factor(src_color_blend_factor)
				.dst_color_blend_factor(dst_color_blend_factor)
				.color_blend_op(color_blend_op)
				.src_alpha_blend_factor(src_alpha_blend_factor)
				.dst_alpha_blend_factor(dst_alpha_blend_factor)
				.alpha_blend_op(alpha_blend_op)
				.build(),
		);
		self
	}

	pub fn color_blend_state_13(
		mut self,
		logic_op_enable: bool,
		logic_op: vk::LogicOp,
		blend_constants: [f32; 4],
	) -> Self {
		self.color_blend_state_create_info = vk::PipelineColorBlendStateCreateInfo::builder()
			.logic_op_enable(logic_op_enable)
			.logic_op(logic_op)
			.attachments(&self.color_blend_attachments)
			.blend_constants(blend_constants)
			.build();
		self
	}

	pub fn add_dynamic_state_14(mut self, dynamic_state: vk::DynamicState) -> Self {
		self.dynamic_states.push(dynamic_state);
		self
	}

	pub fn add_descriptor_set_15(
		mut self,
		descriptor_set: &DescriptorSet,
		set_index: usize,
	) -> Self {
		self.descriptor_sets
			.push(descriptor_set.descriptor_set_layout[set_index]);
		self
	}

	pub fn add_push_constant_16(mut self, push_constant: &PushConstant) -> Self {
		self.push_constants.push(push_constant.range);
		self
	}

	pub fn renderpass_17(
		mut self,
		renderpass: &renderpass::RenderPass,
		subpass_index: u32,
	) -> Self {
		self.renderpass = renderpass.renderpass;
		self.subpass_index = subpass_index;
		self
	}

	pub fn base_pipeline_18(
		mut self,
		base_pipeline: vk::Pipeline,
		base_pipeline_index: i32,
	) -> Self {
		self.base_pipeline = base_pipeline;
		self.base_pipeline_index = base_pipeline_index;
		self
	}

	pub fn flags_19(mut self, flags: vk::PipelineCreateFlags) -> Self {
		self.flags = flags;
		self
	}

	pub fn build(self, device: &Device) -> GraphicsPipeline {
		let mut pipeline_shader_stage_create_info = Vec::with_capacity(2);
		if self.vertex_module.is_some() {
			let vertex_module = self.vertex_module.as_ref().unwrap();
			pipeline_shader_stage_create_info.push(
				vk::PipelineShaderStageCreateInfo::builder()
					.module(vertex_module.shader_module)
					.name(vertex_module.entry_point.as_c_str())
					.stage(vk::ShaderStageFlags::VERTEX)
					.flags(vk::PipelineShaderStageCreateFlags::empty())
					.build(),
			);
		}
		if self.fragment_module.is_some() {
			let fragment_module = self.fragment_module.as_ref().unwrap();
			pipeline_shader_stage_create_info.push(
				vk::PipelineShaderStageCreateInfo::builder()
					.module(fragment_module.shader_module)
					.name(fragment_module.entry_point.as_c_str())
					.stage(vk::ShaderStageFlags::FRAGMENT)
					.flags(vk::PipelineShaderStageCreateFlags::empty())
					.build(),
			);
		}

		let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::builder()
			.vertex_binding_descriptions(&self.vertex_binding_description_create_info)
			.vertex_attribute_descriptions(&self.vertex_attribute_descriptions_create_info)
			.build();

		let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
			.viewports(&self.viewports)
			.scissors(&self.scissors)
			.build();

		let dynamic_state_create_info = vk::PipelineDynamicStateCreateInfo::builder()
			.dynamic_states(&self.dynamic_states)
			.build();

		let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder()
			.push_constant_ranges(&self.push_constants)
			.set_layouts(&self.descriptor_sets)
			.build();

		let pipeline_layout = unsafe {
			device
				.device
				.create_pipeline_layout(&pipeline_layout_create_info, None)
				.expect("Failed to create a pipeline layout.")
		};

		let pipeline_create_info = [vk::GraphicsPipelineCreateInfo::builder()
			.flags(self.flags)
			.stages(&pipeline_shader_stage_create_info)
			.vertex_input_state(&vertex_input_state_create_info)
			.input_assembly_state(&self.assembly_state_create_info)
			.tessellation_state(&self.tessellation_state_create_info)
			.viewport_state(&viewport_state)
			.rasterization_state(&self.rasterization_state_create_info)
			.multisample_state(&self.multisample_state_create_info)
			.depth_stencil_state(&self.depth_stencil_state_create_info)
			.color_blend_state(&self.color_blend_state_create_info)
			.dynamic_state(&dynamic_state_create_info)
			.layout(pipeline_layout)
			.render_pass(self.renderpass)
			.subpass(self.subpass_index)
			.base_pipeline_handle(self.base_pipeline)
			.base_pipeline_index(self.base_pipeline_index)
			.build()];

		let pipeline = unsafe {
			device
				.device
				.create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_create_info, None)
				.expect("Failed to create a graphic pipeline.")
		};
		let pipeline = pipeline[0];
		GraphicsPipeline {
			pipeline,
			pipeline_layout,
			builder: Some(self),
			device: device.device.clone(),
		}
	}
}

pub struct ComputePipeline {
	pub pipeline: vk::Pipeline,
	pub pipeline_layout: vk::PipelineLayout,
	device: Arc<ash::Device>,
}

impl Drop for ComputePipeline {
	#![allow(dead_code)]
	fn drop(&mut self) {
		unsafe {
			self.device.destroy_pipeline(self.pipeline, None);
			self.device
				.destroy_pipeline_layout(self.pipeline_layout, None);
		};
	}
}

impl ComputePipeline {
	#![allow(dead_code)]
	pub fn builder() -> ComputePipelineBuilder {
		ComputePipelineBuilder {
			..Default::default()
		}
	}
}

#[derive(Default)]
pub struct ComputePipelineBuilder {
	pipeline_create_flags: vk::PipelineCreateFlags,
	shader_stage_create_info: vk::PipelineShaderStageCreateInfo,
	descriptor_sets: Vec<vk::DescriptorSetLayout>,
	push_constants: Vec<vk::PushConstantRange>,
	base_pipeline: vk::Pipeline,
	base_pipeline_index: i32,
}

impl ComputePipelineBuilder {
	#![allow(dead_code)]
	pub fn flags(mut self, pipeline_create_flags: vk::PipelineCreateFlags) -> Self {
		self.pipeline_create_flags = pipeline_create_flags;
		self
	}

	pub fn compute_module(
		mut self,
		compute_module: &shader_module::ShaderModule,
		flags: vk::PipelineShaderStageCreateFlags,
	) -> Self {
		self.shader_stage_create_info = vk::PipelineShaderStageCreateInfo::builder()
			.flags(flags)
			.module(compute_module.shader_module)
			.name(compute_module.entry_point.as_c_str())
			.stage(vk::ShaderStageFlags::COMPUTE)
			.build();

		self
	}

	pub fn add_descriptor_set(mut self, descriptor_set: &DescriptorSet, set_index: usize) -> Self {
		self.descriptor_sets
			.push(descriptor_set.descriptor_set_layout[set_index]);
		self
	}

	pub fn add_push_constant(mut self, push_constant: &PushConstant) -> Self {
		self.push_constants.push(push_constant.range);
		self
	}

	pub fn base_pipeline(mut self, base_pipeline: vk::Pipeline, base_pipeline_index: i32) -> Self {
		self.base_pipeline = base_pipeline;
		self.base_pipeline_index = base_pipeline_index;
		self
	}

	pub fn build(self, device: &Device) -> ComputePipeline {
		let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder()
			.push_constant_ranges(&self.push_constants)
			.set_layouts(&self.descriptor_sets)
			.build();

		let pipeline_layout = unsafe {
			device
				.device
				.create_pipeline_layout(&pipeline_layout_create_info, None)
				.expect("Failed to create a pipeline layout.")
		};

		let pipeline_create_infos = vk::ComputePipelineCreateInfo::builder()
			.flags(self.pipeline_create_flags)
			.stage(self.shader_stage_create_info)
			.layout(pipeline_layout)
			.base_pipeline_handle(self.base_pipeline)
			.base_pipeline_index(self.base_pipeline_index)
			.build();

		let pipeline = unsafe {
			device
				.device
				.create_compute_pipelines(vk::PipelineCache::null(), &[pipeline_create_infos], None)
				.expect("Failed to create a compute pipeline.")
		};

		let pipeline = pipeline[0];

		ComputePipeline {
			pipeline,
			pipeline_layout,
			device: device.device.clone(),
		}
	}
}
