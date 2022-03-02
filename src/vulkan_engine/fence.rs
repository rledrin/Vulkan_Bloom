use std::sync::Arc;

use ash::vk;

use super::device::Device;

pub struct Fence {
	pub fences: Vec<vk::Fence>,
	device: Arc<ash::Device>,
}

impl Drop for Fence {
	fn drop(&mut self) {
		for i in 0..self.fences.len() {
			unsafe {
				self.device.destroy_fence(self.fences[i], None);
			};
		}
	}
}

impl Fence {
	#![allow(dead_code)]
	pub fn new(device: &Device, signaled: bool, number: usize) -> Fence {
		let flags = if signaled {
			vk::FenceCreateFlags::SIGNALED
		} else {
			vk::FenceCreateFlags::empty()
		};

		// let fence_create_info = vk::FenceCreateInfo::builder().flags(flags).build();

		let mut fences = Vec::with_capacity(number);
		for _ in 0..number {
			fences.push(unsafe {
				device
					.device
					.create_fence(&vk::FenceCreateInfo::builder().flags(flags).build(), None)
					.expect("Failed to create a fence.")
			});
		}
		Fence {
			fences,
			device: device.device.clone(),
		}
	}
}
