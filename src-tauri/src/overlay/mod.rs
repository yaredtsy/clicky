//! Screen highlight overlay (NSPanel) for tree selection → on-screen rectangle.

#[cfg(target_os = "macos")]
mod highlight_panel;
#[cfg(target_os = "macos")]
mod tooltip_panel;
#[cfg(target_os = "macos")]
pub mod panel;

pub mod commands;
