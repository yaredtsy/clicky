use super::highlight_panel::HighlightOverlay;
use super::tooltip_panel::TooltipPopover;

use monitor::get_monitor_with_cursor;
use serde_json::json;
use tauri::{AppHandle, LogicalSize, Manager, Size, WebviewUrl};
use tauri_nspanel::{
    CollectionBehavior, ManagerExt, NSPoint, NSRect, NSSize, PanelBuilder, PanelLevel, StyleMask,
};

pub const OVERLAY_LABEL: &str = "highlight-overlay";
pub const TOOLTIP_LABEL: &str = "highlight-tooltip";

const TOOLTIP_WIDTH: f64 = 300.0;
const TOOLTIP_HEIGHT: f64 = 132.0;
const EDGE_MARGIN: f64 = 8.0;
const ANCHOR_GAP: f64 = 8.0;

fn overlay_collection_behavior() -> CollectionBehavior {
    CollectionBehavior::new()
        .can_join_all_spaces()
        .stationary()
        .ignores_cycle()
        .full_screen_auxiliary()
}

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
        .collection_behavior(overlay_collection_behavior())
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

pub fn create_tooltip_panel(app: &AppHandle) -> Result<(), String> {
    let panel = PanelBuilder::<_, TooltipPopover>::new(app, TOOLTIP_LABEL)
        .url(WebviewUrl::App("tooltip-overlay.html".into()))
        .title("")
        // Transparent host: only the HTML card paints white; avoids square white “halo” behind rounded corners.
        .transparent(true)
        .opaque(false)
        .has_shadow(false)
        .level(PanelLevel::ScreenSaver)
        .no_activate(true)
        .style_mask(StyleMask::empty().borderless().nonactivating_panel())
        .collection_behavior(overlay_collection_behavior())
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
        .size(Size::Logical(LogicalSize::new(
            TOOLTIP_WIDTH,
            TOOLTIP_HEIGHT,
        )))
        .build()
        .map_err(|e| format!("Failed to create tooltip panel: {e}"))?;

    panel.hide();

    Ok(())
}

/// Logical monitor size (top-left origin, Y downward) for the display under the cursor.
fn monitor_logical_size() -> Result<(f64, f64), String> {
    let monitor = get_monitor_with_cursor()
        .ok_or_else(|| "No monitor found for cursor position".to_string())?;
    let scale = monitor.scale_factor();
    let size = monitor.size().to_logical::<f64>(scale);
    Ok((size.width, size.height))
}

/// Pick tooltip top-left (logical) and placement label for popover-style positioning.
fn tooltip_placement(
    monitor_w: f64,
    monitor_h: f64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    tw: f64,
    th: f64,
) -> (f64, f64, &'static str) {
    let m = EDGE_MARGIN;
    let gap = ANCHOR_GAP;

    let fits_below = y + height + gap + th <= monitor_h - m;
    let fits_above = y >= gap + th + m;

    if fits_below {
        let anchor_cx = x + width / 2.0;
        let mut tip_x = anchor_cx - tw / 2.0;
        tip_x = tip_x.max(m).min(monitor_w - tw - m);
        let tip_y = y + height + gap;
        return (tip_x, tip_y, "below");
    }

    if fits_above {
        let anchor_cx = x + width / 2.0;
        let mut tip_x = anchor_cx - tw / 2.0;
        tip_x = tip_x.max(m).min(monitor_w - tw - m);
        let tip_y = y - gap - th;
        return (tip_x, tip_y, "above");
    }

    if x + width + gap + tw <= monitor_w - m {
        let mut tip_y = y + height / 2.0 - th / 2.0;
        tip_y = tip_y.max(m).min(monitor_h - th - m);
        return (x + width + gap, tip_y, "right");
    }

    if x >= tw + gap + m {
        let mut tip_y = y + height / 2.0 - th / 2.0;
        tip_y = tip_y.max(m).min(monitor_h - th - m);
        return (x - gap - tw, tip_y, "left");
    }

    let anchor_cx = x + width / 2.0;
    let mut tip_x = anchor_cx - tw / 2.0;
    tip_x = tip_x.max(m).min(monitor_w - tw - m);
    let tip_y = (monitor_h - th - m).max(m);
    (tip_x, tip_y, "fallback")
}

/// Push tooltip text into the tooltip webview.
///
/// **Do not** use `Panel::to_window` here: in `tauri-nspanel`, `to_window` calls
/// `remove_webview_panel`, which unregisters the panel so the next `get_webview_panel` fails.
fn push_tooltip_content(
    app: &AppHandle,
    title: &Option<String>,
    description: &Option<String>,
    placement: &str,
) -> Result<(), String> {
    let window = app
        .get_webview_window(TOOLTIP_LABEL)
        .ok_or_else(|| "Tooltip webview window not found".to_string())?;

    let payload = json!({
        "title": title.as_deref().unwrap_or(""),
        "description": description.as_deref().unwrap_or(""),
        "placement": placement,
    });

    // Guard: page may not have run inline script yet on first paint.
    let script = format!(
        "if (window.__applyTooltip) {{ window.__applyTooltip({}); }}",
        payload
    );
    window
        .eval(script)
        .map_err(|e| format!("Tooltip eval failed: {e}"))?;

    Ok(())
}

fn show_tooltip_on_main(
    app: &AppHandle,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    title: &Option<String>,
    description: &Option<String>,
) -> Result<(), String> {
    let (monitor_w, monitor_h) = monitor_logical_size()?;

    let (tip_x, tip_y_top, placement) = tooltip_placement(
        monitor_w,
        monitor_h,
        x,
        y,
        width,
        height,
        TOOLTIP_WIDTH,
        TOOLTIP_HEIGHT,
    );

    push_tooltip_content(app, title, description, placement)?;

    let panel = app
        .get_webview_panel(TOOLTIP_LABEL)
        .map_err(|e| format!("Tooltip panel not found: {e:?}"))?;

    panel.show();

    let panel = panel.as_panel();
    let appkit_y = monitor_h - tip_y_top - TOOLTIP_HEIGHT;

    let rect = NSRect {
        origin: NSPoint {
            x: tip_x,
            y: appkit_y,
        },
        size: NSSize {
            width: TOOLTIP_WIDTH,
            height: TOOLTIP_HEIGHT,
        },
    };

    panel.setFrame_display(rect, true);

    Ok(())
}

pub fn show_highlight(
    app: &AppHandle,
    frame: crate::models::ax_node::Frame,
    title: Option<String>,
    description: Option<String>,
) -> Result<(), String> {
    if frame.width < 1.0 || frame.height < 1.0 {
        return hide_highlight(app);
    }

    let app_for_cb = app.clone();
    let x = frame.x;
    let y = frame.y;
    let width = frame.width;
    let height = frame.height;

    show_highlight_on_main(&app_for_cb, x, y, width, height)?;
    if let Err(e) = show_tooltip_on_main(&app_for_cb, x, y, width, height, &title, &description) {
        eprintln!("[claw-kernel] highlight tooltip (non-fatal): {e}");
    }
    Ok(())
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

    let (_monitor_w, monitor_h) = monitor_logical_size()?;

    let appkit_y = monitor_h - y - height;

    panel.show();

    let panel = panel.as_panel();

    let rect = NSRect {
        origin: NSPoint { x, y: appkit_y },
        size: NSSize { width, height },
    };

    panel.setFrame_display(rect, true);

    Ok(())
}

pub fn hide_highlight(app: &AppHandle) -> Result<(), String> {
    let app_for_cb = app.clone();

    if let Ok(panel) = app_for_cb.get_webview_panel(OVERLAY_LABEL) {
        panel.hide();
    }

    if let Ok(panel) = app_for_cb.get_webview_panel(TOOLTIP_LABEL) {
        panel.hide();
    }

    Ok(())
}
