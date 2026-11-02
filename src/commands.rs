use tauri::{command, Runtime, State, WebviewWindow};

use crate::{Button, ButtonUpdate, Thumbar};

#[command]
pub fn initialize<R: Runtime>(
    window: WebviewWindow<R>,
    buttons: Vec<Button>,
    state: State<'_, Thumbar<R>>,
) -> Result<(), String> {
    state
        .initialize(&window, buttons, None)
        .map_err(|error| error.to_string())
}

#[command]
pub fn update<R: Runtime>(
    window: WebviewWindow<R>,
    button: ButtonUpdate,
    state: State<'_, Thumbar<R>>,
) -> Result<(), String> {
    state
        .update(&window, button)
        .map_err(|error| error.to_string())
}

#[command]
pub fn is_supported() -> bool {
    cfg!(target_os = "windows")
}
