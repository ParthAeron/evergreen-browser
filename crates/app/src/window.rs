//! Window configuration and helper utilities for Evergreen Browser.

use winit::dpi::LogicalSize;
use winit::window::WindowAttributes;

pub const DEFAULT_WINDOW_WIDTH: f64 = 1280.0;
pub const DEFAULT_WINDOW_HEIGHT: f64 = 800.0;

pub fn default_window_attributes() -> WindowAttributes {
    winit::window::Window::default_attributes()
        .with_title("Evergreen Browser")
        .with_inner_size(LogicalSize::new(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT))
}
