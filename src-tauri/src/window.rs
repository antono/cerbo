//! The main window's size across restarts (change `frameless-desktop-window`):
//! saved on the shared exit path, applied again when the app starts.

use crate::get_context;
use cerbo_core::ui_settings::{self, WindowBounds};
use tauri::{AppHandle, LogicalSize, Manager, Size, WebviewWindow};

/// The window the size belongs to (`tauri.conf.json`).
const MAIN_WINDOW: &str = "main";
/// Smallest size worth restoring; anything below came from a broken file.
const MIN_WIDTH: f64 = 320.0;
const MIN_HEIGHT: f64 = 240.0;

/// The saved size to open the window at, or `None` to keep the default from
/// `tauri.conf.json`.
fn restorable(bounds: Option<&WindowBounds>) -> Option<LogicalSize<f64>> {
    let b = bounds?;
    (b.width.is_finite() && b.height.is_finite() && b.width >= MIN_WIDTH && b.height >= MIN_HEIGHT)
        .then(|| LogicalSize::new(b.width, b.height))
}

/// The size to save from a window of `physical` pixels at `scale`, or `None`
/// while it is maximized or minimized: those are not sizes to reopen at, so
/// the last restored size stays saved.
fn savable(physical: (u32, u32), scale: f64, maximized: bool, minimized: bool) -> Option<WindowBounds> {
    if maximized || minimized || !(scale.is_finite() && scale > 0.0) {
        return None;
    }
    let bounds = WindowBounds { width: physical.0 as f64 / scale, height: physical.1 as f64 / scale };
    restorable(Some(&bounds)).map(|_| bounds)
}

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(MAIN_WINDOW)
}

/// Open the main window at the size saved on the last exit, if any.
pub fn restore_size(app: &AppHandle) -> Result<(), String> {
    let ctx = get_context(app)?;
    let settings = ui_settings::load_ui_settings(&ctx)?;
    let (Some(size), Some(window)) = (restorable(settings.window_bounds.as_ref()), main_window(app)) else {
        return Ok(());
    };
    window.set_size(Size::Logical(size)).map_err(|e| e.to_string())
}

/// Save the main window's current size into `ui.toml`, before exiting.
pub fn save_size(app: &AppHandle) -> Result<(), String> {
    let Some(window) = main_window(app) else {
        return Ok(());
    };
    let size = window.inner_size().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let maximized = window.is_maximized().unwrap_or(false);
    let minimized = window.is_minimized().unwrap_or(false);
    let Some(bounds) = savable((size.width, size.height), scale, maximized, minimized) else {
        return Ok(());
    };
    let ctx = get_context(app)?;
    let current = ui_settings::load_ui_settings(&ctx)?;
    let merged = ui_settings::merge_ui_settings(&current, None, None, None, None, Some(bounds), None);
    ui_settings::save_ui_settings(&ctx, &merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_size_is_restored_in_logical_pixels() {
        let size = restorable(Some(&WindowBounds { width: 1370.0, height: 900.0 })).unwrap();
        assert_eq!((size.width, size.height), (1370.0, 900.0));
    }

    #[test]
    fn missing_or_broken_sizes_keep_the_default() {
        assert!(restorable(None).is_none());
        assert!(restorable(Some(&WindowBounds { width: 137.0, height: 900.0 })).is_none());
        assert!(restorable(Some(&WindowBounds { width: f64::NAN, height: 900.0 })).is_none());
        assert!(restorable(Some(&WindowBounds { width: 1370.0, height: f64::INFINITY })).is_none());
    }

    #[test]
    fn physical_size_is_saved_unscaled() {
        let bounds = savable((2740, 1800), 2.0, false, false).unwrap();
        assert_eq!((bounds.width, bounds.height), (1370.0, 900.0));
    }

    #[test]
    fn maximized_or_minimized_window_keeps_the_last_saved_size() {
        assert!(savable((2740, 1800), 2.0, true, false).is_none());
        assert!(savable((0, 0), 1.0, false, true).is_none());
        assert!(savable((1370, 900), 0.0, false, false).is_none());
    }
}
