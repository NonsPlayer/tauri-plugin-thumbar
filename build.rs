const COMMANDS: &[&str] = &["initialize", "update", "is_supported"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
