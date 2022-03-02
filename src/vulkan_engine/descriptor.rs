use std::sync::Arc;

use ash::vk;

use super::device::Device;

pub struct DescriptorPool {
	pub descriptor_pool: vk::DescriptorPool,
	device: Arc<ash::Device>,
}

impl Drop for DescriptorPool {
	fn drop(&mut self) {
		unsafe {
			self.device
				.destroy_descriptor_pool(self.descriptor_pool, None);
		};
	}
}

impl DescriptorPool {
	#![allow(dead_code)]
	pub fn new(
		device: &Device,
		descritpor_type: Vec<(vk::DescriptorType, u32)>,
		max_set: u32,
	) -> DescriptorPool {
		let mut pool_size = Vec::with_capacity(descritpor_type.len());
		for dtype in descritpor_type.iter() {
			pool_size.push(
				vk::DescriptorPoolSize::builder()
					.ty(dtype.0)
					.descriptor_count(dtype.1)
					.build(),
			);
		}

		let descripto_pool_create_info = vk::DescriptorPoolCreateInfo::builder()
			.flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
			.max_sets(max_set)
			.pool_sizes(&pool_size)
			.build();

		let descriptor_pool = unsafe {
			device
				.device
				.create_descriptor_pool(&descripto_pool_create_info, None)
				.unwrap_or_else(|_| {
					panic!(
						"Failed to create a DecriptoPool of type {:?}",
						descritpor_type
					)
				})
		};
		DescriptorPool {
			descriptor_pool,
			device: device.device.clone(),
		}
	}
}

pub struct DescriptorSet {
	pub descriptor_set: Vec<vk::DescriptorSet>,
	pub descriptor_set_layout: Vec<vk::DescriptorSetLayout>,
	pub descriptor_pool: DescriptorPool,
	pub bindings_info: Vec<vk::DescriptorSetLayoutBinding>,
	device: Arc<ash::Device>,
}

impl Drop for DescriptorSet {
	fn drop(&mut self) {
		unsafe {
			for layout in self.descriptor_set_layout.iter() {
				self.device.destroy_descriptor_set_layout(*layout, None);
			}
		};
	}
}

impl DescriptorSet {
	#![allow(dead_code)]
	pub fn new(
		device: &Device,
		descritpor_type: Vec<(vk::DescriptorType, u32)>,
		max_set: u32,
		bindings: Vec<vk::DescriptorSetLayoutBinding>,
	) -> DescriptorSet {
		let descriptor_pool = DescriptorPool::new(device, descritpor_type, max_set);

		let descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo::builder()
			.flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
			.bindings(&bindings)
			.build();

		let mut descriptor_set_layout = Vec::with_capacity(1);
		descriptor_set_layout.push(unsafe {
			device
				.device
				.create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
				.expect("Failed to create a DescriptorSet Layout.")
		});

		let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::builder()
			.descriptor_pool(descriptor_pool.descriptor_pool)
			.set_layouts(&descriptor_set_layout)
			.build();

		let descriptor_set = unsafe {
			device
				.device
				.allocate_descriptor_sets(&descriptor_set_allocate_info)
				.expect("Failed to allocate a DescriptorSet.")
		};
		DescriptorSet {
			descriptor_set,
			descriptor_set_layout,
			descriptor_pool,
			bindings_info: bindings.to_vec(),
			device: device.device.clone(),
		}
	}

	pub fn create_another_set(
		&mut self,
		device: &Device,
		bindings: Vec<vk::DescriptorSetLayoutBinding>,
	) {
		let descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo::builder()
			.flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
			.bindings(&bindings)
			.build();

		self.descriptor_set_layout.push(unsafe {
			device
				.device
				.create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
				.expect("Failed to create a DescriptorSet Layout.")
		});

		let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::builder()
			.descriptor_pool(self.descriptor_pool.descriptor_pool)
			.set_layouts(&[*self.descriptor_set_layout.last().unwrap()])
			.build();

		self.descriptor_set.push(unsafe {
			device
				.device
				.allocate_descriptor_sets(&descriptor_set_allocate_info)
				.expect("Failed to allocate a DescriptorSet.")[0]
		});
	}

	pub fn update_descriptor_set(
		&self,
		dst_set: u32,
		dst_binding: u32,
		buffer_info: Option<Vec<vk::DescriptorBufferInfo>>,
		image_info: Option<Vec<vk::DescriptorImageInfo>>,
	) {
		let mut write_descriptor_builder = vk::WriteDescriptorSet::builder()
			.dst_set(self.descriptor_set[dst_set as usize])
			.dst_binding(dst_binding)
			.dst_array_element(0)
			.descriptor_type(self.bindings_info[dst_binding as usize].descriptor_type);
		if buffer_info.is_some() {
			write_descriptor_builder =
				write_descriptor_builder.buffer_info(buffer_info.as_ref().unwrap());
		}
		if image_info.is_some() {
			write_descriptor_builder =
				write_descriptor_builder.image_info(image_info.as_ref().unwrap());
		}
		let write_descriptor = write_descriptor_builder.build();

		unsafe {
			self.device.update_descriptor_sets(&[write_descriptor], &[]);
		};
	}
}
