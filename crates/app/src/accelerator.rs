//! Win32 keyboard accelerator hook for WebView2.
//! Intercepts browser shortcuts (Ctrl+T, Ctrl+W, Ctrl+L, F12, etc.) at the COM layer
//! before child webview HWNDs consume them.

use winit::event_loop::EventLoopProxy;
use wry::WebView;

#[cfg(target_os = "windows")]
pub fn attach_accelerator_keys(
    webview: &WebView,
    proxy: EventLoopProxy<crate::BrowserEvent>,
) {
    use webview2_com::AcceleratorKeyPressedEventHandler;
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN,
        COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_MENU, VK_SHIFT,
    };
    use wry::WebViewExtWindows;

    let controller = webview.controller().clone();

    let handler = AcceleratorKeyPressedEventHandler::create(Box::new(move |_sender, args| {
        if let Some(args) = args {
            let mut kind = webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_KEY_EVENT_KIND(0);
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
                            0x54 => { // 'T'
                                shortcut = Some("Ctrl+T");
                                handled = true;
                            }
                            0x57 => { // 'W'
                                shortcut = Some("Ctrl+W");
                                handled = true;
                            }
                            0x4C => { // 'L'
                                shortcut = Some("Ctrl+L");
                                handled = true;
                            }
                            0x52 => { // 'R'
                                shortcut = Some("Ctrl+R");
                                handled = true;
                            }
                            0x4E => { // 'N'
                                shortcut = Some("Ctrl+N");
                                handled = true;
                            }
                            0x09 => { // Tab
                                if shift {
                                    shortcut = Some("Ctrl+Shift+Tab");
                                } else {
                                    shortcut = Some("Ctrl+Tab");
                                }
                                handled = true;
                            }
                            _ => {}
                        }
                    } else if alt && !ctrl {
                        match vkey {
                            0x25 => { // Left Arrow
                                shortcut = Some("Alt+Left");
                                handled = true;
                            }
                            0x27 => { // Right Arrow
                                shortcut = Some("Alt+Right");
                                handled = true;
                            }
                            _ => {}
                        }
                    } else if !ctrl && !alt {
                        match vkey {
                            0x7B => { // F12
                                shortcut = Some("F12");
                                handled = true;
                            }
                            0x74 => { // F5
                                shortcut = Some("F5");
                                handled = true;
                            }
                            _ => {}
                        }
                    }

                    if handled {
                        let _ = unsafe { args.SetHandled(true) };
                        if let Some(s) = shortcut {
                            let _ = proxy.send_event(crate::BrowserEvent::Shortcut(s.to_string()));
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
    tab_id: evergreen_core::tabs::TabId,
    proxy: EventLoopProxy<crate::BrowserEvent>,
) {
    use webview2_com::{DocumentTitleChangedEventHandler, HistoryChangedEventHandler, SourceChangedEventHandler};
    use wry::WebViewExtWindows;

    if let Ok(core) = unsafe { webview.controller().CoreWebView2() } {
        // 1. History Changed
        let core_history = core.clone();
        let proxy_history = proxy.clone();
        let history_handler = HistoryChangedEventHandler::create(Box::new(move |_sender, _args| {
            let mut can_back = windows::core::BOOL(0);
            let mut can_forward = windows::core::BOOL(0);
            unsafe {
                let _ = core_history.CanGoBack(&mut can_back);
                let _ = core_history.CanGoForward(&mut can_forward);
            }
            let mut uri_pwstr = windows::core::PWSTR::null();
            if unsafe { core_history.Source(&mut uri_pwstr) }.is_ok() && !uri_pwstr.is_null() {
                let uri_str = unsafe { uri_pwstr.to_string() }.unwrap_or_default();
                if !uri_str.is_empty() && !uri_str.starts_with("data:text/html") {
                    let _ = proxy_history.send_event(crate::BrowserEvent::TabNavigated(tab_id, uri_str));
                }
            }
            let mut title_pwstr = windows::core::PWSTR::null();
            if unsafe { core_history.DocumentTitle(&mut title_pwstr) }.is_ok() && !title_pwstr.is_null() {
                let title_str = unsafe { title_pwstr.to_string() }.unwrap_or_default();
                if !title_str.is_empty() {
                    let _ = proxy_history.send_event(crate::BrowserEvent::TabTitleChanged(tab_id, title_str));
                }
            }
            let _ = proxy_history.send_event(crate::BrowserEvent::HistoryChanged(
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
                if !uri_str.is_empty() && !uri_str.starts_with("data:text/html") {
                    let _ = proxy_source.send_event(crate::BrowserEvent::TabNavigated(tab_id, uri_str));
                }
            }
            let mut title_pwstr = windows::core::PWSTR::null();
            if unsafe { core_source.DocumentTitle(&mut title_pwstr) }.is_ok() && !title_pwstr.is_null() {
                let title_str = unsafe { title_pwstr.to_string() }.unwrap_or_default();
                if !title_str.is_empty() {
                    let _ = proxy_source.send_event(crate::BrowserEvent::TabTitleChanged(tab_id, title_str));
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
        let title_handler = DocumentTitleChangedEventHandler::create(Box::new(move |_sender, _args| {
            let mut title_pwstr = windows::core::PWSTR::null();
            if unsafe { core_title.DocumentTitle(&mut title_pwstr) }.is_ok() && !title_pwstr.is_null() {
                let title_str = unsafe { title_pwstr.to_string() }.unwrap_or_default();
                if !title_str.is_empty() {
                    let _ = proxy_title.send_event(crate::BrowserEvent::TabTitleChanged(tab_id, title_str));
                }
            }
            Ok(())
        }));
        let mut token_t = Default::default();
        let _ = unsafe { core.add_DocumentTitleChanged(&title_handler, &mut token_t) };
    }
}

#[cfg(not(target_os = "windows"))]
pub fn attach_accelerator_keys(
    _webview: &WebView,
    _proxy: EventLoopProxy<crate::BrowserEvent>,
) {}

#[cfg(not(target_os = "windows"))]
pub fn attach_navigation_events(
    _webview: &WebView,
    _tab_id: evergreen_core::tabs::TabId,
    _proxy: EventLoopProxy<crate::BrowserEvent>,
) {}
