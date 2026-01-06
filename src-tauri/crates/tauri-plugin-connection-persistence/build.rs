fn main() {
    // Registers this crate as a Tauri plugin and wires up the mobile (Android/iOS) projects.
    tauri_plugin::Builder::new(&[])
        .android_path("android")
        .build();
}

