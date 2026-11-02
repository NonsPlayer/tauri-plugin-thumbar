//! Runtime-defined Windows taskbar thumbnail toolbar controls for Tauri applications.
//!
//! Call [`Thumbar::initialize`] once per window with the complete button list.
//! Windows only allows the button set to be added once per window; later changes
//! go through [`Thumbar::update`], which can update icons, tooltips, enabled and
//! visible state without re-adding buttons.
//!
//! [`Thumbar::set_cover`] replaces the taskbar hover thumbnail with a static
//! cover image. Passing `None` restores the live window thumbnail.

mod commands;
mod desktop;
mod error;
mod models;

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, RunEvent, Runtime, WebviewWindow, WindowEvent,
};

pub use error::{Error, Result};
pub use models::{
    Button, ButtonHandler, ButtonUpdate, ClickEvent, Cover, MAX_BUTTONS, MAX_COVER_BYTES,
};

use models::WindowState;

/// Plugin state and the high-level Rust API.
pub struct Thumbar<R: Runtime> {
    // Keep the runtime generic without adding extra trait bounds on R.
    _marker: PhantomData<fn() -> R>,
    windows: Mutex<HashMap<String, Arc<WindowState>>>,
}

impl<R: Runtime> Thumbar<R> {
    fn new() -> Self {
        Self {
            _marker: PhantomData,
            windows: Mutex::new(HashMap::new()),
        }
    }

    /// Initializes the thumbnail toolbar for a window.
    ///
    /// The complete button list must be provided here. Windows only supports
    /// up to [`MAX_BUTTONS`] buttons, and the list cannot be extended later.
    /// Use [`Thumbar::update`] to change a button's icon, tooltip, enabled or
    /// visible state.
    ///
    /// `handler` runs on the window's UI thread and must not block. It receives
    /// the window label and the button id.
    pub fn initialize(
        &self,
        window: &WebviewWindow<R>,
        buttons: Vec<Button>,
        handler: Option<ButtonHandler>,
    ) -> Result<()> {
        models::validate_buttons(&buttons)?;

        let label = window.label().to_string();
        let mut windows = self
            .windows
            .lock()
            .map_err(|_| Error::Message("thumbar registry is poisoned".into()))?;

        if windows.contains_key(&label) {
            return Err(Error::Message(format!(
                "thumbar is already initialized for window '{label}'"
            )));
        }

        let state = desktop::attach(window, buttons, handler)?;
        windows.insert(label, state);
        Ok(())
    }

    /// Applies a partial update to an existing button.
    pub fn update(&self, window: &WebviewWindow<R>, update: ButtonUpdate) -> Result<()> {
        let state = self.window_state(window)?;
        desktop::update(window, &state, update)
    }

    /// Sets or clears the taskbar hover thumbnail cover.
    ///
    /// The cover replaces the live window thumbnail with a static image. Pass
    /// `None` to restore the default thumbnail. The cover is only used by the
    /// Windows taskbar; other platforms accept the call as a no-op.
    pub fn set_cover(&self, window: &WebviewWindow<R>, cover: Option<Cover>) -> Result<()> {
        let state = self.window_state(window)?;
        desktop::set_cover(window, &state, cover)
    }

    fn window_state(&self, window: &WebviewWindow<R>) -> Result<Arc<WindowState>> {
        let label = window.label();
        let windows = self
            .windows
            .lock()
            .map_err(|_| Error::Message("thumbar registry is poisoned".into()))?;
        windows.get(label).cloned().ok_or_else(|| {
            Error::Message(format!("thumbar is not initialized for window '{label}'"))
        })
    }

    /// Returns whether native thumbnail toolbar behavior is supported.
    pub const fn is_supported() -> bool {
        cfg!(target_os = "windows")
    }

    fn remove(&self, label: &str) {
        if let Ok(mut windows) = self.windows.lock() {
            windows.remove(label);
        }
    }
}

/// Extensions to access the thumbar plugin APIs from app-level managers.
pub trait ThumbarExt<R: Runtime> {
    fn thumbar(&self) -> &Thumbar<R>;
}

impl<R: Runtime, T: Manager<R>> ThumbarExt<R> for T {
    fn thumbar(&self) -> &Thumbar<R> {
        self.state::<Thumbar<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R, ()>::new("thumbar")
        .setup(|app, _api| {
            app.manage(Thumbar::<R>::new());
            Ok(())
        })
        .on_event(|app, event| {
            if let RunEvent::WindowEvent {
                label,
                event: WindowEvent::Destroyed,
                ..
            } = event
            {
                if let Some(thumbar) = app.try_state::<Thumbar<R>>() {
                    thumbar.remove(label);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::initialize,
            commands::update,
            commands::set_cover,
            commands::is_supported,
        ])
        .build()
}
