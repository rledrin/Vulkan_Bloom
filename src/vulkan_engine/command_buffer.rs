use std::sync::Arc;

use ash::vk;

use super::device::Device;

pub struct CommandPool {
	pub command_pool: vk::CommandPool,
	device: Arc<ash::Device>,
}

impl Drop for CommandPool {
	fn drop(&mut self) {
		unsafe {
			self.device.destroy_command_pool(self.command_pool, None);
		};
	}
}

impl CommandPool {
	#![allow(dead_code)]
	pub fn new(device: &Device, flags: vk::CommandPoolCreateFlags) -> CommandPool {
		let command_pool_create_info = vk::CommandPoolCreateInfo::builder()
			.flags(flags)
			.queue_family_index(device.queue_family_index)
			.build();

		let command_pool = unsafe {
			device
				.device
				.create_command_pool(&command_pool_create_info, None)
				.expect("Failed to create a CommandPool.")
		};

		CommandPool {
			command_pool,
			device: device.device.clone(),
		}
	}
}

pub struct CommandBuffer {
	#[allow(unused)]
	pub command_buffer: Vec<vk::CommandBuffer>,
}

impl CommandBuffer {
	#![allow(dead_code)]
	pub fn new(
		device: &Device,
		command_pool: &CommandPool,
		command_buffer_count: u32,
		level: vk::CommandBufferLevel,
	) -> CommandBuffer {
		let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
			.command_buffer_count(command_buffer_count)
			.command_pool(command_pool.command_pool)
			.level(level)
			.build();

		let command_buffer = unsafe {
			device
				.device
				.allocate_command_buffers(&command_buffer_allocate_info)
				.expect("Failed to allocate a CommandBuffer.")
		};
		CommandBuffer { command_buffer }
	}
}

#[repr(u32)]
#[allow(dead_code)]
pub enum CommandBufferUsage {
	OneTimeSubmit = vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT.as_raw(),
	MultipleSubmit = 0,
	SimultaneousUse = vk::CommandBufferUsageFlags::SIMULTANEOUS_USE.as_raw(),
}

#[allow(dead_code)]
pub struct CommandBufferBuilder {
	pub command_pool: CommandPool,
	buffer_usage: vk::CommandBufferUsageFlags,
	level: vk::CommandBufferLevel,
	pub command_buffer_allocate_info: vk::CommandBufferAllocateInfo,
	pub command_buffer_begin_info: vk::CommandBufferBeginInfo,
}

impl CommandBufferBuilder {
	#![allow(dead_code)]
	pub fn primary(device: &Device, buffer_usage: CommandBufferUsage) -> CommandBufferBuilder {
		let (command_pool, buffer_usage) = match buffer_usage {
			CommandBufferUsage::OneTimeSubmit => (
				CommandPool::new(device, vk::CommandPoolCreateFlags::TRANSIENT),
				vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
			),
			CommandBufferUsage::MultipleSubmit => (
				CommandPool::new(device, vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
				vk::CommandBufferUsageFlags::empty(),
			),
			CommandBufferUsage::SimultaneousUse => (
				CommandPool::new(device, vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
				vk::CommandBufferUsageFlags::SIMULTANEOUS_USE,
			),
		};

		let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
			.command_buffer_count(1)
			.command_pool(command_pool.command_pool)
			.level(vk::CommandBufferLevel::PRIMARY)
			.build();

		let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
			.flags(buffer_usage)
			.build();

		CommandBufferBuilder {
			command_pool,
			buffer_usage,
			level: vk::CommandBufferLevel::PRIMARY,
			command_buffer_allocate_info,
			command_buffer_begin_info,
		}
	}

	pub fn secondary(
		device: &Device,
		buffer_usage: CommandBufferUsage,
		inheritance_info: &vk::CommandBufferInheritanceInfo,
	) -> CommandBufferBuilder {
		let (command_pool, buffer_usage) = match buffer_usage {
			CommandBufferUsage::OneTimeSubmit => (
				CommandPool::new(device, vk::CommandPoolCreateFlags::TRANSIENT),
				vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
			),
			CommandBufferUsage::MultipleSubmit => (
				CommandPool::new(device, vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
				vk::CommandBufferUsageFlags::empty(),
			),
			CommandBufferUsage::SimultaneousUse => (
				CommandPool::new(device, vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
				vk::CommandBufferUsageFlags::SIMULTANEOUS_USE,
			),
		};

		let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
			.command_buffer_count(1)
			.command_pool(command_pool.command_pool)
			.level(vk::CommandBufferLevel::SECONDARY)
			.build();

		let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
			.flags(buffer_usage)
			.inheritance_info(inheritance_info)
			.build();

		CommandBufferBuilder {
			command_pool,
			buffer_usage,
			level: vk::CommandBufferLevel::SECONDARY,
			command_buffer_allocate_info,
			command_buffer_begin_info,
		}
	}

	pub fn build(&self) -> vk::CommandBuffer {
		let command_buffer = unsafe {
			self.command_pool
				.device
				.allocate_command_buffers(&self.command_buffer_allocate_info)
				.expect("Failed to allocate a CommandBuffer.")
		};

		unsafe {
			self.command_pool
				.device
				.begin_command_buffer(command_buffer[0], &self.command_buffer_begin_info)
				.expect("Failed to begin the recording of a CommandBuffer.");
		};

		command_buffer[0]
	}
}
