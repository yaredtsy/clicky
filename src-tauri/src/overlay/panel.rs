use super::highlight_panel::HighlightOverlay;

use tauri::{AppHandle, LogicalSize, Size, WebviewUrl};
use tauri_nspanel::{
    CollectionBehavior, ManagerExt, NSPoint, NSRect, NSSize, PanelBuilder, PanelLevel, StyleMask,
};

pub const OVERLAY_LABEL: &str = "highlight-overlay";

pub fn create_overlay_panel(app: &AppHandle) -> Result<(), String> {
    let panel = PanelBuilder::<_, HighlightOverlay>::new(app, OVERLAY_LABEL)
        .url(WebviewUrl::App("overlay.html".into()))
        .title("")
        .transparent(true)
        .opaque(false)
        .has_shadow(false)
        .level(PanelLevel::ScreenSaver)
        .no_activate(true)
        .style_mask(StyleMask::empty().borderless().nonactivating_panel())
        .collection_behavior(
            CollectionBehavior::new()
                .can_join_all_spaces()
                .stationary()
                .ignores_cycle()
                .full_screen_auxiliary(),
        )
        .hides_on_deactivate(false)
        .ignores_mouse_events(true)
        .with_window(|window| {
            window
                .transparent(true)
                .decorations(false)
                .skip_taskbar(true)
                .resizable(false)
                .always_on_top(true)
        })
        .size(Size::Logical(LogicalSize::new(1.0, 1.0)))
        .build()
        .map_err(|e| format!("Failed to create overlay panel: {e}"))?;

    panel.hide();

    Ok(())
}

pub fn show_highlight(app: &AppHandle, frame: crate::models::ax_node::Frame) -> Result<(), String> {
    if frame.width < 1.0 || frame.height < 1.0 {
        return hide_highlight(app);
    }

    let app_for_cb = app.clone();
    let x = frame.x;
    let y = frame.y;
    let width = frame.width;
    let height = frame.height;

    show_highlight_on_main(&app_for_cb, x, y, width, height)
}

fn show_highlight_on_main(
    app: &AppHandle,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let panel = app
        .get_webview_panel(OVERLAY_LABEL)
        .map_err(|e| format!("Overlay panel not found: {e:?}"))?;

    let monitor = monitor::get_monitor_with_cursor()
        .ok_or_else(|| "No monitor found for cursor position".to_string())?;

    let monitor_scale_factor = monitor.scale_factor();

    let monitor_size = monitor.size().to_logical::<f64>(monitor_scale_factor);

    let appkit_y = monitor_size.height - y - height;

    // let Some(window) = panel.to_window() else {
    //     return Err("Overlay WebviewWindow missing".into());
    // };

    // AX / Swift frames: top-left origin, Y downward — same as Tao logical screen coords.
    // window
    //     .set_size(Size::Logical(LogicalSize::new(width, height)))
    //     .map_err(|e| e.to_string())?;
    // window
    //     .set_position(Position::Logical(LogicalPosition::new(x, y)))
    //     .map_err(|e| e.to_string())?;

    panel.show();

    let panel = panel.as_panel();

    let rect = NSRect {
        origin: NSPoint { x: x, y: appkit_y },
        size: NSSize {
            width: width,
            height: height,
        },
    };

    panel.setFrame_display(rect, true);

    Ok(())
}

pub fn hide_highlight(app: &AppHandle) -> Result<(), String> {
    let app_for_cb = app.clone();

    let panel = app_for_cb
        .get_webview_panel(OVERLAY_LABEL)
        .map_err(|e| format!("{e:?}"))?;
    panel.hide();
    Ok(())
}
