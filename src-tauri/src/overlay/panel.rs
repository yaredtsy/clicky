//! Transparent NSPanel used as a screen-space highlight rectangle.
//!
//! ## Position
//! AX frames from Swift use **top-left** screen origin with **Y downward** (see `AXHelpers.swift`).
//! That matches Tauri/Tao `LogicalPosition`, so we use `WebviewWindow::set_position` / `set_size`
//! instead of raw `NSWindow::setFrame` (which WRY can fight, so the rect would appear “stuck”).
//!
//! ## Transparency
//! On macOS, a transparent webview requires Tauri’s **`macos-private-api`** feature plus
//! `.transparent(true)` on the `WebviewWindowBuilder`.

use std::sync::mpsc;

use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Position, Size, WebviewUrl,
};
use tauri_nspanel::{CollectionBehavior, ManagerExt, PanelBuilder, PanelLevel, StyleMask};

use super::highlight_panel::HighlightOverlay;

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

    let (tx, rx) = mpsc::sync_channel(1);
    let runner = app.clone();
    let app_for_cb = app.clone();
    let x = frame.x;
    let y = frame.y;
    let width = frame.width;
    let height = frame.height;

    runner
        .run_on_main_thread(move || {
            let res = show_highlight_on_main(&app_for_cb, x, y, width, height);
            let _ = tx.send(res);
        })
        .map_err(|e| e.to_string())?;

    rx.recv()
        .map_err(|_| "highlight channel closed".to_string())?
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

    let Some(window) = panel.to_window() else {
        return Err("Overlay WebviewWindow missing".into());
    };

    // AX / Swift frames: top-left origin, Y downward — same as Tao logical screen coords.
    window
        .set_size(Size::Logical(LogicalSize::new(width, height)))
        .map_err(|e| e.to_string())?;
    window
        .set_position(Position::Logical(LogicalPosition::new(x, y)))
        .map_err(|e| e.to_string())?;

    panel.order_front_regardless();

    let payload = serde_json::json!({
        "x": x,
        "y": y,
        "width": width,
        "height": height,
    });
    let _ = window.emit("highlight-update", payload);

    Ok(())
}

pub fn hide_highlight(app: &AppHandle) -> Result<(), String> {
    let (tx, rx) = mpsc::sync_channel(1);
    let runner = app.clone();
    let app_for_cb = app.clone();
    runner
        .run_on_main_thread(move || {
            let res = (|| {
                let panel = app_for_cb
                    .get_webview_panel(OVERLAY_LABEL)
                    .map_err(|e| format!("{e:?}"))?;
                panel.hide();
                Ok::<(), String>(())
            })();
            let _ = tx.send(res);
        })
        .map_err(|e| e.to_string())?;

    rx.recv()
        .map_err(|_| "highlight channel closed".to_string())?
}
