const COMMANDS: &[&str] = &["initialize", "update", "set_cover", "is_supported"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
