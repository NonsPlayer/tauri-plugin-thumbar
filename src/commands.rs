use tauri::{command, Runtime, State, WebviewWindow};

use crate::Taskbar;

#[command]
pub fn initialize<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Taskbar<R>>,
) -> Result<(), String> {
    state.initialize(&window).map_err(|e| e.to_string())
}

#[command]
pub fn set_playback_state<R: Runtime>(
    window: WebviewWindow<R>,
    is_playing: bool,
    state: State<'_, Taskbar<R>>,
) -> Result<(), String> {
    state
        .set_playback_state(&window, is_playing)
        .map_err(|e| e.to_string())
}

#[command]
pub fn set_navigation_enabled<R: Runtime>(
    window: WebviewWindow<R>,
    previous_enabled: bool,
    next_enabled: bool,
    state: State<'_, Taskbar<R>>,
) -> Result<(), String> {
    state
        .set_navigation_enabled(&window, previous_enabled, next_enabled)
        .map_err(|e| e.to_string())
}

#[command]
pub fn is_supported() -> bool {
    cfg!(target_os = "windows")
}
