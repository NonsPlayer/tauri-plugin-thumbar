#[cfg(target_os = "windows")]
mod platform {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, Mutex};

    use tauri::{path::BaseDirectory, AppHandle, Emitter, Manager, Runtime, WebviewWindow};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
    use windows::Win32::UI::Shell::{
        DefSubclassProc, ITaskbarList3, RemoveWindowSubclass, SetWindowSubclass, TaskbarList,
        THBF_DISABLED, THBF_ENABLED, THBF_HIDDEN, THBN_CLICKED, THB_FLAGS, THB_ICON, THB_TOOLTIP,
        THUMBBUTTON, THUMBBUTTONFLAGS,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DestroyIcon, LoadImageW, PostMessageW, RegisterWindowMessageW, HICON, IMAGE_ICON,
        LR_DEFAULTSIZE, LR_LOADFROMFILE, WM_COMMAND, WM_NCDESTROY, WM_USER,
    };

    use crate::error::{Error, Result};
    use crate::models::{
        Button, ButtonHandler, ButtonState, ButtonUpdate, ClickEvent, WindowState,
    };

    const SUBCLASS_ID: usize = 4242;
    const WM_USER_UPDATE: u32 = WM_USER + 1;

    static TASKBAR_CREATED_MSG_ID: AtomicU32 = AtomicU32::new(0);

    type EventEmitter = Box<dyn Fn(&str, u32) + Send + Sync>;

    struct Context {
        hwnd: HWND,
        label: String,
        state: Arc<WindowState>,
        taskbar: Option<ITaskbarList3>,
        added: bool,
        icons: HashMap<PathBuf, HICON>,
        handler: Option<ButtonHandler>,
        emit: EventEmitter,
    }

    pub fn attach<R: Runtime>(
        window: &WebviewWindow<R>,
        buttons: Vec<Button>,
        handler: Option<ButtonHandler>,
    ) -> Result<Arc<WindowState>> {
        let hwnd = window
            .hwnd()
            .map_err(|error| Error::Message(format!("failed to get window handle: {error}")))?;
        let label = window.label().to_string();
        let app = window.app_handle().clone();

        let mut states = Vec::with_capacity(buttons.len());
        let mut icons = HashMap::with_capacity(buttons.len());
        for button in buttons {
            let path = resolve_icon(&app, &button.icon)?;
            if !icons.contains_key(&path) {
                let icon = unsafe { load_icon(&path)? };
                icons.insert(path.clone(), icon);
            }
            states.push(ButtonState::new(
                button,
                path.to_string_lossy().into_owned(),
            ));
        }

        let state = Arc::new(WindowState {
            hwnd: hwnd.0 as isize,
            buttons: Mutex::new(states),
        });

        let emit_label = label.clone();
        let emit: EventEmitter = Box::new(move |event: &str, id: u32| {
            let _ = app.emit(
                event,
                ClickEvent {
                    id,
                    window: emit_label.clone(),
                },
            );
        });

        let context = Box::new(Context {
            hwnd,
            label,
            state: state.clone(),
            taskbar: None,
            added: false,
            icons,
            handler,
            emit,
        });
        let context_ptr = Box::into_raw(context);

        unsafe {
            let _ = SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, context_ptr as usize);
            let message = taskbar_created_message_id();
            if message != 0 {
                let _ = PostMessageW(Some(hwnd), message, WPARAM(0), LPARAM(0));
            }
        }

        Ok(state)
    }

    pub fn update<R: Runtime>(
        window: &WebviewWindow<R>,
        state: &Arc<WindowState>,
        update: ButtonUpdate,
    ) -> Result<()> {
        let icon = match update.icon.as_deref() {
            Some(relative) => Some(resolve_icon(window.app_handle(), relative)?),
            None => None,
        };

        {
            let mut buttons = state
                .buttons
                .lock()
                .map_err(|_| Error::Message("thumbar button state is poisoned".into()))?;
            let button = buttons
                .iter_mut()
                .find(|button| button.id == update.id)
                .ok_or_else(|| Error::Message(format!("unknown button id {}", update.id)))?;

            if let Some(path) = icon {
                button.icon = path.to_string_lossy().into_owned();
            }
            if let Some(tooltip) = update.tooltip {
                button.tooltip = tooltip;
            }
            if let Some(enabled) = update.enabled {
                button.enabled = enabled;
            }
            if let Some(visible) = update.visible {
                button.visible = visible;
            }
        }

        unsafe {
            PostMessageW(
                Some(HWND(state.hwnd as _)),
                WM_USER_UPDATE,
                WPARAM(0),
                LPARAM(0),
            )
            .map_err(|error| Error::Message(format!("failed to post thumbar update: {error}")))?;
        }

        Ok(())
    }

    unsafe extern "system" fn subclass_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        u_id_subclass: usize,
        dw_ref_data: usize,
    ) -> LRESULT {
        let context = &mut *(dw_ref_data as *mut Context);

        let created_message = TASKBAR_CREATED_MSG_ID.load(Ordering::Relaxed);
        if created_message != 0 && msg == created_message {
            context.taskbar = None;
            context.added = false;

            if let Err(error) = initialize_taskbar(context) {
                log::error!("thumbar: failed to add buttons: {error}");
            }

            return DefSubclassProc(hwnd, msg, wparam, lparam);
        }

        if msg == WM_USER_UPDATE {
            if context.added {
                let buttons = build_buttons(context);
                if !buttons.is_empty() {
                    if let Some(taskbar) = &context.taskbar {
                        if let Err(error) = taskbar.ThumbBarUpdateButtons(context.hwnd, &buttons) {
                            log::error!("thumbar: failed to update buttons: {error}");
                        }
                    }
                }
            }

            return LRESULT(0);
        }

        if msg == WM_COMMAND {
            let high = (wparam.0 >> 16) & 0xFFFF;
            let id = (wparam.0 & 0xFFFF) as u32;

            if high as u32 == THBN_CLICKED {
                let click = context.state.buttons.lock().ok().and_then(|buttons| {
                    buttons
                        .iter()
                        .find(|button| button.id == id)
                        .map(|button| button.event.clone())
                });

                if let Some(event) = click {
                    if let Some(handler) = &context.handler {
                        handler(&context.label, id);
                    }
                    if let Some(event) = event {
                        (context.emit)(&event, id);
                    }

                    return LRESULT(0);
                }
            }
        }

        if msg == WM_NCDESTROY {
            let _ = RemoveWindowSubclass(hwnd, Some(subclass_proc), u_id_subclass);
            for icon in context.icons.values() {
                let _ = DestroyIcon(*icon);
            }
            context.icons.clear();
            let _ = Box::from_raw(context);
        }

        DefSubclassProc(hwnd, msg, wparam, lparam)
    }

    unsafe fn initialize_taskbar(context: &mut Context) -> Result<()> {
        let taskbar: ITaskbarList3 = CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| Error::Message(format!("CoCreateInstance(TaskbarList): {error}")))?;
        taskbar
            .HrInit()
            .map_err(|error| Error::Message(format!("ITaskbarList::HrInit: {error}")))?;

        let buttons = build_buttons(context);
        if buttons.is_empty() {
            return Err(Error::Message("no thumbar buttons could be created".into()));
        }

        taskbar
            .ThumbBarAddButtons(context.hwnd, &buttons)
            .map_err(|error| Error::Message(format!("ThumbBarAddButtons: {error}")))?;

        context.taskbar = Some(taskbar);
        context.added = true;
        Ok(())
    }

    unsafe fn build_buttons(context: &mut Context) -> Vec<THUMBBUTTON> {
        let buttons = match context.state.buttons.lock() {
            Ok(buttons) => buttons.clone(),
            Err(_) => {
                log::error!("thumbar: button state is poisoned");
                return Vec::new();
            }
        };

        buttons
            .iter()
            .filter_map(|button| {
                let path = Path::new(&button.icon);
                let icon = context.icon(path)?;
                Some(make_button(button, icon))
            })
            .collect()
    }

    impl Context {
        unsafe fn icon(&mut self, path: &Path) -> Option<HICON> {
            if let Some(icon) = self.icons.get(path) {
                return Some(*icon);
            }

            match load_icon(path) {
                Ok(icon) => {
                    self.icons.insert(path.to_path_buf(), icon);
                    Some(icon)
                }
                Err(error) => {
                    log::error!("thumbar: failed to load icon {}: {error}", path.display());
                    None
                }
            }
        }
    }

    fn make_button(button: &ButtonState, icon: HICON) -> THUMBBUTTON {
        let mut thumb = THUMBBUTTON {
            dwMask: THB_ICON | THB_TOOLTIP | THB_FLAGS,
            iId: button.id,
            hIcon: icon,
            dwFlags: button_flags(button),
            ..Default::default()
        };
        set_wide_string(&mut thumb.szTip, &button.tooltip);
        thumb
    }

    fn button_flags(button: &ButtonState) -> THUMBBUTTONFLAGS {
        let flags = if button.enabled {
            THBF_ENABLED
        } else {
            THBF_DISABLED
        };
        if button.visible {
            flags
        } else {
            flags | THBF_HIDDEN
        }
    }

    fn resolve_icon<R: Runtime>(app: &AppHandle<R>, relative: &str) -> Result<PathBuf> {
        if !relative.to_ascii_lowercase().ends_with(".ico") {
            return Err(Error::Message(format!(
                "icon '{relative}' must be an .ico resource path"
            )));
        }

        let path = app
            .path()
            .resolve(relative, BaseDirectory::Resource)
            .map_err(|error| {
                Error::Message(format!("failed to resolve icon '{relative}': {error}"))
            })?;
        if !path.is_file() {
            return Err(Error::Message(format!(
                "icon '{relative}' was not found as a bundled resource"
            )));
        }

        Ok(path)
    }

    unsafe fn load_icon(path: &Path) -> Result<HICON> {
        let wide = wide_string(&path.to_string_lossy());
        let icon = LoadImageW(
            None,
            PCWSTR(wide.as_ptr()),
            IMAGE_ICON,
            0,
            0,
            LR_LOADFROMFILE | LR_DEFAULTSIZE,
        )
        .map_err(|error| {
            Error::Message(format!("failed to load icon '{}': {error}", path.display()))
        })?;

        Ok(HICON(icon.0))
    }

    fn taskbar_created_message_id() -> u32 {
        let message = TASKBAR_CREATED_MSG_ID.load(Ordering::Relaxed);
        if message != 0 {
            return message;
        }

        let name = wide_string("TaskbarButtonCreated");
        let message = unsafe { RegisterWindowMessageW(PCWSTR(name.as_ptr())) };
        TASKBAR_CREATED_MSG_ID.store(message, Ordering::Relaxed);
        message
    }

    fn wide_string(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn set_wide_string(buf: &mut [u16; 260], value: &str) {
        let mut count = 0;
        for (index, code_unit) in value.encode_utf16().enumerate().take(259) {
            buf[index] = code_unit;
            count = index + 1;
        }

        if count < 260 {
            buf[count] = 0;
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use std::sync::{Arc, Mutex};

    use tauri::{Runtime, WebviewWindow};

    use crate::error::Result;
    use crate::models::{Button, ButtonHandler, ButtonState, ButtonUpdate, WindowState};

    pub fn attach<R: Runtime>(
        _window: &WebviewWindow<R>,
        buttons: Vec<Button>,
        _handler: Option<ButtonHandler>,
    ) -> Result<Arc<WindowState>> {
        Ok(Arc::new(WindowState {
            hwnd: 0,
            buttons: Mutex::new(buttons.into_iter().map(ButtonState::from).collect()),
        }))
    }

    pub fn update<R: Runtime>(
        _window: &WebviewWindow<R>,
        _state: &Arc<WindowState>,
        _update: ButtonUpdate,
    ) -> Result<()> {
        Ok(())
    }
}

pub use platform::{attach, update};
