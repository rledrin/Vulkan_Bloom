use std::rc::Rc;
use std::sync::{Arc, Mutex};

use ash::vk;
use gpu_alloc::{GpuAllocator, MemoryBlock, Request, UsageFlags};
use gpu_alloc_ash::AshMemoryDevice;

use super::command_buffer::CommandBufferBuilder;
use super::device::Device;

pub struct Buffer {
	pub buffer: Rc<vk::Buffer>,
	pub block: Option<MemoryBlock<vk::DeviceMemory>>,
	pub data_size: u64,
	pub allocated_size: u64,
	pub memory_requierments: vk::MemoryRequirements,
	device: Arc<ash::Device>,
	allocator: Arc<Mutex<GpuAllocator<vk::DeviceMemory>>>,
}

impl Drop for Buffer {
	fn drop(&mut self) {
		unsafe {
			let block = std::mem::take(&mut self.block).unwrap();
			self.allocator
				.lock()
				.expect("Failed to lock the Allocator's Mutex in a buffer drop.")
				.dealloc(AshMemoryDevice::wrap(&self.device), block);
			self.device.destroy_buffer(*self.buffer, None);
		};
	}
}

impl Buffer {
	#![allow(dead_code)]
	pub fn new(
		device: &Device,
		flags: vk::BufferCreateFlags,
		mut size: u64,
		usage: vk::BufferUsageFlags,
		sharing_mode: vk::SharingMode,
		allocation_type: UsageFlags,
	) -> Buffer {
		#[cfg(debug_assertions)]
		if usage & vk::BufferUsageFlags::UNIFORM_BUFFER == vk::BufferUsageFlags::UNIFORM_BUFFER
			&& size % 64 != 0
		{
			println!("Created an uniform buffer of size {} (may cause offset error) which is not multiple of 64, size was set to be {}.", size, size + size % 64);
			size -= size % 64;
			size += 64;
		}
		let buffer_create_info = vk::BufferCreateInfo::builder()
			.flags(flags)
			.size(size)
			.usage(usage)
			.sharing_mode(sharing_mode)
			.queue_family_indices(&[device.queue_family_index])
			.build();

		let buffer = unsafe {
			device
				.device
				.create_buffer(&buffer_create_info, None)
				.unwrap_or_else(|_| panic!("Failed to create a buffer of size {}.", size))
		};

		let memory_requierments = unsafe { device.device.get_buffer_memory_requirements(buffer) };

		let block = unsafe {
			device
				.allocator
				.lock()
				.expect("Failed to lock the Allocator's Mutex in a buffer creation.")
				.alloc(
					AshMemoryDevice::wrap(&device.device),
					Request {
						size: memory_requierments.size,
						align_mask: memory_requierments.alignment,
						usage: allocation_type,
						memory_types: !0,
					},
				)
				.unwrap_or_else(|_| panic!("Failed to allocate a buffer of size {}.", size))
		};

		unsafe {
			device
				.device
				.bind_buffer_memory(buffer, *block.memory(), 0)
				.unwrap_or_else(|_| panic!("Failed to bind a buffer of size {}.", size))
		}

		let allocated_size = block.size();

		let buffer = Rc::new(buffer);
		Buffer {
			buffer,
			block: Some(block),
			data_size: size,
			allocated_size,
			memory_requierments,
			device: device.device.clone(),
			allocator: device.allocator.clone(),
		}
	}

	pub fn write<T>(&mut self, offset: u64, data: Vec<T>) {
		unsafe {
			let (_, bytes, _) = data.align_to::<u8>();

			self.block
				.as_mut()
				.unwrap()
				.write_bytes(AshMemoryDevice::wrap(&self.device), offset, bytes)
				.expect("Failed to write to a buffer.");
		};
	}

	pub fn write_to_vram<T>(
		&mut self,
		device: &Device,
		command_builder: &CommandBufferBuilder,
		offset: u64,
		data: Vec<T>,
	) {
		let size = (std::mem::size_of::<T>() * data.len()) as u64;

		let mut staging_buffer = Buffer::new(
			device,
			vk::BufferCreateFlags::empty(),
			self.data_size,
			vk::BufferUsageFlags::TRANSFER_SRC,
			vk::SharingMode::EXCLUSIVE,
			UsageFlags::UPLOAD,
		);
		staging_buffer.write(0, data);

		let copy_region = [vk::BufferCopy {
			src_offset: 0,
			dst_offset: offset,
			size,
		}];

		let command_buffer = command_builder.build();
		unsafe {
			self.device.cmd_copy_buffer(
				command_buffer,
				*staging_buffer.buffer,
				*self.buffer,
				&copy_region,
			);
			self.device
				.end_command_buffer(command_buffer)
				.expect("Failed to stop a command buffer.");
		};

		let submit_info = [vk::SubmitInfo::builder()
			.command_buffers(&[command_buffer])
			.build()];
		unsafe {
			self.device
				.queue_submit(device.transfer_queue, &submit_info, vk::Fence::null())
				.expect("Failed to submit to transfer queue.");
			self.device
				.queue_wait_idle(device.transfer_queue)
				.expect("Failed to wait queue idle");

			self.device
				.free_command_buffers(command_builder.command_pool.command_pool, &[command_buffer]);
		};
	}

	pub fn read<T>(&mut self, offset: u64, data: &mut Vec<T>) {
		let buffer_len = self.data_size as usize / std::mem::size_of::<T>();
		if data.capacity() < buffer_len {
			data.reserve(buffer_len - data.capacity())
		}
		unsafe {
			data.set_len(buffer_len);

			let (_, bytes, _) = data.align_to_mut::<u8>();
			self.block
				.as_mut()
				.unwrap()
				.read_bytes(AshMemoryDevice::wrap(&self.device), offset, bytes)
				.expect("Failed to read to a buffer.");
		};
	}

	pub fn read_from_vram<T>(
		&mut self,
		device: &Device,
		command_builder: &CommandBufferBuilder,
		offset: u64,
		data: &mut Vec<T>,
	) {
		let mut staging_buffer = Buffer::new(
			device,
			vk::BufferCreateFlags::empty(),
			self.block.as_ref().unwrap().size(),
			vk::BufferUsageFlags::TRANSFER_DST,
			vk::SharingMode::EXCLUSIVE,
			UsageFlags::DOWNLOAD,
		);
		let copy_region = [vk::BufferCopy {
			src_offset: offset,
			dst_offset: 0,
			size: self.block.as_ref().unwrap().size(),
		}];

		let command_buffer = command_builder.build();
		unsafe {
			self.device.cmd_copy_buffer(
				command_buffer,
				*self.buffer,
				*staging_buffer.buffer,
				&copy_region,
			);
			self.device
				.end_command_buffer(command_buffer)
				.expect("Failed to stop a command buffer.");
		};

		let submit_info = [vk::SubmitInfo::builder()
			.command_buffers(&[command_buffer])
			.build()];
		unsafe {
			self.device
				.queue_submit(device.transfer_queue, &submit_info, vk::Fence::null())
				.expect("Failed to submit to transfer queue.");
			self.device
				.queue_wait_idle(device.transfer_queue)
				.expect("Failed to wait queue idle");

			self.device
				.free_command_buffers(command_builder.command_pool.command_pool, &[command_buffer]);
		};
		staging_buffer.read(0, data);
	}
}

pub fn aligne_offset(offset: u64) -> u64 {
	(offset - offset % 64) + 64
}

pub struct BufferOffsetRange {
	pub buffer_ref: Rc<Buffer>,
	pub offset: usize,
	pub range: usize,
}

impl BufferOffsetRange {
	#![allow(dead_code)]
	pub fn new(buffer_ref: Rc<Buffer>, offset: usize, range: usize) -> BufferOffsetRange {
		BufferOffsetRange {
			buffer_ref,
			offset,
			range,
		}
	}
}
