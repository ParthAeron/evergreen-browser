//! Window configuration and helper utilities for Evergreen Browser.

use winit::dpi::LogicalSize;
use winit::window::{Icon, WindowAttributes};

pub const DEFAULT_WINDOW_WIDTH: f64 = 1280.0;
pub const DEFAULT_WINDOW_HEIGHT: f64 = 800.0;
pub const APP_ICON_RGBA: &[u8] = include_bytes!("../ui/icon_32.rgba");

pub fn default_window_attributes() -> WindowAttributes {
    let mut attrs = winit::window::Window::default_attributes()
        .with_title("Evergreen Browser")
        .with_inner_size(LogicalSize::new(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT))
        .with_theme(Some(winit::window::Theme::Dark));

    if let Ok(icon) = Icon::from_rgba(APP_ICON_RGBA.to_vec(), 32, 32) {
        attrs = attrs.with_window_icon(Some(icon));
    }

    attrs
}
