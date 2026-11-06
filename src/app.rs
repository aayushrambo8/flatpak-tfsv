use std::cell::Cell;
use std::path::PathBuf;

use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Row, Table};
use ratatui::Terminal;

use crate::config::Settings;
use crate::events::{handle_key_event, handle_mouse_event};
use crate::filesystem::{create_directory as fs_create_directory, copy_path, delete_path, load_directory, move_path, rename_path, sort_entries, FileEntry, SortMode};

struct TerminalSession {
    active: bool,
}

impl TerminalSession {
    fn enter() -> std::io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
            let _ = execute!(stdout, DisableMouseCapture, LeaveAlternateScreen);
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self { active: true })
    }

    fn suspend(&mut self) -> std::io::Result<()> {
        disable_raw_mode()?;
        let mut stdout = std::io::stdout();
        if let Err(error) = execute!(stdout, DisableMouseCapture, LeaveAlternateScreen) {
            let _ = enable_raw_mode();
            return Err(error);
        }
        self.active = false;
        Ok(())
    }

}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if self.active {
            let _ = disable_raw_mode();
            let mut stdout = std::io::stdout();
            let _ = execute!(stdout, DisableMouseCapture, LeaveAlternateScreen);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ClipboardMode {
    Copy,
    Cut,
}

struct FileClipboard {
    source: PathBuf,
    mode: ClipboardMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolbarAction {
    New,
    Copy,
    Cut,
    Paste,
    Rename,
    Delete,
    Refresh,
    Sort,
    Hidden,
}

impl ToolbarAction {
    const ALL: [Self; 9] = [
        Self::New,
        Self::Copy,
        Self::Cut,
        Self::Paste,
        Self::Rename,
        Self::Delete,
        Self::Refresh,
        Self::Sort,
        Self::Hidden,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::New => "New",
            Self::Copy => "Copy",
            Self::Cut => "Cut",
            Self::Paste => "Paste",
            Self::Rename => "Rename",
            Self::Delete => "Delete",
            Self::Refresh => "Refresh",
            Self::Sort => "Sort",
            Self::Hidden => "Hidden",
        }
    }
}

pub struct App {
    settings: Settings,
    home_dir: PathBuf,
    unrestricted: bool,
    current_dir: PathBuf,
    entries: Vec<FileEntry>,
    selected: usize,
    sort_mode: SortMode,
    status: String,
    context_menu_open: bool,
    context_menu_index: usize,
    context_menu_items: Vec<&'static str>,
    clipboard: Option<FileClipboard>,
    dialog_message: Option<String>,
    visible_start: Cell<usize>,
    toolbar_area: Cell<Rect>,
    places_area: Cell<Rect>,
    list_area: Cell<Rect>,
    breadcrumb_area: Cell<Rect>,
    hovered_entry: Cell<Option<usize>>,
    screen_invalidated: bool,
}

impl App {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let settings = Settings::load()?;
        let unrestricted = std::env::var_os("SUDO_UID").is_some();
        let home_dir = if unrestricted {
            PathBuf::from("/")
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/"))
                .canonicalize()?
        };
        let launch_dir = std::env::current_dir().ok();
        let current_dir = resolve_start_directory(
            &home_dir,
            launch_dir,
            &settings.start_dir,
            unrestricted,
        );
        let mut app = Self {
            settings,
            home_dir,
            unrestricted,
            current_dir,
            entries: Vec::new(),
            selected: 0,
            sort_mode: SortMode::Name,
            status: String::from("TFSV ready"),
            context_menu_open: false,
            context_menu_index: 0,
            context_menu_items: vec!["Open", "Copy", "Cut", "Paste", "Rename", "Delete", "Properties"],
            clipboard: None,
            dialog_message: None,
            visible_start: Cell::new(0),
            toolbar_area: Cell::new(Rect::default()),
            places_area: Cell::new(Rect::default()),
            list_area: Cell::new(Rect::default()),
            breadcrumb_area: Cell::new(Rect::default()),
            hovered_entry: Cell::new(None),
            screen_invalidated: false,
        };
        app.refresh();
        Ok(app)
    }

    pub fn run_main() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = Self::new()?;
        let mut session = TerminalSession::enter()?;
        Self::run_terminal_loop(&mut app)?;
        session.suspend()?;
        Ok(())
    }

    fn run_terminal_loop(app: &mut Self) -> Result<(), Box<dyn std::error::Error>> {
        let stdout = std::io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        loop {
            if app.screen_invalidated {
                terminal.clear()?;
                app.screen_invalidated = false;
            }
            terminal.draw(|frame| app.render(frame))?;
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if handle_key_event(app, key)? {
                        break;
                    }
                }
                Event::Mouse(mouse) => handle_mouse_event(app, mouse),
                _ => {}
            }
        }
        Ok(())
    }

    pub fn refresh(&mut self) {
        let mut entries = load_directory(&self.current_dir).unwrap_or_default();
        if !self.settings.show_hidden {
            entries.retain(|entry| !entry.name.starts_with('.'));
        }
        sort_entries(&mut entries, self.sort_mode);
        self.entries = entries;
        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
        self.hovered_entry.set(None);
        self.status = format!("Browsing {}", self.current_dir.display());
    }

    pub fn move_selection(&mut self, delta: isize) {
        if self.entries.is_empty() {
            return;
        }
        let current = self.selected as isize;
        let next = current + delta;
        self.selected = next.clamp(0, self.entries.len() as isize - 1) as usize;
    }

    pub fn select_index(&mut self, index: usize) {
        if self.entries.is_empty() {
            return;
        }
        self.selected = index.min(self.entries.len().saturating_sub(1));
    }

    pub fn select_visible_index(&mut self, index: usize) {
        self.select_index(self.visible_start.get().saturating_add(index));
    }

    fn entry_at_position(&self, column: u16, row: u16) -> Option<usize> {
        let area = self.list_area.get();
        let content_top = area.y.saturating_add(2);
        let content_bottom = area.y.saturating_add(area.height).saturating_sub(1);
        if column < area.x
            || column >= area.x.saturating_add(area.width)
            || row < content_top
            || row >= content_bottom
        {
            return None;
        }

        let index = self.visible_start.get().saturating_add((row - content_top) as usize);
        (index < self.entries.len()).then_some(index)
    }

    pub fn pointer_move(&mut self, column: u16, row: u16) {
        let hovered = self.entry_at_position(column, row);
        if self.hovered_entry.replace(hovered) != hovered {
            self.status = hovered
                .map(|index| {
                    let entry = &self.entries[index];
                    let kind = if entry.path.is_dir() { "Directory" } else { "File" };
                    format!("{kind}: {}", entry.display_name)
                })
                .unwrap_or_else(|| format!("Browsing {}", self.current_dir.display()));
        }
    }

    pub fn pointer_click(&mut self, column: u16, row: u16) {
        let Some(index) = self.entry_at_position(column, row) else {
            return;
        };
        self.select_index(index);
        self.open_selected();
    }

    pub fn pointer_context_menu(&mut self, column: u16, row: u16) {
        if let Some(index) = self.entry_at_position(column, row) {
            self.select_index(index);
            self.hovered_entry.set(Some(index));
            self.toggle_context_menu();
        }
    }

    pub fn pointer_scroll(&mut self, column: u16, row: u16, delta: isize) {
        let area = self.list_area.get();
        if column >= area.x
            && column < area.x.saturating_add(area.width)
            && row >= area.y
            && row < area.y.saturating_add(area.height)
        {
            self.move_selection(delta);
            self.pointer_move(column, row);
        }
    }

    pub fn pointer_breadcrumb(&mut self, column: u16, row: u16) -> bool {
        let area = self.breadcrumb_area.get();
        if row != area.y || column < area.x || column >= area.x.saturating_add(area.width) {
            return false;
        }

        let components = self
            .current_dir
            .strip_prefix(&self.home_dir)
            .map(|path| {
                path.components()
                    .map(|component| component.as_os_str().to_os_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let username = self
            .home_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let mut segment_end = area.x.saturating_add(
            unicode_width::UnicodeWidthStr::width(username.as_str()) as u16,
        );
        if column <= segment_end {
            self.navigate_to(self.home_dir.clone());
            return true;
        }

        let mut target = self.home_dir.clone();
        for component in components {
            target.push(&component);
            segment_end = segment_end.saturating_add(
                1 + unicode_width::UnicodeWidthStr::width(component.to_string_lossy().as_ref()) as u16,
            );
            if column <= segment_end {
                self.navigate_to(target);
                return true;
            }
        }
        false
    }

    pub fn pointer_place(&mut self, column: u16, row: u16) -> bool {
        let area = self.places_area.get();
        let content_top = area.y.saturating_add(1);
        if column < area.x
            || column >= area.x.saturating_add(area.width)
            || row < content_top
            || row >= area.y.saturating_add(area.height).saturating_sub(1)
        {
            return false;
        }

        let index = (row - content_top) as usize;
        let places = self.places();
        let Some((label, path)) = places.get(index) else {
            return true;
        };
        if !path.is_dir() {
            self.status = format!("{} is not available", label);
            return true;
        }
        self.navigate_to(path.clone());
        true
    }

    pub fn pointer_toolbar(&mut self, column: u16, row: u16) -> bool {
        let Some(action) = toolbar_action_at(self.toolbar_area.get(), column, row) else {
            return false;
        };
        match action {
            ToolbarAction::New => self.create_directory("new-folder"),
            ToolbarAction::Copy => self.copy_selected(),
            ToolbarAction::Cut => self.cut_selected(),
            ToolbarAction::Paste => self.paste_clipboard(),
            ToolbarAction::Rename => self.rename_selected("renamed-item"),
            ToolbarAction::Delete => self.delete_selected(),
            ToolbarAction::Refresh => self.refresh(),
            ToolbarAction::Sort => self.cycle_sort(),
            ToolbarAction::Hidden => self.toggle_hidden(),
        }
        true
    }

    fn places(&self) -> Vec<(&'static str, PathBuf)> {
        let mut places = vec![("Home", self.home_dir.clone())];
        for (label, folder) in [
            ("Desktop", "Desktop"),
            ("Documents", "Documents"),
            ("Downloads", "Downloads"),
            ("Music", "Music"),
            ("Pictures", "Pictures"),
            ("Videos", "Videos"),
            ("Public", "Public"),
            ("Templates", "Templates"),
        ] {
            places.push((label, self.home_dir.join(folder)));
        }
        places.push(("Trash", self.home_dir.join(".local/share/Trash/files")));
        if self.unrestricted {
            places.push(("File System", PathBuf::from("/")));
        }
        places
    }

    fn navigate_to(&mut self, path: PathBuf) {
        self.navigate_to_with_selection(path, None);
    }

    fn navigate_to_with_selection(&mut self, path: PathBuf, selected_name: Option<&std::ffi::OsStr>) {
        let Ok(path) = path.canonicalize() else {
            self.status = format!("Cannot open {}", path.display());
            return;
        };
        if !path.is_dir() {
            self.status = format!("Cannot enter {}", path.display());
            return;
        }
        if !self.unrestricted && !path.starts_with(&self.home_dir) {
            self.status = "Navigation is limited to your home directory".to_string();
            return;
        }
        self.current_dir = path;
        self.selected = 0;
        self.refresh();
        if let Some(name) = selected_name {
            if let Some(index) = self.entries.iter().position(|entry| entry.path.file_name() == Some(name)) {
                self.selected = index;
            }
        }
    }

    pub fn page_down(&mut self, lines: usize) {
        if self.entries.is_empty() {
            return;
        }
        let delta = lines.max(1) as isize;
        self.move_selection(delta);
    }

    pub fn page_up(&mut self, lines: usize) {
        if self.entries.is_empty() {
            return;
        }
        let delta = -(lines.max(1) as isize);
        self.move_selection(delta);
    }

    pub fn move_to_start(&mut self) {
        self.select_index(0);
    }

    pub fn move_to_end(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        self.selected = self.entries.len() - 1;
    }

    pub fn enter_selected(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let target = self.entries[self.selected].path.clone();
        if target.is_dir() {
            self.navigate_to(target);
        } else {
            self.status = "Selected item is not a directory".to_string();
        }
    }

    pub fn go_parent(&mut self) {
        if self.current_dir == self.home_dir {
            self.status = "Traversal root reached".to_string();
            return;
        }
        let selected_name = self.current_dir.file_name().map(|name| name.to_os_string());
        if let Some(parent) = self.current_dir.parent() {
            if self.unrestricted || parent.starts_with(&self.home_dir) {
                self.navigate_to_with_selection(parent.to_path_buf(), selected_name.as_deref());
            }
        }
    }

    pub fn cycle_sort(&mut self) {
        self.sort_mode = match self.sort_mode {
            SortMode::Name => SortMode::Type,
            SortMode::Type => SortMode::Size,
            SortMode::Size => SortMode::Name,
        };
        self.refresh();
    }

    pub fn toggle_hidden(&mut self) {
        self.settings.show_hidden = !self.settings.show_hidden;
        self.refresh();
        self.status = if self.settings.show_hidden {
            "Showing hidden items".to_string()
        } else {
            "Hiding hidden items".to_string()
        };
    }

    pub fn create_directory(&mut self, name: &str) {
        let target = self.current_dir.join(name);
        if let Err(err) = fs_create_directory(&target) {
            self.status = format!("Create failed: {err}");
            return;
        }
        self.refresh();
        self.status = format!("Created directory {}", target.display());
    }

    pub fn rename_selected(&mut self, new_name: &str) {
        if self.entries.is_empty() {
            return;
        }
        let old_path = self.entries[self.selected].path.clone();
        match rename_path(&old_path, new_name) {
            Ok(_) => {
                self.refresh();
                self.status = format!("Renamed {} to {}", old_path.display(), new_name);
            }
            Err(err) => self.status = format!("Rename failed: {err}"),
        }
    }

    pub fn delete_selected(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let target = self.entries[self.selected].path.clone();
        match delete_path(&target) {
            Ok(_) => {
                self.refresh();
                self.status = format!("Deleted {}", target.display());
            }
            Err(err) => self.status = format!("Delete failed: {err}"),
        }
    }

    pub fn copy_selected(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let source = self.entries[self.selected].path.clone();
        self.clipboard = Some(FileClipboard {
            source: source.clone(),
            mode: ClipboardMode::Copy,
        });
        self.status = format!("Copied {}. Choose a folder and Paste", source.display());
    }

    pub fn cut_selected(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let source = self.entries[self.selected].path.clone();
        self.clipboard = Some(FileClipboard {
            source: source.clone(),
            mode: ClipboardMode::Cut,
        });
        self.status = format!("Cut {}. Choose a folder and Paste", source.display());
    }

    pub fn paste_clipboard(&mut self) {
        let Some(clipboard) = self.clipboard.as_ref() else {
            self.status = "Clipboard is empty".to_string();
            return;
        };
        let source = clipboard.source.clone();
        let mode = clipboard.mode;
        let Some(name) = source.file_name() else {
            self.status = "Cannot paste an item without a filename".to_string();
            return;
        };
        let target = self.current_dir.join(name);
        if target == source {
            self.status = "Source and destination are the same".to_string();
            return;
        }
        if source.is_dir() && self.current_dir.starts_with(&source) {
            self.status = "Cannot paste a directory into itself".to_string();
            return;
        }

        let result = match mode {
            ClipboardMode::Copy => copy_path(&source, &target),
            ClipboardMode::Cut => move_path(&source, &target),
        };
        match result {
            Ok(()) => {
                if mode == ClipboardMode::Cut {
                    self.clipboard = None;
                }
                self.refresh();
                self.status = format!("Pasted {}", target.display());
            }
            Err(error) => self.status = format!("Paste failed: {error}"),
        }
    }

    pub fn open_selected(&mut self) {
        if self.entries.is_empty() {
            self.status = "No item selected to open".to_string();
            return;
        }

        let target = self.entries[self.selected].path.clone();
        if target.is_dir() {
            self.enter_selected();
            self.status = format!("Opened directory {}", target.display());
            return;
        }
        if self.entries[self.selected].is_symlink {
            self.status = format!("Not opening symbolic link {}", target.display());
            return;
        }

        let result = crate::opener::launch_file_with_terminal(&target);
        self.screen_invalidated = true;
        match result {
            Ok(_) => self.status = format!("Opened {}", target.display()),
            Err(err) => self.status = format!("Open failed: {err}"),
        }
    }

    pub fn toggle_context_menu(&mut self) {
        self.context_menu_open = !self.context_menu_open;
        if self.context_menu_open {
            self.context_menu_index = 0;
            self.dialog_message = None;
        }
    }

    pub fn is_context_menu_open(&self) -> bool {
        self.context_menu_open
    }

    pub fn close_context_menu(&mut self) {
        self.context_menu_open = false;
        self.dialog_message = None;
    }

    pub fn move_context_menu(&mut self, delta: isize) {
        if !self.context_menu_open || self.context_menu_items.is_empty() {
            return;
        }
        let next = (self.context_menu_index as isize + delta).clamp(0, self.context_menu_items.len() as isize - 1) as usize;
        self.context_menu_index = next;
    }

    pub fn apply_context_menu(&mut self) {
        if !self.context_menu_open {
            return;
        }
        let action = self.context_menu_items.get(self.context_menu_index).copied().unwrap_or("Open");
        match action {
            "Open" => {
                self.open_selected();
            }
            "Rename" => {
                self.rename_selected("renamed-item");
            }
            "Delete" => {
                self.delete_selected();
            }
            "Copy" => {
                self.copy_selected();
            }
            "Cut" => {
                self.cut_selected();
            }
            "Paste" => {
                self.paste_clipboard();
            }
            "Properties" => {
                let target = if self.entries.is_empty() { "No item selected".to_string() } else { format!("{} [{}]", self.entries[self.selected].name, self.entries[self.selected].kind.indicator()) };
                self.dialog_message = Some(format!("Properties: {target}"));
            }
            _ => {}
        }
        self.close_context_menu();
    }

    fn render(&self, frame: &mut ratatui::Frame<'_>) {
        let _ = &self.settings;
        let area = frame.area();
        let outer = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Min(5), Constraint::Length(4)])
            .split(area);

        let header_rows = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(1)])
            .split(outer[0]);

        let header = Paragraph::new(Line::from(vec![
            Span::styled("TFSV - Terminal File System Visualiser", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::raw(format!("by github.com/aayushrambo8")),
        ]))
        .block(Block::default().borders(Borders::ALL).title("tfsv"));
        frame.render_widget(header, header_rows[0]);

        self.toolbar_area.set(header_rows[1]);
        let mut toolbar_spans = Vec::new();
        for (index, action) in ToolbarAction::ALL.into_iter().enumerate() {
            if index > 0 {
                toolbar_spans.push(Span::raw(" "));
            }
            toolbar_spans.push(Span::styled(
                format!("[{}]", action.label()),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ));
        }
        frame.render_widget(Paragraph::new(Line::from(toolbar_spans)), header_rows[1]);

        let body = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(outer[1]);

        self.places_area.set(body[0]);
        let places = self.places();
        let sidebar = List::new(places.iter().map(|(label, path)| {
            let style = if !path.is_dir() {
                Style::default().fg(Color::DarkGray)
            } else if self.current_dir == *path {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!(" {}", label)).style(style)
        }))
        .block(Block::default().title("PLACES").borders(Borders::ALL));
        frame.render_widget(sidebar, body[0]);

        let file_pane = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(3)])
            .split(body[1]);
        self.breadcrumb_area.set(file_pane[0]);
        let breadcrumb = self.breadcrumb_text();
        frame.render_widget(Paragraph::new(breadcrumb), file_pane[0]);

        let visible_rows = file_pane[1].height.saturating_sub(3) as usize;
        self.list_area.set(file_pane[1]);
        let (first_visible, end_visible) = visible_entry_bounds(
            self.entries.len(),
            self.selected,
            visible_rows,
        );
        self.visible_start.set(first_visible);
        let rows = self.entries.iter().enumerate().skip(first_visible).take(end_visible - first_visible).map(|(index, entry)| {
            let selected = index == self.selected;
            let hovered = self.hovered_entry.get() == Some(index);
            let mut style = Style::default();
            if selected {
                style = style.add_modifier(Modifier::REVERSED);
            } else if hovered {
                style = style.fg(Color::Black).bg(Color::Cyan);
            }
            let name = if selected { format!("> {}", entry.display_name) } else { entry.display_name.clone() };
            Row::new(vec![
                entry.extension_badge(),
                name,
                if entry.path.is_dir() { "Directory".to_string() } else { "File".to_string() },
                format_size(entry.size),
            ]).style(style)
        }).collect::<Vec<_>>();

        let table = Table::new(
            rows,
            [Constraint::Length(12), Constraint::Min(20), Constraint::Length(12), Constraint::Length(10)],
        )
        .header(Row::new(vec!["TYPE", "NAME", "CLASS", "SIZE"]).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(table, file_pane[1]);

        let footer = Paragraph::new(vec![
            Line::from(self.status.as_str()),
            Line::from("Enter/l open  h/Left up  . hidden  Home/End jump  PgUp/PgDn page  Ctrl+C copy  Ctrl+X cut  Ctrl+V paste  Wheel scroll  c/Right-click menu  q quit"),
        ])
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(footer, outer[2]);

        if self.context_menu_open {
            let modal = context_menu_rect(
                area,
                self.list_area.get(),
                self.selected,
                self.visible_start.get(),
                self.context_menu_items.len(),
            );
            let menu = Paragraph::new(
                self.context_menu_items
                    .iter()
                    .enumerate()
                    .map(|(idx, item)| {
                        let prefix = if idx == self.context_menu_index { ">" } else { " " };
                        format!("{prefix} {item}")
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
            .block(Block::default().title("Context Menu").borders(Borders::ALL));
            frame.render_widget(Clear, modal);
            frame.render_widget(menu, modal);
        }

        if let Some(message) = &self.dialog_message {
            let dialog = Rect {
                x: area.x + 2,
                y: area.y + area.height / 2,
                width: area.width.saturating_sub(4),
                height: 3,
            };
            let msg = Paragraph::new(message.as_str())
                .block(Block::default().title("Dialog").borders(Borders::ALL));
            frame.render_widget(msg, dialog);
        }
    }

    fn breadcrumb_text(&self) -> String {
        if self.unrestricted {
            let mut path = self.current_dir.to_string_lossy().into_owned();
            if !path.ends_with('/') {
                path.push('/');
            }
            return path;
        }
        let username = self.home_dir.file_name().unwrap_or_default().to_string_lossy();
        let relative = self
            .current_dir
            .strip_prefix(&self.home_dir)
            .unwrap_or_else(|_| std::path::Path::new(""));
        let mut breadcrumb = username.into_owned();
        for component in relative.components() {
            breadcrumb.push('/');
            breadcrumb.push_str(&component.as_os_str().to_string_lossy());
        }
        breadcrumb.push('/');
        breadcrumb
    }
}

fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit_index = 0;
    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", value, UNITS[unit_index])
    }
}

fn visible_entry_bounds(entry_count: usize, selected: usize, visible_rows: usize) -> (usize, usize) {
    let first = selected
        .saturating_sub(visible_rows.saturating_sub(1))
        .min(entry_count.saturating_sub(visible_rows));
    (first, (first + visible_rows).min(entry_count))
}

fn toolbar_action_at(area: Rect, column: u16, row: u16) -> Option<ToolbarAction> {
    if row != area.y || column < area.x || column >= area.x.saturating_add(area.width) {
        return None;
    }
    let mut button_start = area.x;
    for (index, action) in ToolbarAction::ALL.into_iter().enumerate() {
        let button_end = button_start.saturating_add(action.label().len() as u16 + 2);
        if column >= button_start && column < button_end {
            return Some(action);
        }
        button_start = button_end.saturating_add(1);
        if index + 1 == ToolbarAction::ALL.len() {
            break;
        }
    }
    None
}

#[cfg(test)]
fn toolbar_button_column(action: ToolbarAction) -> u16 {
    let mut column = 0u16;
    for candidate in ToolbarAction::ALL {
        if candidate == action {
            return column;
        }
        column = column.saturating_add(candidate.label().len() as u16 + 3);
    }
    column
}

fn resolve_start_directory(
    root: &std::path::Path,
    launch_dir: Option<PathBuf>,
    configured_start: &std::path::Path,
    unrestricted: bool,
) -> PathBuf {
    let is_allowed_directory = |path: &std::path::Path| {
        path.canonicalize()
            .ok()
            .filter(|resolved| resolved.is_dir() && (unrestricted || resolved.starts_with(root)))
    };

    launch_dir
        .as_deref()
        .and_then(is_allowed_directory)
        .or_else(|| is_allowed_directory(configured_start))
        .unwrap_or_else(|| root.to_path_buf())
}

fn context_menu_rect(area: Rect, list_area: Rect, selected: usize, visible_start: usize, item_count: usize) -> Rect {
    let width = area.width.min(24);
    let height = area.height.min(item_count.saturating_add(2) as u16);
    let max_x = area.x.saturating_add(area.width).saturating_sub(width);
    let max_y = area.y.saturating_add(area.height).saturating_sub(height);
    let x = list_area.x.saturating_add(4).clamp(area.x, max_x);
    let selected_row = list_area
        .y
        .saturating_add(2)
        .saturating_add(selected.saturating_sub(visible_start) as u16);
    let y = if selected_row.saturating_add(height) > area.y.saturating_add(area.height) {
        selected_row.saturating_sub(height.saturating_sub(1))
    } else {
        selected_row
    }
    .clamp(area.y, max_y);
    Rect { x, y, width, height }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_navigation_moves_selection() {
        let mut app = App {
            settings: Settings::default(),
            home_dir: PathBuf::from("/tmp"),
            unrestricted: false,
            current_dir: std::path::PathBuf::from("/tmp"),
            entries: vec![
                FileEntry::from_path(std::path::PathBuf::from("/tmp/a")),
                FileEntry::from_path(std::path::PathBuf::from("/tmp/b")),
                FileEntry::from_path(std::path::PathBuf::from("/tmp/c")),
                FileEntry::from_path(std::path::PathBuf::from("/tmp/d")),
                FileEntry::from_path(std::path::PathBuf::from("/tmp/e")),
            ],
            selected: 0,
            sort_mode: SortMode::Name,
            status: String::new(),
            context_menu_open: false,
            context_menu_index: 0,
            context_menu_items: vec!["Open", "Copy", "Cut", "Paste", "Rename", "Delete", "Properties"],
            clipboard: None,
            dialog_message: None,
            visible_start: Cell::new(0),
            toolbar_area: Cell::new(Rect::default()),
            places_area: Cell::new(Rect::default()),
            list_area: Cell::new(Rect::default()),
            breadcrumb_area: Cell::new(Rect::default()),
            hovered_entry: Cell::new(None),
            screen_invalidated: false,
        };

        app.page_down(2);
        assert_eq!(app.selected, 2);
        app.page_up(1);
        assert_eq!(app.selected, 1);
        app.move_to_end();
        assert_eq!(app.selected, app.entries.len() - 1);
        app.move_to_start();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn mouse_selection_clamps_to_valid_index() {
        let mut app = App {
            settings: Settings::default(),
            home_dir: PathBuf::from("/tmp"),
            unrestricted: false,
            current_dir: std::path::PathBuf::from("/tmp"),
            entries: vec![
                FileEntry::from_path(std::path::PathBuf::from("/tmp/a")),
                FileEntry::from_path(std::path::PathBuf::from("/tmp/b")),
                FileEntry::from_path(std::path::PathBuf::from("/tmp/c")),
            ],
            selected: 0,
            sort_mode: SortMode::Name,
            status: String::new(),
            context_menu_open: false,
            context_menu_index: 0,
            context_menu_items: vec!["Open", "Copy", "Cut", "Paste", "Rename", "Delete", "Properties"],
            clipboard: None,
            dialog_message: None,
            visible_start: Cell::new(0),
            toolbar_area: Cell::new(Rect::default()),
            places_area: Cell::new(Rect::default()),
            list_area: Cell::new(Rect::default()),
            breadcrumb_area: Cell::new(Rect::default()),
            hovered_entry: Cell::new(None),
            screen_invalidated: false,
        };

        app.select_index(99);
        assert_eq!(app.selected, 2);
    }

    #[test]
    fn context_menu_toggles_and_moves_selection() {
        let mut app = App {
            settings: Settings::default(),
            home_dir: PathBuf::from("/tmp"),
            unrestricted: false,
            current_dir: std::path::PathBuf::from("/tmp"),
            entries: vec![
                FileEntry::from_path(std::path::PathBuf::from("/tmp/a")),
                FileEntry::from_path(std::path::PathBuf::from("/tmp/b")),
            ],
            selected: 0,
            sort_mode: SortMode::Name,
            status: String::new(),
            context_menu_open: false,
            context_menu_index: 0,
            context_menu_items: vec!["Open", "Copy", "Cut", "Paste", "Rename", "Delete", "Properties"],
            clipboard: None,
            dialog_message: None,
            visible_start: Cell::new(0),
            toolbar_area: Cell::new(Rect::default()),
            places_area: Cell::new(Rect::default()),
            list_area: Cell::new(Rect::default()),
            breadcrumb_area: Cell::new(Rect::default()),
            hovered_entry: Cell::new(None),
            screen_invalidated: false,
        };

        app.toggle_context_menu();
        assert!(app.context_menu_open);
        app.move_context_menu(2);
        assert_eq!(app.context_menu_index, 2);
        app.close_context_menu();
        assert!(!app.context_menu_open);
    }

    #[test]
    fn clipboard_copy_and_cut_paste_files_into_current_directory() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("tfsv-clipboard-{}-{unique}", std::process::id()));
        let destination = root.join("destination");
        std::fs::create_dir_all(&destination).unwrap();
        let copy_source = root.join("copy.txt");
        let cut_source = root.join("cut.txt");
        std::fs::write(&copy_source, "copy data").unwrap();
        std::fs::write(&cut_source, "cut data").unwrap();

        let mut app = App {
            settings: Settings::default(),
            home_dir: root.clone(),
            unrestricted: false,
            current_dir: root.clone(),
            entries: vec![FileEntry::from_path(copy_source.clone())],
            selected: 0,
            sort_mode: SortMode::Name,
            status: String::new(),
            context_menu_open: false,
            context_menu_index: 0,
            context_menu_items: vec!["Open", "Copy", "Cut", "Paste", "Rename", "Delete", "Properties"],
            clipboard: None,
            dialog_message: None,
            visible_start: Cell::new(0),
            toolbar_area: Cell::new(Rect::default()),
            places_area: Cell::new(Rect::default()),
            list_area: Cell::new(Rect::default()),
            breadcrumb_area: Cell::new(Rect::default()),
            hovered_entry: Cell::new(None),
            screen_invalidated: false,
        };

        app.copy_selected();
        app.current_dir = destination.clone();
        app.paste_clipboard();
        assert!(copy_source.exists());
        assert_eq!(std::fs::read(destination.join("copy.txt")).unwrap(), b"copy data");

        app.current_dir = root.clone();
        app.entries = vec![FileEntry::from_path(cut_source.clone())];
        app.selected = 0;
        app.cut_selected();
        app.current_dir = destination.clone();
        app.paste_clipboard();
        assert!(!cut_source.exists());
        assert_eq!(std::fs::read(destination.join("cut.txt")).unwrap(), b"cut data");
        assert!(app.clipboard.is_none());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn enter_opens_selected_directory_and_h_returns_to_parent() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("tfsv-navigation-{}-{unique}", std::process::id()));
        let nested = root.join("nested");
        let hidden = root.join(".hidden");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(&hidden).unwrap();

        let mut app = App {
            settings: Settings::default(),
            home_dir: root.clone(),
            unrestricted: false,
            current_dir: root.clone(),
            entries: Vec::new(),
            selected: 0,
            sort_mode: SortMode::Name,
            status: String::new(),
            context_menu_open: false,
            context_menu_index: 0,
            context_menu_items: vec!["Open", "Copy", "Cut", "Paste", "Rename", "Delete", "Properties"],
            clipboard: None,
            dialog_message: None,
            visible_start: Cell::new(0),
            toolbar_area: Cell::new(Rect::default()),
            places_area: Cell::new(Rect::default()),
            list_area: Cell::new(Rect::default()),
            breadcrumb_area: Cell::new(Rect::default()),
            hovered_entry: Cell::new(None),
            screen_invalidated: false,
        };
        app.refresh();

        assert!(!handle_key_event(
            &mut app,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        )
        .unwrap());
        assert_eq!(app.current_dir, nested);

        handle_key_event(
            &mut app,
            KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
        )
        .unwrap();
        assert_eq!(app.current_dir, root);
        assert_eq!(app.entries[app.selected].path, nested);
        assert!(!app.entries.iter().any(|entry| entry.path == hidden));
        app.toggle_hidden();
        assert!(app.entries.iter().any(|entry| entry.path == hidden));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn visible_entry_bounds_follow_selection_without_exceeding_list() {
        assert_eq!(visible_entry_bounds(100, 50, 10), (41, 51));
        assert_eq!(visible_entry_bounds(5, 4, 10), (0, 5));
        assert_eq!(visible_entry_bounds(0, 0, 10), (0, 0));
    }

    #[test]
    fn context_menu_is_anchored_near_selection_and_clamped_to_screen() {
        let screen = Rect::new(0, 0, 80, 24);
        let list = Rect::new(20, 3, 60, 17);
        let menu = context_menu_rect(screen, list, 5, 2, 5);
        assert_eq!(menu.x, 24);
        assert_eq!(menu.y, 8);
        assert_eq!(menu.width, 24);
        assert_eq!(menu.height, 7);

        let near_bottom = context_menu_rect(screen, list, 18, 2, 5);
        assert!(near_bottom.y + near_bottom.height <= screen.height);
    }

    #[test]
    fn toolbar_actions_have_mouse_hit_targets_on_the_toolbar_row() {
        let area = Rect::new(4, 3, 100, 1);
        for action in ToolbarAction::ALL {
            let column = area.x + toolbar_button_column(action);
            assert_eq!(toolbar_action_at(area, column, area.y), Some(action));
            assert_eq!(toolbar_action_at(area, column, area.y + 1), None);
        }
        assert_eq!(toolbar_action_at(area, area.x + area.width, area.y), None);
    }

    #[test]
    fn startup_prefers_launch_directory_and_allows_sudo_paths() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("tfsv-start-{}-{unique}", std::process::id()));
        let home = base.join("home");
        let launch = home.join("work");
        let outside = base.join("outside");
        std::fs::create_dir_all(&launch).unwrap();
        std::fs::create_dir_all(&outside).unwrap();

        assert_eq!(
            resolve_start_directory(&home, Some(launch.clone()), &home, false),
            launch
        );
        assert_eq!(
            resolve_start_directory(&home, Some(outside.clone()), &home, false),
            home
        );
        assert_eq!(
            resolve_start_directory(&home, Some(outside.clone()), &home, true),
            outside
        );

        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn navigation_and_breadcrumb_are_confined_to_home() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let home = std::env::temp_dir().join(format!("tfsv-home-{}-{unique}", std::process::id()));
        let child = home.join("child");
        let nested = child.join("nested");
        let documents = home.join("Documents");
        std::fs::create_dir_all(&documents).unwrap();
        std::fs::create_dir_all(&nested).unwrap();

        let mut app = App {
            settings: Settings::default(),
            home_dir: home.clone(),
            unrestricted: false,
            current_dir: nested.clone(),
            entries: Vec::new(),
            selected: 0,
            sort_mode: SortMode::Name,
            status: String::new(),
            context_menu_open: false,
            context_menu_index: 0,
            context_menu_items: vec!["Open", "Copy", "Cut", "Paste", "Rename", "Delete", "Properties"],
            clipboard: None,
            dialog_message: None,
            visible_start: Cell::new(0),
            toolbar_area: Cell::new(Rect::default()),
            places_area: Cell::new(Rect::new(0, 0, 20, 20)),
            list_area: Cell::new(Rect::default()),
            breadcrumb_area: Cell::new(Rect::new(5, 3, 60, 1)),
            hovered_entry: Cell::new(None),
            screen_invalidated: false,
        };

        assert_eq!(app.breadcrumb_text(), format!("{}/child/nested/", home.file_name().unwrap().to_string_lossy()));
        let place_names = app.places().into_iter().map(|(name, _)| name).collect::<Vec<_>>();
        assert!(place_names.contains(&"Desktop"));
        assert!(place_names.contains(&"Documents"));
        assert!(place_names.contains(&"Downloads"));
        assert!(place_names.contains(&"Music"));
        assert!(place_names.contains(&"Pictures"));
        assert!(place_names.contains(&"Videos"));
        app.go_parent();
        assert_eq!(app.current_dir, child);
        let username_width = home.file_name().unwrap().to_string_lossy().chars().count() as u16;
        app.pointer_breadcrumb(5 + username_width + 1 + 2, 3);
        assert_eq!(app.current_dir, child);
        app.pointer_breadcrumb(5, 3);
        assert_eq!(app.current_dir, home);
        assert!(app.pointer_place(1, 3));
        assert_eq!(app.current_dir, documents);
        app.go_parent();
        assert_eq!(app.current_dir, home);

        #[cfg(unix)]
        {
            let outside = home.parent().unwrap().join(format!("tfsv-outside-{unique}"));
            std::fs::create_dir_all(&outside).unwrap();
            let escape = home.join("outside-link");
            std::os::unix::fs::symlink(&outside, &escape).unwrap();
            app.entries = vec![FileEntry::from_path(escape)];
            app.selected = 0;
            app.enter_selected();
            assert_eq!(app.current_dir, home);
            std::fs::remove_dir_all(outside).unwrap();
        }

        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn mouse_click_opens_selected_directory()
    {
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("tfsv-pointer-{}-{unique}", std::process::id()));
        let nested = root.join("nested");
        std::fs::create_dir_all(&nested).unwrap();

        let mut app = App {
            settings: Settings::default(),
            home_dir: root.clone(),
            unrestricted: false,
            current_dir: root.clone(),
            entries: vec![FileEntry::from_path(nested.clone())],
            selected: 0,
            sort_mode: SortMode::Name,
            status: String::new(),
            context_menu_open: false,
            context_menu_index: 0,
            context_menu_items: vec!["Open", "Copy", "Cut", "Paste", "Rename", "Delete", "Properties"],
            clipboard: None,
            dialog_message: None,
            visible_start: Cell::new(0),
            toolbar_area: Cell::new(Rect::default()),
            places_area: Cell::new(Rect::default()),
            list_area: Cell::new(Rect::new(10, 2, 40, 8)),
            breadcrumb_area: Cell::new(Rect::default()),
            hovered_entry: Cell::new(None),
            screen_invalidated: false,
        };

        app.pointer_move(11, 4);
        assert_eq!(app.hovered_entry.get(), Some(0));
        assert!(app.status.contains("nested"));

        app.pointer_click(11, 4);
        assert_eq!(app.current_dir, nested);

        std::fs::remove_dir_all(root).unwrap();
    }
}
