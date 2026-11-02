const COMMANDS: &[&str] = &[
    "initialize",
    "set_playback_state",
    "set_navigation_enabled",
    "is_supported",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
