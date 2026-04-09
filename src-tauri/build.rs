fn main() {
    #[cfg(target_os = "macos")]
    swift_rs::SwiftLinker::new("10.15")
        .with_package("ClawAccessibility", "./swift/ClawAccessibility")
        .link();
    tauri_build::build()
}
