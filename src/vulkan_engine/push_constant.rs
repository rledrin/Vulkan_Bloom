use ash::vk;

pub struct PushConstant {
	pub range: vk::PushConstantRange,
	pub data: Vec<u8>,
}

impl PushConstant {
	#![allow(dead_code)]

	pub fn new<T>(
		offset: u32,
		size: u32,
		stage_flags: vk::ShaderStageFlags,
		data: Vec<T>,
	) -> PushConstant {
		let range = vk::PushConstantRange::builder()
			.offset(offset)
			.size(size)
			.stage_flags(stage_flags)
			.build();
		let aligned_data = unsafe { data.align_to::<u8>() };
		PushConstant {
			range,
			data: aligned_data.1.to_vec(),
		}
	}

	pub fn set_data<T>(&mut self, data: Vec<T>) {
		let aligned_data = unsafe { data.align_to::<u8>() };
		self.data = aligned_data.1.to_vec();
	}
}
