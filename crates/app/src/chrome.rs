//! Chrome UI webview management and IPC message routing.

use evergreen_core::ipc::{HostToUiMessage, UiToHostMessage};

pub const EMBEDDED_CHROME_HTML: &str = include_str!("../ui/index.html");

pub struct ChromeController {
    // IPC and webview bridge state
}

impl ChromeController {
    pub fn new() -> Self {
        Self {}
    }

    pub fn handle_incoming_ipc(&mut self, message: UiToHostMessage) {
        match message {
            UiToHostMessage::CreateTab { url } => {
                println!("Chrome IPC: CreateTab {:?}", url);
            }
            UiToHostMessage::SwitchTab { id } => {
                println!("Chrome IPC: SwitchTab {:?}", id);
            }
            UiToHostMessage::CloseTab { id } => {
                println!("Chrome IPC: CloseTab {:?}", id);
            }
            UiToHostMessage::Navigate { url } => {
                println!("Chrome IPC: Navigate {}", url);
            }
            UiToHostMessage::GoBack => {
                println!("Chrome IPC: GoBack");
            }
            UiToHostMessage::GoForward => {
                println!("Chrome IPC: GoForward");
            }
            UiToHostMessage::Reload => {
                println!("Chrome IPC: Reload");
            }
            UiToHostMessage::Stop => {
                println!("Chrome IPC: Stop");
            }
            UiToHostMessage::OpenDevTools => {
                println!("Chrome IPC: OpenDevTools");
            }
            UiToHostMessage::OpenSettings => {
                println!("Chrome IPC: OpenSettings");
            }
            UiToHostMessage::SaveSettings { settings_json } => {
                println!("Chrome IPC: SaveSettings length {}", settings_json.len());
            }
            UiToHostMessage::RunEngineUpdate => {
                println!("Chrome IPC: RunEngineUpdate");
            }
            UiToHostMessage::RunForkUpdate => {
                println!("Chrome IPC: RunForkUpdate");
            }
        }
    }
}
