use std::path::Path;
use std::process::Command;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};

use super::detect_opener;

pub fn launch_file(path: &Path) -> std::io::Result<()> {
    let mut command_parts = detect_opener(path);
    if command_parts.is_empty() {
        command_parts.push("xdg-open".to_string());
    }

    let program = command_parts.remove(0);
    let mut command = Command::new(program);
    command.args(command_parts).arg(path);
    command.status()?;
    Ok(())
}

pub fn launch_file_with_terminal(path: &Path) -> std::io::Result<()> {
    disable_raw_mode()?;
    let mut stdout = std::io::stdout();
    if let Err(error) = execute!(stdout, DisableMouseCapture, LeaveAlternateScreen) {
        let _ = execute!(stdout, EnterAlternateScreen, EnableMouseCapture);
        let _ = enable_raw_mode();
        return Err(error);
    }

    let launch_result = launch_file(path);
    let restore_result = restore_terminal();
    restore_result?;
    launch_result
}

fn restore_terminal() -> std::io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    if let Err(error) = execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
        let _ = execute!(stdout, DisableMouseCapture, LeaveAlternateScreen);
        let _ = disable_raw_mode();
        return Err(error);
    }
    Ok(())
}
