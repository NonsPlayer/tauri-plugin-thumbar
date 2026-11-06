use tauri::{command, Runtime, State, WebviewWindow};

use crate::{Button, ButtonUpdate, Cover, Progress, Thumbar};

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
pub async fn set_cover<R: Runtime>(
    window: WebviewWindow<R>,
    cover: Option<Cover>,
    state: State<'_, Thumbar<R>>,
) -> Result<(), String> {
    state
        .set_cover(&window, cover)
        .map_err(|error| error.to_string())
}

#[command]
pub fn set_progress<R: Runtime>(
    window: WebviewWindow<R>,
    progress: Option<Progress>,
    state: State<'_, Thumbar<R>>,
) -> Result<(), String> {
    state
        .set_progress(&window, progress)
        .map_err(|error| error.to_string())
}

#[command]
pub fn is_supported() -> bool {
    cfg!(target_os = "windows")
}
