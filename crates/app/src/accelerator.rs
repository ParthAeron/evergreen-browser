//! Win32 keyboard accelerator hook for WebView2.
//! Intercepts browser shortcuts (Ctrl+T, Ctrl+W, Ctrl+L, F12, etc.) at the COM layer
//! before child webview HWNDs consume them.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use winit::event_loop::EventLoopProxy;
use wry::WebView;

#[cfg(target_os = "windows")]
pub struct ActivePermission {
    pub args:
        webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2PermissionRequestedEventArgs,
    pub deferral: webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Deferral,
}

#[cfg(target_os = "windows")]
unsafe impl Send for ActivePermission {}
#[cfg(target_os = "windows")]
unsafe impl Sync for ActivePermission {}

#[cfg(target_os = "windows")]
static NEXT_PERMISSION_ID: AtomicU64 = AtomicU64::new(1);

#[cfg(target_os = "windows")]
static PERMISSION_STORE: Mutex<Option<HashMap<u64, ActivePermission>>> = Mutex::new(None);

#[cfg(target_os = "windows")]
pub fn resolve_permission(permission_id: u64, allow: bool) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_PERMISSION_STATE_ALLOW, COREWEBVIEW2_PERMISSION_STATE_DENY,
    };
    if let Ok(mut lock) = PERMISSION_STORE.lock() {
        if let Some(map) = lock.as_mut() {
            if let Some(active) = map.remove(&permission_id) {
                let state = if allow {
                    COREWEBVIEW2_PERMISSION_STATE_ALLOW
                } else {
                    COREWEBVIEW2_PERMISSION_STATE_DENY
                };
                unsafe {
                    let _ = active.args.SetState(state);
                    let _ = active.deferral.Complete();
                }
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn resolve_permission(_permission_id: u64, _allow: bool) {}

#[cfg(target_os = "windows")]
pub fn wake_webview(wv: &WebView) {
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_3;
    use windows::core::Interface;
    use wry::WebViewExtWindows;

    let controller = wv.controller();
    let _ = unsafe { controller.NotifyParentWindowPositionChanged() };
    if let Ok(core) = unsafe { controller.CoreWebView2() } {
        if let Ok(core3) = core.cast::<ICoreWebView2_3>() {
            let mut is_suspended = windows::core::BOOL(0);
            if unsafe { core3.IsSuspended(&mut is_suspended) }.is_ok() && is_suspended.as_bool() {
                let _ = unsafe { core3.Resume() };
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub fn focus_webview(wv: &WebView) {
    use webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC;
    use wry::WebViewExtWindows;

    let controller = wv.controller();
    let _ = unsafe { controller.MoveFocus(COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC) };
}

#[cfg(not(target_os = "windows"))]
pub fn wake_webview(_wv: &WebView) {}

#[cfg(not(target_os = "windows"))]
pub fn focus_webview(_wv: &WebView) {}

#[cfg(target_os = "windows")]
pub fn attach_accelerator_keys(
    webview: &WebView,
    window_id: winit::window::WindowId,
    proxy: EventLoopProxy<crate::BrowserEvent>,
) {
    use webview2_com::AcceleratorKeyPressedEventHandler;
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN, COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_MENU, VK_SHIFT,
    };
    use wry::WebViewExtWindows;

    let controller = webview.controller().clone();

    let handler = AcceleratorKeyPressedEventHandler::create(Box::new(move |_sender, args| {
        if let Some(args) = args {
            let mut kind =
                webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_KEY_EVENT_KIND(0);
            if unsafe { args.KeyEventKind(&mut kind) }.is_err() {
                return Ok(());
            }

            if kind == COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN
                || kind == COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN
            {
                let mut vkey = 0;
                if unsafe { args.VirtualKey(&mut vkey) }.is_ok() {
                    let ctrl = unsafe { GetAsyncKeyState(VK_CONTROL.0 as i32) } & -32768 != 0;
                    let alt = unsafe { GetAsyncKeyState(VK_MENU.0 as i32) } & -32768 != 0;
                    let shift = unsafe { GetAsyncKeyState(VK_SHIFT.0 as i32) } & -32768 != 0;

                    let mut handled = false;
                    let mut shortcut = None;

                    if ctrl && !alt {
                        match vkey {
                            0x54 => {
                                // 'T'
                                if shift {
                                    shortcut = Some("Ctrl+Shift+T");
                                } else {
                                    shortcut = Some("Ctrl+T");
                                }
                                handled = true;
                            }
                            0x57 => {
                                // 'W'
                                shortcut = Some("Ctrl+W");
                                handled = true;
                            }
                            0x4C => {
                                // 'L'
                                shortcut = Some("Ctrl+L");
                                handled = true;
                            }
                            0x45 => {
                                // 'E'
                                shortcut = Some("Ctrl+E");
                                handled = true;
                            }
                            0x4B => {
                                // 'K'
                                shortcut = Some("Ctrl+K");
                                handled = true;
                            }
                            0x46 => {
                                // 'F'
                                shortcut = Some("Ctrl+F");
                                handled = true;
                            }
                            0x4A => {
                                // 'J'
                                shortcut = Some("Ctrl+J");
                                handled = true;
                            }
                            0x50 => {
                                // 'P'
                                shortcut = Some("Ctrl+P");
                                handled = true;
                            }
                            0x52 => {
                                // 'R'
                                if shift {
                                    shortcut = Some("Ctrl+Shift+R");
                                } else {
                                    shortcut = Some("Ctrl+R");
                                }
                                handled = true;
                            }
                            0x4E => {
                                // 'N'
                                if shift {
                                    shortcut = Some("Ctrl+Shift+N");
                                } else {
                                    shortcut = Some("Ctrl+N");
                                }
                                handled = true;
                            }
                            0x09 => {
                                // Tab
                                if shift {
                                    shortcut = Some("Ctrl+Shift+Tab");
                                } else {
                                    shortcut = Some("Ctrl+Tab");
                                }
                                handled = true;
                            }
                            0x31 => {
                                shortcut = Some("Ctrl+1");
                                handled = true;
                            }
                            0x32 => {
                                shortcut = Some("Ctrl+2");
                                handled = true;
                            }
                            0x33 => {
                                shortcut = Some("Ctrl+3");
                                handled = true;
                            }
                            0x34 => {
                                shortcut = Some("Ctrl+4");
                                handled = true;
                            }
                            0x35 => {
                                shortcut = Some("Ctrl+5");
                                handled = true;
                            }
                            0x36 => {
                                shortcut = Some("Ctrl+6");
                                handled = true;
                            }
                            0x37 => {
                                shortcut = Some("Ctrl+7");
                                handled = true;
                            }
                            0x38 => {
                                shortcut = Some("Ctrl+8");
                                handled = true;
                            }
                            0x39 => {
                                shortcut = Some("Ctrl+9");
                                handled = true;
                            }
                            0xBB | 0x6B => {
                                // '+' or Numpad '+'
                                shortcut = Some("Ctrl+Plus");
                                handled = true;
                            }
                            0xBD | 0x6D => {
                                // '-' or Numpad '-'
                                shortcut = Some("Ctrl+Minus");
                                handled = true;
                            }
                            0x30 | 0x60 => {
                                // '0' or Numpad '0'
                                shortcut = Some("Ctrl+Zero");
                                handled = true;
                            }
                            _ => {}
                        }
                    } else if alt && !ctrl {
                        match vkey {
                            0x25 => {
                                // Left Arrow
                                shortcut = Some("Alt+Left");
                                handled = true;
                            }
                            0x27 => {
                                // Right Arrow
                                shortcut = Some("Alt+Right");
                                handled = true;
                            }
                            _ => {}
                        }
                    } else if !ctrl && !alt {
                        match vkey {
                            0x7B => {
                                // F12
                                shortcut = Some("F12");
                                handled = true;
                            }
                            0x74 => {
                                // F5
                                shortcut = Some("F5");
                                handled = true;
                            }
                            0x1B => {
                                // Escape
                                shortcut = Some("Escape");
                                handled = true;
                            }
                            _ => {}
                        }
                    }

                    if handled {
                        let _ = unsafe { args.SetHandled(true) };
                        if let Some(s) = shortcut {
                            let _ = proxy.send_event(crate::BrowserEvent::Shortcut(
                                window_id,
                                s.to_string(),
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }));

    let mut token = Default::default();
    let _ = unsafe { controller.add_AcceleratorKeyPressed(&handler, &mut token) };
}

#[cfg(target_os = "windows")]
pub fn attach_navigation_events(
    webview: &WebView,
    window_id: winit::window::WindowId,
    tab_id: evergreen_core::tabs::TabId,
    proxy: EventLoopProxy<crate::BrowserEvent>,
    allowed_cert_hosts: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
) {
    use webview2_com::{
        DocumentTitleChangedEventHandler, HistoryChangedEventHandler, SourceChangedEventHandler,
    };
    use wry::WebViewExtWindows;

    if let Ok(core) = unsafe { webview.controller().CoreWebView2() } {
        // 1. History Changed
        let core_history = core.clone();
        let proxy_history = proxy.clone();
        let history_handler =
            HistoryChangedEventHandler::create(Box::new(move |_sender, _args| {
                let mut can_back = windows::core::BOOL(0);
                let mut can_forward = windows::core::BOOL(0);
                unsafe {
                    let _ = core_history.CanGoBack(&mut can_back);
                    let _ = core_history.CanGoForward(&mut can_forward);
                }
                let mut uri_pwstr = windows::core::PWSTR::null();
                if unsafe { core_history.Source(&mut uri_pwstr) }.is_ok() && !uri_pwstr.is_null() {
                    let uri_str = unsafe { uri_pwstr.to_string() }.unwrap_or_default();
                    if !uri_str.is_empty()
                        && !uri_str.starts_with("data:text/html")
                        && uri_str != "about:blank"
                        && uri_str != "aboutblank"
                    {
                        let _ = proxy_history.send_event(crate::BrowserEvent::TabNavigated(
                            window_id, tab_id, uri_str,
                        ));
                    }
                }
                let mut title_pwstr = windows::core::PWSTR::null();
                if unsafe { core_history.DocumentTitle(&mut title_pwstr) }.is_ok()
                    && !title_pwstr.is_null()
                {
                    let title_str = unsafe { title_pwstr.to_string() }.unwrap_or_default();
                    if !title_str.is_empty() {
                        let _ = proxy_history.send_event(crate::BrowserEvent::TabTitleChanged(
                            window_id, tab_id, title_str,
                        ));
                    }
                }
                let _ = proxy_history.send_event(crate::BrowserEvent::HistoryChanged(
                    window_id,
                    tab_id,
                    can_back.as_bool(),
                    can_forward.as_bool(),
                ));
                Ok(())
            }));
        let mut token_h = Default::default();
        let _ = unsafe { core.add_HistoryChanged(&history_handler, &mut token_h) };

        // 2. Source Changed (instantaneous URI updates on redirect, SPA pushState, back/forward)
        let core_source = core.clone();
        let proxy_source = proxy.clone();
        let source_handler = SourceChangedEventHandler::create(Box::new(move |_sender, _args| {
            let mut uri_pwstr = windows::core::PWSTR::null();
            if unsafe { core_source.Source(&mut uri_pwstr) }.is_ok() && !uri_pwstr.is_null() {
                let uri_str = unsafe { uri_pwstr.to_string() }.unwrap_or_default();
                if !uri_str.is_empty()
                    && !uri_str.starts_with("data:text/html")
                    && uri_str != "about:blank"
                    && uri_str != "aboutblank"
                {
                    let _ = proxy_source.send_event(crate::BrowserEvent::TabNavigated(
                        window_id, tab_id, uri_str,
                    ));
                }
            }
            let mut title_pwstr = windows::core::PWSTR::null();
            if unsafe { core_source.DocumentTitle(&mut title_pwstr) }.is_ok()
                && !title_pwstr.is_null()
            {
                let title_str = unsafe { title_pwstr.to_string() }.unwrap_or_default();
                if !title_str.is_empty() {
                    let _ = proxy_source.send_event(crate::BrowserEvent::TabTitleChanged(
                        window_id, tab_id, title_str,
                    ));
                }
            }
            // Also update history state on source change
            let mut can_back = windows::core::BOOL(0);
            let mut can_forward = windows::core::BOOL(0);
            unsafe {
                let _ = core_source.CanGoBack(&mut can_back);
                let _ = core_source.CanGoForward(&mut can_forward);
            }
            let _ = proxy_source.send_event(crate::BrowserEvent::HistoryChanged(
                window_id,
                tab_id,
                can_back.as_bool(),
                can_forward.as_bool(),
            ));
            Ok(())
        }));
        let mut token_s = Default::default();
        let _ = unsafe { core.add_SourceChanged(&source_handler, &mut token_s) };

        // 3. Document Title Changed (instantaneous title updates)
        let core_title = core.clone();
        let proxy_title = proxy.clone();
        let title_handler =
            DocumentTitleChangedEventHandler::create(Box::new(move |_sender, _args| {
                let mut title_pwstr = windows::core::PWSTR::null();
                if unsafe { core_title.DocumentTitle(&mut title_pwstr) }.is_ok()
                    && !title_pwstr.is_null()
                {
                    let title_str = unsafe { title_pwstr.to_string() }.unwrap_or_default();
                    if !title_str.is_empty() {
                        let _ = proxy_title.send_event(crate::BrowserEvent::TabTitleChanged(
                            window_id, tab_id, title_str,
                        ));
                    }
                }
                Ok(())
            }));
        let mut token_t = Default::default();
        let _ = unsafe { core.add_DocumentTitleChanged(&title_handler, &mut token_t) };

        // 4. New Window Requested (target="_blank" link clicks, e.g. Garry Tan YC profile)
        let proxy_new_win = proxy.clone();
        let new_window_handler =
            webview2_com::NewWindowRequestedEventHandler::create(Box::new(move |_sender, args| {
                if let Some(args) = args {
                    let mut uri_pwstr = windows::core::PWSTR::null();
                    if unsafe { args.Uri(&mut uri_pwstr) }.is_ok() && !uri_pwstr.is_null() {
                        let uri_str = unsafe { uri_pwstr.to_string() }.unwrap_or_default();
                        if !uri_str.is_empty() {
                            let _ = unsafe { args.SetHandled(true) };
                            let _ = proxy_new_win.send_event(crate::BrowserEvent::Ipc(
                                window_id,
                                evergreen_core::ipc::UiToHostMessage::CreateTab {
                                    url: Some(uri_str),
                                },
                            ));
                        }
                    }
                }
                Ok(())
            }));
        let mut token_nw = Default::default();
        let _ = unsafe { core.add_NewWindowRequested(&new_window_handler, &mut token_nw) };

        // 5. Site Permission Requested
        let proxy_perm = proxy.clone();
        let perm_handler = webview2_com::PermissionRequestedEventHandler::create(Box::new(
            move |_sender, args| {
                if let Some(args) = args {
                    let deferral = unsafe { args.GetDeferral() }.ok();
                    let perm_id = NEXT_PERMISSION_ID.fetch_add(1, Ordering::SeqCst);

                    let mut uri_pwstr = windows::core::PWSTR::null();
                    let _ = unsafe { args.Uri(&mut uri_pwstr) };
                    let uri_str = if !uri_pwstr.is_null() {
                        unsafe { uri_pwstr.to_string() }.unwrap_or_default()
                    } else {
                        String::new()
                    };
                    let mut kind =
                        webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_PERMISSION_KIND(
                            0,
                        );
                    let _ = unsafe { args.PermissionKind(&mut kind) };
                    let kind_str = match kind.0 {
                        1 => "Microphone",
                        2 => "Camera",
                        3 => "Geolocation",
                        4 => "Notifications",
                        5 => "Other Sensors",
                        6 => "Clipboard Read",
                        _ => "Site Permission",
                    };

                    if let Some(d) = deferral {
                        if let Ok(mut lock) = PERMISSION_STORE.lock() {
                            let map = lock.get_or_insert_with(HashMap::new);
                            map.insert(
                                perm_id,
                                ActivePermission {
                                    args: args.clone(),
                                    deferral: d,
                                },
                            );
                        }
                    }

                    let host = crate::extract_host(&uri_str).unwrap_or_else(|| uri_str.clone());
                    let _ = proxy_perm.send_event(crate::BrowserEvent::PermissionPrompt {
                        window_id,
                        permission_id: perm_id,
                        origin: host,
                        permission_kind: kind_str.to_string(),
                    });
                }
                Ok(())
            },
        ));
        let mut token_p = Default::default();
        let _ = unsafe { core.add_PermissionRequested(&perm_handler, &mut token_p) };

        // Process Failed Handler (detect GPU/renderer reset on sleep/standby)
        let proxy_fail = proxy.clone();
        let process_failed_handler = webview2_com::ProcessFailedEventHandler::create(Box::new(
            move |_sender, args| {
                if let Some(args) = args {
                    let mut kind = webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_PROCESS_FAILED_KIND(0);
                    let _ = unsafe { args.ProcessFailedKind(&mut kind) };
                    eprintln!(
                        "[PROCESS FAILED] WebView2 process failed kind: {:?}",
                        kind.0
                    );
                    let _ = proxy_fail
                        .send_event(crate::BrowserEvent::TabCrashed(window_id, tab_id, kind.0));
                }
                Ok(())
            },
        ));
        let mut token_pf = Default::default();
        let _ = unsafe { core.add_ProcessFailed(&process_failed_handler, &mut token_pf) };

        // 6. Strict TLS Warning Interstitial (ServerCertificateErrorDetected)
        if let Ok(core14) =
            core.cast::<webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_14>()
        {
            let proxy_cert = proxy.clone();
            let cert_allowed = allowed_cert_hosts.clone();
            let cert_handler = webview2_com::ServerCertificateErrorDetectedEventHandler::create(
                Box::new(move |_sender, args| {
                    if let Some(args) = args {
                        let mut error_status = webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_WEB_ERROR_STATUS(0);
                        let _ = unsafe { args.ErrorStatus(&mut error_status) };
                        let mut uri_pwstr = windows::core::PWSTR::null();
                        let _ = unsafe { args.RequestUri(&mut uri_pwstr) };
                        let uri_str = if !uri_pwstr.is_null() {
                            unsafe { uri_pwstr.to_string() }.unwrap_or_default()
                        } else {
                            String::new()
                        };

                        let host = crate::extract_host(&uri_str).unwrap_or_else(|| uri_str.clone());
                        if cert_allowed
                            .lock()
                            .map(|set| set.contains(&host))
                            .unwrap_or(false)
                        {
                            eprintln!(
                                "[TLS BYPASS] Allowing user-whitelisted certificate for: {}",
                                host
                            );
                            let _ = unsafe {
                                args.SetAction(webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_SERVER_CERTIFICATE_ERROR_ACTION_ALWAYS_ALLOW)
                            };
                            return Ok(());
                        }

                        // Otherwise cancel the untrusted request and show interstitial
                        let _ = unsafe {
                            args.SetAction(webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_SERVER_CERTIFICATE_ERROR_ACTION_CANCEL)
                        };
                        let _ =
                            proxy_cert.send_event(crate::BrowserEvent::ServerCertificateError {
                                window_id,
                                tab_id,
                                request_uri: uri_str,
                                error_status: error_status.0,
                            });
                    }
                    Ok(())
                }),
            );
            let mut token_cert = Default::default();
            let _ = unsafe {
                core14.add_ServerCertificateErrorDetected(&cert_handler, &mut token_cert)
            };
        }

        // 7. Download Starting with Save As prompt and progress tracking
        use windows::core::Interface;
        if let Ok(core4) =
            core.cast::<webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_4>()
        {
            let proxy_dl = proxy.clone();
            let dl_handler = webview2_com::DownloadStartingEventHandler::create(Box::new(
                move |_sender, args| {
                    if let Some(args) = args {
                        if let Ok(op) = unsafe { args.DownloadOperation() } {
                            let mut result_path_pwstr = windows::core::PWSTR::null();
                            let _ = unsafe { op.ResultFilePath(&mut result_path_pwstr) };
                            let mut total_bytes = 0i64;
                            let _ = unsafe { op.TotalBytesToReceive(&mut total_bytes) };

                            let filename = if !result_path_pwstr.is_null() {
                                let path_str =
                                    unsafe { result_path_pwstr.to_string() }.unwrap_or_default();
                                std::path::Path::new(&path_str)
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "download".to_string())
                            } else {
                                "download".to_string()
                            };

                            let save_path = show_save_file_dialog(&filename);
                            if let Some(chosen_path) = save_path {
                                let wpath: Vec<u16> = chosen_path
                                    .to_string_lossy()
                                    .encode_utf16()
                                    .chain(std::iter::once(0))
                                    .collect();
                                let _ = unsafe {
                                    args.SetResultFilePath(windows::core::PCWSTR(wpath.as_ptr()))
                                };
                                let _ = unsafe { args.SetHandled(true) };

                                let proxy_bytes = proxy_dl.clone();
                                let dl_fn = filename.clone();
                                let op_bytes = op.clone();
                                let bytes_handler =
                                    webview2_com::BytesReceivedChangedEventHandler::create(
                                        Box::new(move |_sender, _args| {
                                            let mut received = 0i64;
                                            let mut total = 0i64;
                                            let mut raw_state = webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_DOWNLOAD_STATE(0);
                                            unsafe {
                                                let _ = op_bytes.BytesReceived(&mut received);
                                                let _ = op_bytes.TotalBytesToReceive(&mut total);
                                                let _ = op_bytes.State(&mut raw_state);
                                            }
                                            let state_str = match raw_state.0 {
                                                1 => "Interrupted",
                                                2 => "Completed",
                                                _ => {
                                                    if total > 0 && received >= total {
                                                        "Completed"
                                                    } else {
                                                        "InProgress"
                                                    }
                                                }
                                            };
                                            let _ = proxy_bytes.send_event(
                                                crate::BrowserEvent::DownloadProgress {
                                                    download_id: 1,
                                                    filename: dl_fn.clone(),
                                                    received_bytes: received,
                                                    total_bytes: total,
                                                    state: state_str.to_string(),
                                                },
                                            );
                                            Ok(())
                                        }),
                                    );
                                let mut token_b = Default::default();
                                let _ = unsafe {
                                    op.add_BytesReceivedChanged(&bytes_handler, &mut token_b)
                                };

                                let proxy_state = proxy_dl.clone();
                                let dl_fn2 = filename.clone();
                                let op_state = op.clone();
                                let state_handler = webview2_com::StateChangedEventHandler::create(
                                    Box::new(move |_sender, _args| {
                                        let mut state = webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_DOWNLOAD_STATE(0);
                                        let mut received = 0i64;
                                        let mut total = 0i64;
                                        unsafe {
                                            let _ = op_state.State(&mut state);
                                            let _ = op_state.BytesReceived(&mut received);
                                            let _ = op_state.TotalBytesToReceive(&mut total);
                                        }
                                        let state_str = match state.0 {
                                            0 if total > 0 && received >= total => "Completed",
                                            0 => "InProgress",
                                            1 => "Interrupted",
                                            2 => "Completed",
                                            _ => "InProgress",
                                        };
                                        let _ = proxy_state.send_event(
                                            crate::BrowserEvent::DownloadProgress {
                                                download_id: 1,
                                                filename: dl_fn2.clone(),
                                                received_bytes: received,
                                                total_bytes: total,
                                                state: state_str.to_string(),
                                            },
                                        );
                                        Ok(())
                                    }),
                                );
                                let mut token_st = Default::default();
                                let _ =
                                    unsafe { op.add_StateChanged(&state_handler, &mut token_st) };

                                let _ =
                                    proxy_dl.send_event(crate::BrowserEvent::DownloadProgress {
                                        download_id: 1,
                                        filename,
                                        received_bytes: 0,
                                        total_bytes,
                                        state: "InProgress".to_string(),
                                    });
                            } else {
                                let _ = unsafe { args.SetCancel(true) };
                                let _ = unsafe { args.SetHandled(true) };
                            }
                        }
                    }
                    Ok(())
                },
            ));
            let mut token_dl = Default::default();
            let _ = unsafe { core4.add_DownloadStarting(&dl_handler, &mut token_dl) };
        }
    }
}

#[cfg(target_os = "windows")]
pub fn show_save_file_dialog(default_filename: &str) -> Option<std::path::PathBuf> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
    use windows::Win32::UI::Shell::{FileSaveDialog, IFileSaveDialog, SIGDN_FILESYSPATH};

    unsafe {
        let dialog: IFileSaveDialog = CoCreateInstance(&FileSaveDialog, None, CLSCTX_ALL).ok()?;
        let wname: Vec<u16> = default_filename
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let _ = dialog.SetFileName(PCWSTR(wname.as_ptr()));
        let title: Vec<u16> = "Save Download As"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let _ = dialog.SetTitle(PCWSTR(title.as_ptr()));

        if dialog.Show(None).is_ok() {
            if let Ok(item) = dialog.GetResult() {
                if let Ok(path_pwstr) = item.GetDisplayName(SIGDN_FILESYSPATH) {
                    if !path_pwstr.is_null() {
                        let path_str = path_pwstr.to_string().ok()?;
                        return Some(std::path::PathBuf::from(path_str));
                    }
                }
            }
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
pub fn attach_accelerator_keys(
    _webview: &WebView,
    _window_id: winit::window::WindowId,
    _proxy: EventLoopProxy<crate::BrowserEvent>,
) {
}

#[cfg(not(target_os = "windows"))]
pub fn attach_navigation_events(
    _webview: &WebView,
    _window_id: winit::window::WindowId,
    _tab_id: evergreen_core::tabs::TabId,
    _proxy: EventLoopProxy<crate::BrowserEvent>,
    _allowed_cert_hosts: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
) {
}
