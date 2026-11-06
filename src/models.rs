use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Maximum number of buttons supported by the Windows thumbnail toolbar.
pub const MAX_BUTTONS: usize = 7;

/// Maximum number of decoded bytes accepted for a cover image.
pub const MAX_COVER_BYTES: usize = 16 * 1024 * 1024;

/// Button identifiers must fit in the low word of `WM_COMMAND`.
const MAX_BUTTON_ID: u32 = u16::MAX as u32;

const fn default_true() -> bool {
    true
}

/// A thumbnail toolbar button provided when the toolbar is initialized.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Button {
    /// Unique identifier within the window, in `1..=65535`.
    pub id: u32,
    /// Tauri resource path to an `.ico` file.
    pub icon: String,
    #[serde(default)]
    pub tooltip: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub visible: bool,
    /// Optional app event emitted when the button is clicked.
    #[serde(default)]
    pub event: Option<String>,
}

impl Button {
    pub fn new(id: u32, icon: impl Into<String>) -> Self {
        Self {
            id,
            icon: icon.into(),
            tooltip: String::new(),
            enabled: true,
            visible: true,
            event: None,
        }
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = tooltip.into();
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn event(mut self, event: impl Into<String>) -> Self {
        self.event = Some(event.into());
        self
    }
}

/// A partial update for a button that is already part of the toolbar.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonUpdate {
    pub id: u32,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub tooltip: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub visible: Option<bool>,
}

impl ButtonUpdate {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }
}

/// A cover image shown in the taskbar thumbnail.
///
/// Exactly one of `path` or `data` must be set. `path` is an absolute path to
/// a local image file. `data` is a base64-encoded image. PNG, JPEG, WebP and
/// GIF are supported.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cover {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub data: Option<String>,
}

impl Cover {
    /// Creates a cover from an absolute local file path.
    pub fn from_path(path: impl Into<String>) -> Self {
        Self {
            path: Some(path.into()),
            data: None,
        }
    }

    /// Creates a cover from encoded image bytes.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            path: None,
            data: Some(base64::engine::general_purpose::STANDARD.encode(bytes)),
        }
    }

    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub(crate) fn source(&self) -> Result<CoverSource> {
        match (self.path.as_deref(), self.data.as_deref()) {
            (Some(_), Some(_)) => Err(Error::Message(
                "cover must provide either a path or data, not both".into(),
            )),
            (None, None) => Err(Error::Message("cover must provide a path or data".into())),
            (Some(path), None) => {
                if path.trim().is_empty() {
                    return Err(Error::Message("cover path must not be empty".into()));
                }
                Ok(CoverSource::Path(path.to_owned()))
            }
            (None, Some(data)) => {
                if data.trim().is_empty() {
                    return Err(Error::Message("cover data must not be empty".into()));
                }
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(data)
                    .map_err(|error| {
                        Error::Message(format!("cover data is not valid base64: {error}"))
                    })?;
                if bytes.len() > MAX_COVER_BYTES {
                    return Err(Error::Message(format!(
                        "cover data must not exceed {MAX_COVER_BYTES} bytes"
                    )));
                }
                Ok(CoverSource::Bytes(bytes))
            }
        }
    }
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) enum CoverSource {
    Path(String),
    Bytes(Vec<u8>),
}

/// Decoded cover pixels kept for DWM thumbnail requests.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) struct CoverImage {
    pub width: u32,
    pub height: u32,
    /// Straight (non-premultiplied) RGBA8 pixels, top-down.
    pub rgba: Vec<u8>,
}

/// Payload of the optional app event emitted for a button click.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClickEvent {
    pub id: u32,
    pub window: String,
}

/// Called on the window's UI thread when a button is clicked.
///
/// The handler must not block: it runs inside the window procedure, so long
/// work should be forwarded to another thread or channel.
pub type ButtonHandler = Box<dyn Fn(&str, u32) + Send + Sync + 'static>;

/// Taskbar progress bar state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProgressState {
    /// Hides the progress bar.
    #[default]
    None,
    Normal,
    Indeterminate,
    Paused,
    Error,
}

/// Taskbar progress bar update.
///
/// `percent` is clamped to `0..=100` and ignored for `None` and
/// `Indeterminate`. Passing `None` to [`crate::Thumbar::set_progress`] hides
/// the progress bar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    #[serde(default)]
    pub state: ProgressState,
    #[serde(default)]
    pub percent: Option<u8>,
}

impl Progress {
    pub fn new(state: ProgressState, percent: Option<u8>) -> Self {
        Self {
            state,
            percent: percent.map(|percent| percent.min(100)),
        }
    }

    pub fn normal(percent: u8) -> Self {
        Self::new(ProgressState::Normal, Some(percent))
    }

    pub fn indeterminate() -> Self {
        Self::new(ProgressState::Indeterminate, None)
    }

    pub fn paused(percent: u8) -> Self {
        Self::new(ProgressState::Paused, Some(percent))
    }

    pub fn error(percent: u8) -> Self {
        Self::new(ProgressState::Error, Some(percent))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ButtonState {
    pub id: u32,
    pub icon: String,
    pub tooltip: String,
    pub enabled: bool,
    pub visible: bool,
    pub event: Option<String>,
}

impl ButtonState {
    pub(crate) fn new(button: Button, icon: String) -> Self {
        Self {
            id: button.id,
            icon,
            tooltip: button.tooltip,
            enabled: button.enabled,
            visible: button.visible,
            event: button.event,
        }
    }
}

impl From<Button> for ButtonState {
    fn from(button: Button) -> Self {
        let icon = button.icon.clone();
        Self::new(button, icon)
    }
}

pub(crate) struct WindowState {
    pub hwnd: isize,
    pub buttons: Mutex<Vec<ButtonState>>,
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub cover: Mutex<Option<Arc<CoverImage>>>,
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub progress: Mutex<Progress>,
}

pub(crate) fn validate_buttons(buttons: &[Button]) -> Result<()> {
    if buttons.is_empty() {
        return Err(Error::Message("at least one button is required".into()));
    }
    if buttons.len() > MAX_BUTTONS {
        return Err(Error::Message(format!(
            "at most {MAX_BUTTONS} buttons are supported"
        )));
    }

    let mut ids = HashSet::with_capacity(buttons.len());
    for button in buttons {
        if button.id == 0 || button.id > MAX_BUTTON_ID {
            return Err(Error::Message(format!(
                "button id {} must be in 1..={MAX_BUTTON_ID}",
                button.id
            )));
        }
        if !ids.insert(button.id) {
            return Err(Error::Message(format!("duplicate button id {}", button.id)));
        }
        if !button.icon.to_ascii_lowercase().ends_with(".ico") {
            return Err(Error::Message(format!(
                "button {} icon must be an .ico resource path",
                button.id
            )));
        }
        if button
            .event
            .as_ref()
            .is_some_and(|event| event.trim().is_empty())
        {
            return Err(Error::Message(format!(
                "button {} event name must not be empty",
                button.id
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buttons(count: usize) -> Vec<Button> {
        (1..=count)
            .map(|id| Button::new(id as u32, format!("icons/{id}.ico")))
            .collect()
    }

    #[test]
    fn accepts_valid_buttons() {
        assert!(validate_buttons(&buttons(7)).is_ok());
    }

    #[test]
    fn rejects_empty_button_list() {
        assert!(validate_buttons(&[]).is_err());
    }

    #[test]
    fn rejects_more_than_seven_buttons() {
        assert!(validate_buttons(&buttons(8)).is_err());
    }

    #[test]
    fn rejects_duplicate_ids() {
        let list = vec![Button::new(1, "icons/a.ico"), Button::new(1, "icons/b.ico")];
        assert!(validate_buttons(&list).is_err());
    }

    #[test]
    fn rejects_zero_and_overflow_ids() {
        assert!(validate_buttons(&[Button::new(0, "icons/a.ico")]).is_err());
        assert!(validate_buttons(&[Button::new(65536, "icons/a.ico")]).is_err());
        assert!(validate_buttons(&[Button::new(65535, "icons/a.ico")]).is_ok());
    }

    #[test]
    fn rejects_non_ico_icons() {
        assert!(validate_buttons(&[Button::new(1, "icons/a.png")]).is_err());
        assert!(validate_buttons(&[Button::new(1, "icons/a.ICO")]).is_ok());
    }

    #[test]
    fn rejects_empty_event_names() {
        let button = Button::new(1, "icons/a.ico").event("  ");
        assert!(validate_buttons(&[button]).is_err());
    }

    #[test]
    fn cover_accepts_a_single_source() {
        assert!(matches!(
            Cover::from_path("C:\\covers\\a.png").source(),
            Ok(CoverSource::Path(_))
        ));
        assert!(matches!(
            Cover::from_bytes(b"image bytes").source(),
            Ok(CoverSource::Bytes(_))
        ));
    }

    #[test]
    fn cover_rejects_missing_or_duplicate_sources() {
        assert!(Cover {
            path: None,
            data: None
        }
        .source()
        .is_err());
        assert!(Cover {
            path: Some("C:\\covers\\a.png".into()),
            data: Some("aW1hZ2U=".into()),
        }
        .source()
        .is_err());
    }

    #[test]
    fn cover_rejects_blank_values_and_invalid_base64() {
        assert!(Cover::from_path("   ").source().is_err());
        assert!(Cover {
            path: None,
            data: Some("not base64!".into()),
        }
        .source()
        .is_err());
    }

    #[test]
    fn cover_from_bytes_round_trips_payload() {
        let bytes = b"cover payload";
        match Cover::from_bytes(bytes).source() {
            Ok(CoverSource::Bytes(decoded)) => assert_eq!(decoded, bytes),
            _ => panic!("expected decoded cover bytes"),
        }
    }

    #[test]
    fn progress_clamps_percent() {
        assert_eq!(Progress::normal(140).percent, Some(100));
        assert_eq!(Progress::paused(42).percent, Some(42));
        assert_eq!(Progress::indeterminate().percent, None);
    }
}
