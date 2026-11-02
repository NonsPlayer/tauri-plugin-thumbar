#[cfg(target_os = "windows")]
mod platform {
    use std::sync::atomic::{AtomicU32, Ordering};

    use tauri::{path::BaseDirectory, Emitter, Manager, Runtime, WebviewWindow};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
    use windows::Win32::UI::Shell::THBF_DISABLED;
    use windows::Win32::UI::Shell::{
        DefSubclassProc, ITaskbarList3, RemoveWindowSubclass, SetWindowSubclass, TaskbarList,
        THBF_ENABLED, THBN_CLICKED, THB_FLAGS, THB_ICON, THB_TOOLTIP, THUMBBUTTON,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DestroyIcon, HICON, IMAGE_ICON, LR_DEFAULTSIZE, LR_LOADFROMFILE, LoadImageW,
        PostMessageW, RegisterWindowMessageW, WM_COMMAND, WM_NCDESTROY, WM_USER,
    };

    use crate::error::{Error, Result};
    use crate::models::Config;

    const SUBCLASS_ID: usize = 4242;

    const BTN_PREV: u32 = 1001;
    const BTN_PLAY_PAUSE: u32 = 1002;
    const BTN_NEXT: u32 = 1003;

    const WM_USER_UPDATE_TASKBAR: u32 = WM_USER + 1;
    const WM_USER_UPDATE_NAV_BUTTONS: u32 = WM_USER + 2;

    static TASKBAR_CREATED_MSG_ID: AtomicU32 = AtomicU32::new(0);

    struct TaskbarContext {
        taskbar: Option<ITaskbarList3>,
        buttons_added: bool,
        emit_event: Box<dyn Fn(&str) + Send + Sync>,
        event_previous: String,
        event_toggle: String,
        event_next: String,
        tooltip_previous: String,
        tooltip_play: String,
        tooltip_pause: String,
        tooltip_next: String,
        icon_prev: HICON,
        icon_prev_disabled: HICON,
        icon_play: HICON,
        icon_pause: HICON,
        icon_next: HICON,
        icon_next_disabled: HICON,
    }

    pub fn attach<R: Runtime>(window: &WebviewWindow<R>, config: &Config) -> Result<()> {
        let hwnd = HWND(
            window
                .hwnd()
                .map_err(|e| Error::Message(format!("failed to get window handle: {e}")))?
                .0 as _,
        );

        if TASKBAR_CREATED_MSG_ID.load(Ordering::Relaxed) == 0 {
            let msg_name = wide_string("TaskbarButtonCreated");
            let msg_id = unsafe { RegisterWindowMessageW(PCWSTR(msg_name.as_ptr())) };
            TASKBAR_CREATED_MSG_ID.store(msg_id, Ordering::Relaxed);
        }

        let app = window.app_handle().clone();

        let icon_prev = load_hicon_from_resource(&app, &config.icons.previous)?;
        let icon_prev_disabled = load_hicon_from_resource(&app, &config.icons.previous_disabled)?;
        let icon_play = load_hicon_from_resource(&app, &config.icons.play)?;
        let icon_pause = load_hicon_from_resource(&app, &config.icons.pause)?;
        let icon_next = load_hicon_from_resource(&app, &config.icons.next)?;
        let icon_next_disabled = load_hicon_from_resource(&app, &config.icons.next_disabled)?;

        let emit_event: Box<dyn Fn(&str) + Send + Sync> = {
            let app = app.clone();
            Box::new(move |event_name| {
                let _ = app.emit(event_name, ());
            })
        };

        let context = Box::new(TaskbarContext {
            taskbar: None,
            buttons_added: false,
            emit_event,
            event_previous: config.events.previous.clone(),
            event_toggle: config.events.toggle.clone(),
            event_next: config.events.next.clone(),
            tooltip_previous: config.tooltips.previous.clone(),
            tooltip_play: config.tooltips.play.clone(),
            tooltip_pause: config.tooltips.pause.clone(),
            tooltip_next: config.tooltips.next.clone(),
            icon_prev,
            icon_prev_disabled,
            icon_play,
            icon_pause,
            icon_next,
            icon_next_disabled,
        });

        let context_ptr = Box::into_raw(context);

        unsafe {
            let _ = SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, context_ptr as usize);
            let msg_id = TASKBAR_CREATED_MSG_ID.load(Ordering::Relaxed);
            if msg_id != 0 {
                let _ = PostMessageW(Some(hwnd), msg_id, WPARAM(0), LPARAM(0));
            }
        }

        Ok(())
    }

    pub fn update_playback_state<R: Runtime>(
        window: &WebviewWindow<R>,
        is_playing: bool,
    ) -> Result<()> {
        let hwnd = HWND(
            window
                .hwnd()
                .map_err(|e| Error::Message(format!("failed to get window handle: {e}")))?
                .0 as _,
        );

        let wparam = WPARAM(if is_playing { 1 } else { 0 });
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_USER_UPDATE_TASKBAR, wparam, LPARAM(0));
        }

        Ok(())
    }

    pub fn update_navigation_enabled<R: Runtime>(
        window: &WebviewWindow<R>,
        previous_enabled: bool,
        next_enabled: bool,
    ) -> Result<()> {
        let hwnd = HWND(
            window
                .hwnd()
                .map_err(|e| Error::Message(format!("failed to get window handle: {e}")))?
                .0 as _,
        );

        let wparam_value =
            (if previous_enabled { 1 } else { 0 }) | ((if next_enabled { 1 } else { 0 }) << 8);
        let wparam = WPARAM(wparam_value);

        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_USER_UPDATE_NAV_BUTTONS, wparam, LPARAM(0));
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
        let context = &mut *(dw_ref_data as *mut TaskbarContext);

        if msg == TASKBAR_CREATED_MSG_ID.load(Ordering::Relaxed) {
            context.taskbar = None;
            context.buttons_added = false;

            match initialize_taskbar_interface(hwnd, context) {
                Ok(tb) => {
                    context.taskbar = Some(tb);
                    context.buttons_added = true;
                }
                Err(err) => {
                    log::error!("taskbar init failed: {err}");
                }
            }

            return DefSubclassProc(hwnd, msg, wparam, lparam);
        }

        if msg == WM_USER_UPDATE_TASKBAR {
            if let Some(ref taskbar) = context.taskbar {
                if context.buttons_added {
                    let is_playing = wparam.0 != 0;
                    let h_icon = if is_playing {
                        context.icon_pause
                    } else {
                        context.icon_play
                    };

                    let mut btn = THUMBBUTTON {
                        dwMask: THB_ICON | THB_TOOLTIP,
                        iId: BTN_PLAY_PAUSE,
                        hIcon: h_icon,
                        szTip: [0; 260],
                        ..Default::default()
                    };

                    let tip = if is_playing {
                        context.tooltip_pause.as_str()
                    } else {
                        context.tooltip_play.as_str()
                    };
                    set_wide_string(&mut btn.szTip, tip);

                    let _ = taskbar.ThumbBarUpdateButtons(hwnd, &[btn]);
                }
            }

            return LRESULT(0);
        }

        if msg == WM_USER_UPDATE_NAV_BUTTONS {
            if let Some(ref taskbar) = context.taskbar {
                if context.buttons_added {
                    let prev_enabled = (wparam.0 & 0xFF) != 0;
                    let next_enabled = ((wparam.0 >> 8) & 0xFF) != 0;

                    let (prev_flags, prev_icon) = if prev_enabled {
                        (THBF_ENABLED, context.icon_prev)
                    } else {
                        (THBF_DISABLED, context.icon_prev_disabled)
                    };
                    let (next_flags, next_icon) = if next_enabled {
                        (THBF_ENABLED, context.icon_next)
                    } else {
                        (THBF_DISABLED, context.icon_next_disabled)
                    };

                    let buttons = [
                        THUMBBUTTON {
                            dwMask: THB_FLAGS | THB_ICON,
                            iId: BTN_PREV,
                            hIcon: prev_icon,
                            dwFlags: prev_flags,
                            ..Default::default()
                        },
                        THUMBBUTTON {
                            dwMask: THB_FLAGS | THB_ICON,
                            iId: BTN_NEXT,
                            hIcon: next_icon,
                            dwFlags: next_flags,
                            ..Default::default()
                        },
                    ];

                    let _ = taskbar.ThumbBarUpdateButtons(hwnd, &buttons);
                }
            }

            return LRESULT(0);
        }

        if msg == WM_COMMAND {
            let high = (wparam.0 >> 16) & 0xFFFF;
            let low = wparam.0 & 0xFFFF;

            if high as u32 == THBN_CLICKED {
                let event_name = match low as u32 {
                    BTN_PREV => Some(context.event_previous.as_str()),
                    BTN_PLAY_PAUSE => Some(context.event_toggle.as_str()),
                    BTN_NEXT => Some(context.event_next.as_str()),
                    _ => None,
                };

                if let Some(event_name) = event_name {
                    (context.emit_event)(event_name);
                }

                return LRESULT(0);
            }
        }

        if msg == WM_NCDESTROY {
            let _ = RemoveWindowSubclass(hwnd, Some(subclass_proc), u_id_subclass);
            let _ = DestroyIcon(context.icon_prev);
            let _ = DestroyIcon(context.icon_prev_disabled);
            let _ = DestroyIcon(context.icon_play);
            let _ = DestroyIcon(context.icon_pause);
            let _ = DestroyIcon(context.icon_next);
            let _ = DestroyIcon(context.icon_next_disabled);
            let _ = Box::from_raw(context);
        }

        DefSubclassProc(hwnd, msg, wparam, lparam)
    }

    unsafe fn initialize_taskbar_interface(
        hwnd: HWND,
        context: &TaskbarContext,
    ) -> windows::core::Result<ITaskbarList3> {
        let taskbar: ITaskbarList3 = CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER)?;
        taskbar.HrInit()?;

        let mut buttons = [
            THUMBBUTTON {
                dwMask: THB_ICON | THB_TOOLTIP | THB_FLAGS,
                iId: BTN_PREV,
                hIcon: context.icon_prev,
                szTip: [0; 260],
                dwFlags: THBF_ENABLED,
                ..Default::default()
            },
            THUMBBUTTON {
                dwMask: THB_ICON | THB_TOOLTIP | THB_FLAGS,
                iId: BTN_PLAY_PAUSE,
                hIcon: context.icon_play,
                szTip: [0; 260],
                dwFlags: THBF_ENABLED,
                ..Default::default()
            },
            THUMBBUTTON {
                dwMask: THB_ICON | THB_TOOLTIP | THB_FLAGS,
                iId: BTN_NEXT,
                hIcon: context.icon_next,
                szTip: [0; 260],
                dwFlags: THBF_ENABLED,
                ..Default::default()
            },
        ];

        set_wide_string(&mut buttons[0].szTip, &context.tooltip_previous);
        set_wide_string(&mut buttons[1].szTip, &context.tooltip_play);
        set_wide_string(&mut buttons[2].szTip, &context.tooltip_next);

        taskbar.ThumbBarAddButtons(hwnd, &buttons)?;
        Ok(taskbar)
    }

    fn load_hicon_from_resource<R: Runtime>(app: &tauri::AppHandle<R>, rel: &str) -> Result<HICON> {
        let path = app
            .path()
            .resolve(rel, BaseDirectory::Resource)
            .map_err(|e| Error::Message(format!("failed to resolve resource path '{rel}': {e}")))?;

        let wpath = wide_string(&path.to_string_lossy());

        unsafe {
            let h = LoadImageW(
                None,
                PCWSTR(wpath.as_ptr()),
                IMAGE_ICON,
                0,
                0,
                LR_LOADFROMFILE | LR_DEFAULTSIZE,
            )?;
            Ok(HICON(h.0))
        }
    }

    fn wide_string(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn set_wide_string(buf: &mut [u16; 260], s: &str) {
        let mut count = 0;
        for (idx, code_unit) in s.encode_utf16().enumerate().take(259) {
            buf[idx] = code_unit;
            count = idx + 1;
        }

        if count < 260 {
            buf[count] = 0;
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use tauri::{Runtime, WebviewWindow};

    use crate::{error::Result, models::Config};

    pub fn attach<R: Runtime>(_window: &WebviewWindow<R>, _config: &Config) -> Result<()> {
        Ok(())
    }

    pub fn update_playback_state<R: Runtime>(
        _window: &WebviewWindow<R>,
        _is_playing: bool,
    ) -> Result<()> {
        Ok(())
    }

    pub fn update_navigation_enabled<R: Runtime>(
        _window: &WebviewWindow<R>,
        _previous_enabled: bool,
        _next_enabled: bool,
    ) -> Result<()> {
        Ok(())
    }
}

pub use platform::{attach, update_navigation_enabled, update_playback_state};
