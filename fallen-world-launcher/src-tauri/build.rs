fn main() {
    // In dev/debug mode, Tauri doesn't strictly need the icon
    // This allows running npm run tauri dev without bundling requirements
    tauri_build::build()
}
