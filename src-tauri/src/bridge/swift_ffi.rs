//! Raw Swift FFI declarations.
//!
//! This module contains the `swift!()` macro invocations that generate
//! `unsafe extern "C"` function bindings. These functions are defined in
//! Swift via `@_cdecl` in `FFI/Exports.swift`.
//!
//! ## Safety
//!
//! All functions in this module are inherently `unsafe` because they:
//! 1. Cross the FFI boundary (calling into a different language runtime)
//! 2. Assume the Swift side is compiled and linked correctly
//! 3. Assume the Swift functions exist with matching signatures
//!
//! This unsafety is contained within this module. The parent `bridge` module
//! exposes safe wrappers that validate inputs/outputs.

use std::ffi::c_void;
use swift_rs::{Bool, SRString, swift};

// Permission
swift!(pub(super) fn claw_ax_is_process_trusted(prompt: Bool) -> Bool);

// Tree extraction (JSON)
swift!(pub(super) fn claw_ax_get_tree_json() -> SRString);

// File dump (XML)
swift!(pub(super) fn claw_ax_dump_frontmost_to_file(path: &SRString) -> SRString);

// Monitor
swift!(pub(super) fn claw_ax_start_frontmost_monitor(
    callback: *const c_void,
    dump_path: &SRString
));
swift!(pub(super) fn claw_ax_stop_frontmost_monitor());
