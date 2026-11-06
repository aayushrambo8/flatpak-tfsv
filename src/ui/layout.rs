use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn render_main_view(area: Rect) -> Vec<Rect> {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5), Constraint::Length(2)])
        .split(area);
    chunks.to_vec()
}

pub fn render_file_view(area: Rect) -> Vec<Rect> {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(26), Constraint::Percentage(74)])
        .split(area);
    chunks.to_vec()
}
