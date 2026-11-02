use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub auto_attach: bool,
    pub window_label: String,
    pub icons: IconPaths,
    pub events: EventNames,
    pub tooltips: Tooltips,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auto_attach: true,
            window_label: "main".to_string(),
            icons: IconPaths::default(),
            events: EventNames::default(),
            tooltips: Tooltips::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct IconPaths {
    pub previous: String,
    pub previous_disabled: String,
    pub play: String,
    pub pause: String,
    pub next: String,
    pub next_disabled: String,
}

impl Default for IconPaths {
    fn default() -> Self {
        Self {
            previous: "icons/taskbar/PREV_THUMB.ico".to_string(),
            previous_disabled: "icons/taskbar/PREV_THUMB_DISABLED.ico".to_string(),
            play: "icons/taskbar/PLAY_THUMB.ico".to_string(),
            pause: "icons/taskbar/PAUSE_THUMB.ico".to_string(),
            next: "icons/taskbar/NEXT_THUMB.ico".to_string(),
            next_disabled: "icons/taskbar/NEXT_THUMB_DISABLED.ico".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct EventNames {
    pub previous: String,
    pub toggle: String,
    pub next: String,
}

impl Default for EventNames {
    fn default() -> Self {
        Self {
            previous: "media-prev".to_string(),
            toggle: "media-toggle".to_string(),
            next: "media-next".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Tooltips {
    pub previous: String,
    pub play: String,
    pub pause: String,
    pub next: String,
}

impl Default for Tooltips {
    fn default() -> Self {
        Self {
            previous: "Previous".to_string(),
            play: "Play".to_string(),
            pause: "Pause".to_string(),
            next: "Next".to_string(),
        }
    }
}
