use std::ffi::CString;
use std::fs::File;
use std::io;
use std::io::{Read, Seek};
use std::slice;
use std::sync::Arc;

use ash::vk;

use super::device::Device;

pub struct ShaderModule {
	pub shader_module: vk::ShaderModule,
	pub entry_point: CString,
	device: Arc<ash::Device>,
}

impl Drop for ShaderModule {
	fn drop(&mut self) {
		unsafe {
			self.device.destroy_shader_module(self.shader_module, None);
		};
	}
}

impl ShaderModule {
	#![allow(dead_code)]
	fn read_spv(shader_path: &str) -> io::Result<Vec<u32>> {
		let mut x = File::open(shader_path)
			.unwrap_or_else(|_| panic!("Failed to open the shader_path: {}.", shader_path));
		let size = x.seek(io::SeekFrom::End(0))?;
		if size % 4 != 0 {
			return Err(io::Error::new(
				io::ErrorKind::InvalidData,
				"input length not divisible by 4",
			));
		}
		if size > usize::max_value() as u64 {
			return Err(io::Error::new(io::ErrorKind::InvalidData, "input too long"));
		}
		let words = (size / 4) as usize;
		let mut result = vec![0u32; words];
		x.seek(io::SeekFrom::Start(0))?;
		x.read_exact(unsafe {
			slice::from_raw_parts_mut(result.as_mut_ptr() as *mut u8, words * 4)
		})?;
		const MAGIC_NUMBER: u32 = 0x0723_0203;
		if !result.is_empty() && result[0] == MAGIC_NUMBER.swap_bytes() {
			for word in &mut result {
				*word = word.swap_bytes();
			}
		}
		if result.is_empty() || result[0] != MAGIC_NUMBER {
			return Err(io::Error::new(
				io::ErrorKind::InvalidData,
				"input missing SPIR-V magic number",
			));
		}
		Ok(result)
	}

	pub fn new(device: &Device, shader_path: &str, entry_point: &str) -> ShaderModule {
		let arg = std::path::Path::new(&std::env::args().into_iter().next().unwrap())
			.parent()
			.unwrap()
			.parent()
			.unwrap()
			.to_str()
			.unwrap()
			.to_owned() + "/../";

		let shader_path = arg + shader_path;

		let bytes_code = ShaderModule::read_spv(&shader_path).expect("Failed to read a shader.");

		let shader_module_create_info = vk::ShaderModuleCreateInfo::builder()
			.code(&bytes_code)
			.build();

		let shader_module = unsafe {
			device
				.device
				.create_shader_module(&shader_module_create_info, None)
				.unwrap_or_else(|_| {
					panic!(
						"Failed to create a Shader Module, shader_path: {}.",
						shader_path
					)
				})
		};

		ShaderModule {
			shader_module,
			entry_point: CString::new(entry_point)
				.expect("Failed to convert the shader_module's entry point(&str) to a CString"),
			device: device.device.clone(),
		}
	}
}
