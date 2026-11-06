const COMMANDS: &[&str] = &[
    "initialize",
    "update",
    "set_cover",
    "set_progress",
    "is_supported",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
