//! Windows taskbar thumbnail toolbar controls for Tauri applications.

mod commands;
mod desktop;
mod error;
mod models;

use std::{collections::HashSet, sync::Mutex};

use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, RunEvent, Runtime, WebviewWindow, WindowEvent,
};

pub use error::{Error, Result};
pub use models::{Config, EventNames, IconPaths, Tooltips};

/// Taskbar plugin state and high-level Rust API.
pub struct Taskbar<R: Runtime> {
    // keep runtime generic without introducing extra trait bounds on R
    _marker: std::marker::PhantomData<fn() -> R>,
    config: Config,
    attached_windows: Mutex<HashSet<String>>,
}

impl<R: Runtime> Taskbar<R> {
    fn new(config: Config) -> Self {
        Self {
            _marker: std::marker::PhantomData::<fn() -> R>,
            config,
            attached_windows: Mutex::new(HashSet::new()),
        }
    }

    /// Returns the active plugin configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    fn ensure_attached_slot(&self, window_label: &str) -> bool {
        self.attached_windows
            .lock()
            .expect("taskbar plugin state mutex poisoned")
            .insert(window_label.to_string())
    }

    fn rollback_attached_slot(&self, window_label: &str) {
        self.attached_windows
            .lock()
            .expect("taskbar plugin state mutex poisoned")
            .remove(window_label);
    }

    fn remove_attached_slot(&self, window_label: &str) {
        self.attached_windows
            .lock()
            .expect("taskbar plugin state mutex poisoned")
            .remove(window_label);
    }

    fn ensure_attached(&self, window: &WebviewWindow<R>) -> Result<()> {
        let label = window.label().to_string();

        if !self.ensure_attached_slot(&label) {
            return Ok(());
        }

        if let Err(err) = desktop::attach(window, &self.config) {
            self.rollback_attached_slot(&label);
            return Err(err);
        }

        Ok(())
    }

    /// Explicitly initializes taskbar thumbnail controls for the provided webview window.
    pub fn initialize(&self, window: &WebviewWindow<R>) -> Result<()> {
        self.ensure_attached(window)
    }

    /// Updates play/pause visual state on taskbar thumbnail buttons.
    pub fn set_playback_state(&self, window: &WebviewWindow<R>, is_playing: bool) -> Result<()> {
        self.ensure_attached(window)?;
        desktop::update_playback_state(window, is_playing)
    }

    /// Updates previous/next enabled state on taskbar thumbnail buttons.
    pub fn set_navigation_enabled(
        &self,
        window: &WebviewWindow<R>,
        previous_enabled: bool,
        next_enabled: bool,
    ) -> Result<()> {
        self.ensure_attached(window)?;
        desktop::update_navigation_enabled(window, previous_enabled, next_enabled)
    }

    /// Returns whether native taskbar thumbnail behavior is supported.
    pub const fn is_supported() -> bool {
        cfg!(target_os = "windows")
    }
}

/// Extensions to access the taskbar plugin APIs from app-level managers.
pub trait TaskbarExt<R: Runtime> {
    fn taskbar(&self) -> &Taskbar<R>;
}

impl<R: Runtime, T: Manager<R>> TaskbarExt<R> for T {
    fn taskbar(&self) -> &Taskbar<R> {
        self.state::<Taskbar<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R, Option<Config>> {
    Builder::<R, Option<Config>>::new("taskbar")
        .setup(|app, api| {
            let config = api.config().clone().unwrap_or_default();
            let taskbar = Taskbar::<R>::new(config.clone());

            if config.auto_attach {
                if let Some(window) = app.get_webview_window(&config.window_label) {
                    if let Err(err) = taskbar.initialize(&window) {
                        log::error!("failed to auto-attach taskbar plugin in setup: {err}");
                    }
                }
            }

            app.manage(taskbar);

            Ok(())
        })
        .on_event(|app, event| {
            if let RunEvent::WindowEvent {
                label,
                event: WindowEvent::Destroyed,
                ..
            } = event
            {
                if let Some(taskbar) = app.try_state::<Taskbar<R>>() {
                    taskbar.remove_attached_slot(label);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::initialize,
            commands::set_playback_state,
            commands::set_navigation_enabled,
            commands::is_supported,
        ])
        .build()
}
