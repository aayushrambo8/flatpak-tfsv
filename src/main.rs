mod app;
mod config;
mod editor;
mod events;
mod filesystem;
mod opener;
mod search;
mod transfer;
mod ui;

fn main() {
    if let Err(err) = app::App::run_main() {
        let _ = crossterm::terminal::disable_raw_mode();
        let mut stdout = std::io::stdout();
        let _ = crossterm::execute!(
            stdout,
            crossterm::event::DisableMouseCapture,
            crossterm::terminal::LeaveAlternateScreen
        );
        eprintln!("TFSV error: {err}");
        std::process::exit(1);
    }
}
