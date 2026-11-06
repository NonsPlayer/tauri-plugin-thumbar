# tauri-plugin-thumbar

Runtime-defined Windows taskbar controls for Tauri applications: thumbnail
toolbar buttons, hover thumbnail covers and the taskbar button progress bar.

The button list is supplied at runtime. Windows only lets an application add
buttons to a window's thumbnail toolbar once, so `initialize` takes the complete
list (up to 7 buttons) and `update` changes icon, tooltip, enabled and visible
state afterwards. `set_cover` replaces the taskbar hover thumbnail with a static
cover image.

## Install

This plugin targets Tauri v2 and Rust 1.88+.

Add the Rust crate to your `src-tauri/Cargo.toml`:

```toml
[dependencies]
tauri-plugin-thumbar = "0.1.1"
```

Add the JS bindings package:

```sh
pnpm add tauri-plugin-thumbar
```

## Rust setup

Register the plugin in your Tauri builder:

```rust
fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_thumbar::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri app");
}
```

Initialize the toolbar from the app setup hook:

```rust
use tauri::Manager;
use tauri_plugin_thumbar::{Button, ThumbarExt};

tauri::Builder::default()
    .plugin(tauri_plugin_thumbar::init())
    .setup(|app| {
        let window = app.get_webview_window("main").unwrap();

        app.thumbar().initialize(
            &window,
            vec![
                Button::new(1, "icons/thumbar/previous.ico")
                    .tooltip("Previous")
                    .event("thumbar-previous"),
                Button::new(2, "icons/thumbar/play.ico")
                    .tooltip("Play")
                    .event("thumbar-toggle"),
                Button::new(3, "icons/thumbar/next.ico")
                    .tooltip("Next")
                    .event("thumbar-next"),
            ],
            Some(Box::new(|window, id| {
                // Runs on the window's UI thread. Do not block; forward to a
                // channel or another thread instead.
                let _ = (window, id);
            })),
        )?;

        Ok(())
    })
```

Update a button later:

```rust
use tauri_plugin_thumbar::ButtonUpdate;

app.thumbar().update(
    &window,
    ButtonUpdate::new(2)
        .icon("icons/thumbar/pause.ico")
        .tooltip("Pause"),
)?;

app.thumbar().update(&window, ButtonUpdate::new(3).enabled(false))?;
```

## Buttons

- At most **7** buttons are supported by the Windows thumbnail toolbar.
- `id` must be unique within the window and in `1..=65535`.
- `icon` is a Tauri resource path to an `.ico` file. Bundle the file through
  `tauri.conf.json` (`bundle.resources`).
- `enabled` and `visible` default to `true`.
- `event` is optional. When set, the plugin emits that app event on click with
  `{ id, window }` as the payload.

Buttons can only be added once per window. Use `update` to change the icon,
tooltip, enabled or visible state; use `visible: false` to hide a button instead
of removing it.

## Cover

`set_cover` makes DWM render a static image in the taskbar hover thumbnail
instead of a live preview of the window. `clear_cover` restores the live
thumbnail.

The cover can come from an absolute local path or from encoded image bytes:

```rust
use tauri_plugin_thumbar::{Cover, ThumbarExt};

app.thumbar().set_cover(&window, Some(Cover::from_path("/path/to/cover.jpg")))?;
app.thumbar().set_cover(&window, Some(Cover::from_bytes(cover_bytes)))?;
app.thumbar().set_cover(&window, None)?;
```

From JavaScript:

```ts
import { clearCover, setCoverFromBytes, setCoverFromPath } from 'tauri-plugin-thumbar'

await setCoverFromPath('C:\\cache\\cover.jpg')
await setCoverFromBytes(new Uint8Array(await response.arrayBuffer()))
await clearCover()
```

- Call `initialize` before `set_cover`; the cover belongs to an initialized
  window.
- PNG, JPEG, WebP and GIF are supported.
- The encoded image must not exceed **16 MiB**.
- Paths must be absolute; relative paths are rejected.
- The cover stays attached until `clear_cover` or the window is destroyed.
- The image is scaled to fit and letterboxed on black. It is rendered when DWM
  requests the thumbnail, for both the iconic thumbnail and the peek preview.
- Setting `DWMWA_FORCE_ICONIC_REPRESENTATION` is required for this behavior.
  It forces the iconic representation for the window's taskbar thumbnail and
  peek preview; Windows versions that show a full-size peek preview from the
  Alt+Tab switcher may therefore use the cover there as well.

## Progress

`set_progress` drives the progress bar on the taskbar button. `percent` is
clamped to `0..=100` and ignored for `none` and `indeterminate`. Pass `None`
from Rust or `null` from JavaScript to hide the bar.

```rust
use tauri_plugin_thumbar::{Progress, ThumbarExt};

app.thumbar().set_progress(&window, Some(Progress::normal(42)))?;
app.thumbar().set_progress(&window, Some(Progress::indeterminate()))?;
app.thumbar().set_progress(&window, None)?;
```

- `normal`: determinate progress.
- `indeterminate`: activity without a known percentage.
- `paused`: determinate progress shown as paused.
- `error`: determinate progress shown as failed.
- `none`: same as passing `None`.
- The progress state is restored when Explorer recreates the taskbar button.

## JavaScript usage

Add the plugin's default permission to your capability (for example
`src-tauri/capabilities/default.json`):

```json
{
  "permissions": ["thumbar:default"]
}
```

```ts
import { initialize, isSupported, setProgress, update } from 'tauri-plugin-thumbar'
import { listen } from '@tauri-apps/api/event'

if (await isSupported()) {
  await initialize([
    { id: 1, icon: 'icons/thumbar/previous.ico', tooltip: 'Previous', event: 'thumbar-previous' },
    { id: 2, icon: 'icons/thumbar/play.ico', tooltip: 'Play', event: 'thumbar-toggle' },
    { id: 3, icon: 'icons/thumbar/next.ico', tooltip: 'Next', event: 'thumbar-next' }
  ])

  await update({ id: 2, icon: 'icons/thumbar/pause.ico', tooltip: 'Pause' })
  await setProgress({ state: 'normal', percent: 42 })
  await setProgress(null)

  const unlisten = await listen<{ id: number; window: string }>('thumbar-toggle', (event) => {
    console.log(event.payload.id, event.payload.window)
  })
}
```

## Production notes

- This plugin does **not** set App User Model ID (AUMID). Set it once in your
  app entrypoint if you need a stable shell identity for notifications, jump
  lists or taskbar pinning.
- Ensure configured icon files are included as Tauri bundle resources.
- Non-Windows platforms intentionally succeed with no-op behavior for
  cross-platform safety.
- The Rust click handler runs inside the window procedure on the UI thread.
  Keep it non-blocking.
- If Explorer restarts, the plugin re-adds the current button state when the
  taskbar button is recreated.
