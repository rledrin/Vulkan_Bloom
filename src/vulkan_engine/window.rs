use ash::vk;
use winit::dpi;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

pub struct Window {
	pub event_loop: EventLoop<()>,
	pub window: winit::window::Window,
	pub window_extent: vk::Extent2D,
}

impl Window {
	#![allow(dead_code)]
	pub fn new(width: u32, height: u32, title: &str) -> Window {
		let event_loop = EventLoop::new();
		let window = WindowBuilder::new()
			.with_title(title)
			.with_inner_size(dpi::LogicalSize::new(f64::from(width), f64::from(height)))
			.build(&event_loop)
			.expect("Failed to build the window.");

		let window_extent = vk::Extent2D::builder()
			.height(window.inner_size().height as u32)
			.width(window.inner_size().width as u32)
			.build();

		Window {
			event_loop,
			window,
			window_extent,
		}
	}
}
