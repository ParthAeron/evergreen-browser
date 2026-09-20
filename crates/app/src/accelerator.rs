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

#[cfg(not(target_os = "windows"))]
pub fn attach_accelerator_keys(
    _webview: &WebView,
    _proxy: EventLoopProxy<crate::BrowserEvent>,
) {}
