use std::sync::Arc;

use ash::vk;

use super::device::Device;

pub struct Semaphore {
	pub semaphores: Vec<vk::Semaphore>,
	device: Arc<ash::Device>,
}

impl Drop for Semaphore {
	fn drop(&mut self) {
		for i in 0..self.semaphores.len() {
			unsafe {
				self.device.destroy_semaphore(self.semaphores[i], None);
			};
		}
	}
}

impl Semaphore {
	#![allow(dead_code)]
	pub fn new(device: &Device, number: usize) -> Semaphore {
		let semaphore_create_info = vk::SemaphoreCreateInfo::builder().build();

		let mut semaphores = Vec::with_capacity(number);
		for _ in 0..number {
			semaphores.push(unsafe {
				device
					.device
					.create_semaphore(&semaphore_create_info, None)
					.expect("Failed to create a semaphore.")
			});
		}
		Semaphore {
			semaphores,
			device: device.device.clone(),
		}
	}
}
