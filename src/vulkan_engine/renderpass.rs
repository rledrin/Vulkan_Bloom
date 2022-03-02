use std::sync::Arc;

use ash::vk::{self, AttachmentReference};

use super::device::Device;

pub struct RenderPass {
	pub renderpass: vk::RenderPass,
	device: Arc<ash::Device>,
}

impl Drop for RenderPass {
	fn drop(&mut self) {
		unsafe {
			self.device.destroy_render_pass(self.renderpass, None);
		};
	}
}

impl RenderPass {
	pub fn builder() -> RenderPassBuilder {
		RenderPassBuilder {
			..Default::default()
		}
	}
}

#[derive(Default)]
pub struct RenderPassBuilder {
	attachments: Vec<vk::AttachmentDescription>,
	subpasses: Vec<vk::SubpassDescription>,
	dependencies: Vec<vk::SubpassDependency>,
	input_attachments: Vec<AttachmentReference>,
	color_attachments: Vec<AttachmentReference>,
	depth_stencil_attachment: AttachmentReference,
	resolve_attachments: Vec<AttachmentReference>,
	preserve_attachments: Vec<u32>,
}

impl RenderPassBuilder {
	#![allow(dead_code)]
	#[allow(clippy::too_many_arguments)]
	pub fn add_attachment(
		mut self,
		format: vk::Format,
		samples: vk::SampleCountFlags,
		load_op: vk::AttachmentLoadOp,
		store_op: vk::AttachmentStoreOp,
		stencil_load_op: vk::AttachmentLoadOp,
		stencil_store_op: vk::AttachmentStoreOp,
		initial_layout: vk::ImageLayout,
		final_layout: vk::ImageLayout,
	) -> Self {
		self.attachments.push(
			vk::AttachmentDescription::builder()
				.format(format)
				.samples(samples)
				.load_op(load_op)
				.store_op(store_op)
				.stencil_load_op(stencil_load_op)
				.stencil_store_op(stencil_store_op)
				.stencil_load_op(stencil_load_op)
				.initial_layout(initial_layout)
				.final_layout(final_layout)
				.build(),
		);
		self
	}

	pub fn add_subpasses(
		mut self,
		pipeline_bind_point: vk::PipelineBindPoint,
		input_attachments_layout: Vec<vk::ImageLayout>,
		number_of_color_attachment: u32,
		depth_stencil_attachment: Option<vk::AttachmentReference>,
		resolve_attachments_layout: Vec<vk::ImageLayout>,
		preserve_attachments: Vec<u32>,
	) -> Self {
		for (attachment, layout) in input_attachments_layout.into_iter().enumerate() {
			self.input_attachments.push(
				vk::AttachmentReference::builder()
					.attachment(attachment as u32)
					.layout(layout)
					.build(),
			);
		}

		for i in 0..number_of_color_attachment {
			self.color_attachments.push(
				vk::AttachmentReference::builder()
					.attachment(i)
					.layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
					.build(),
			);
		}

		for (attachment, layout) in resolve_attachments_layout.into_iter().enumerate() {
			self.resolve_attachments.push(
				vk::AttachmentReference::builder()
					.attachment(attachment as u32)
					.layout(layout)
					.build(),
			);
		}

		self.preserve_attachments = preserve_attachments;

		self.subpasses.push(vk::SubpassDescription {
			flags: vk::SubpassDescriptionFlags::empty(),
			pipeline_bind_point,
			color_attachment_count: 0,
			p_color_attachments: std::ptr::null(),
			p_depth_stencil_attachment: std::ptr::null(),
			input_attachment_count: 0,
			p_input_attachments: std::ptr::null(),
			p_resolve_attachments: std::ptr::null(),
			preserve_attachment_count: 0,
			p_preserve_attachments: std::ptr::null(),
		});

		if !self.color_attachments.is_empty() {
			if let Some(elem) = self.subpasses.last_mut() {
				elem.p_color_attachments = self.color_attachments.as_ptr();
				elem.color_attachment_count = self.color_attachments.len() as u32;
			}
		}
		if let Some(depth_stencil) = depth_stencil_attachment {
			self.depth_stencil_attachment = depth_stencil;
			if let Some(elem) = self.subpasses.last_mut() {
				elem.p_depth_stencil_attachment = &self.depth_stencil_attachment;
			}
		}
		if !self.input_attachments.is_empty() {
			if let Some(elem) = self.subpasses.last_mut() {
				elem.p_input_attachments = self.input_attachments.as_ptr();
				elem.input_attachment_count = self.input_attachments.len() as u32;
			}
		}
		if !self.resolve_attachments.is_empty() {
			if let Some(elem) = self.subpasses.last_mut() {
				elem.p_resolve_attachments = self.resolve_attachments.as_ptr();
			}
		}
		if !self.preserve_attachments.is_empty() {
			if let Some(elem) = self.subpasses.last_mut() {
				elem.p_preserve_attachments = self.preserve_attachments.as_ptr();
				elem.preserve_attachment_count = self.preserve_attachments.len() as u32;
			}
		}
		self
	}

	#[allow(clippy::too_many_arguments)]
	pub fn add_dependencies(
		mut self,
		src_subpass: u32,
		dst_subpass: u32,
		src_stage_mask: vk::PipelineStageFlags,
		dst_stage_mask: vk::PipelineStageFlags,
		src_access_mask: vk::AccessFlags,
		dst_access_mask: vk::AccessFlags,
		dependency_flags: vk::DependencyFlags,
	) -> Self {
		self.dependencies.push(
			vk::SubpassDependency::builder()
				.src_subpass(src_subpass)
				.dst_subpass(dst_subpass)
				.src_stage_mask(src_stage_mask)
				.dst_stage_mask(dst_stage_mask)
				.src_access_mask(src_access_mask)
				.dst_access_mask(dst_access_mask)
				.dependency_flags(dependency_flags)
				.build(),
		);
		self
	}

	pub fn build(self, device: &Device) -> RenderPass {
		let render_pass_create_info = vk::RenderPassCreateInfo::builder()
			.attachments(&self.attachments)
			.subpasses(&self.subpasses)
			.dependencies(&self.dependencies)
			.build();

		let renderpass = unsafe {
			device
				.device
				.create_render_pass(&render_pass_create_info, None)
				.expect("Failed to build a RenderPass.")
		};

		RenderPass {
			renderpass,
			device: device.device.clone(),
		}
	}
}
