# tauri-plugin-taskbar

Windows taskbar thumbnail toolbar controls for Tauri applications.

| Platform | Supported |
| --- | --- |
| Windows | ✓ full |
| Linux | ~ no-op fallback |
| macOS | ~ no-op fallback |
| Android | ~ no-op fallback |
| iOS | ~ no-op fallback |

## Install

This plugin targets Tauri v2 and Rust 1.77.2+.

Add the Rust crate to your `src-tauri/Cargo.toml`:

```toml
[dependencies]
tauri-plugin-taskbar = "0.1.0"
```

Add the JS bindings package:

```sh
pnpm add tauri-plugin-taskbar
# or npm add tauri-plugin-taskbar
# or yarn add tauri-plugin-taskbar
```

## Rust setup

Register the plugin in your Tauri builder:

```rust
fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_taskbar::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri app");
}
```

## Configuration

Configure the plugin in `tauri.conf.json` under `plugins.taskbar`:

```json
{
  "plugins": {
    "taskbar": {
      "autoAttach": true,
      "windowLabel": "main",
      "events": {
        "previous": "media-prev",
        "toggle": "media-toggle",
        "next": "media-next"
      },
      "tooltips": {
        "previous": "Previous",
        "play": "Play",
        "pause": "Pause",
        "next": "Next"
      },
      "icons": {
        "previous": "icons/taskbar/PREV_THUMB.ico",
        "previousDisabled": "icons/taskbar/PREV_THUMB_DISABLED.ico",
        "play": "icons/taskbar/PLAY_THUMB.ico",
        "pause": "icons/taskbar/PAUSE_THUMB.ico",
        "next": "icons/taskbar/NEXT_THUMB.ico",
        "nextDisabled": "icons/taskbar/NEXT_THUMB_DISABLED.ico"
      }
    }
  }
}
```

## JavaScript usage

```ts
import {
  initialize,
  isSupported,
  setNavigationEnabled,
  setPlaybackState
} from 'tauri-plugin-taskbar'

if (await isSupported()) {
  await initialize()
  await setPlaybackState(true)
  await setNavigationEnabled(true, true)
}
```

## Emitted events

When taskbar buttons are clicked, this plugin emits app events:

- `events.previous` (default: `media-prev`)
- `events.toggle` (default: `media-toggle`)
- `events.next` (default: `media-next`)

## Production notes

- This plugin does **not** set App User Model ID (AUMID). Keep AUMID setup in your app entrypoint.
- Ensure configured icon files are included as Tauri bundle resources.
- Non-Windows platforms intentionally succeed with no-op behavior for cross-platform safety.
