#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::process::Command;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Execution result of a local update command.
#[derive(Debug, Clone)]
pub struct UpdateCommandResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Execute a local shell update command (e.g. the WebView2 Evergreen bootstrapper or git pull && cargo build).
pub fn run_local_command(cmd_str: &str) -> Result<UpdateCommandResult, std::io::Error> {
    let output = if cfg!(target_os = "windows") {
        let mut cmd = Command::new("powershell");
        cmd.args(["-NoProfile", "-Command", cmd_str]);
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd.output()?
    } else {
        Command::new("sh").args(["-c", cmd_str]).output()?
    };

    Ok(UpdateCommandResult {
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}
