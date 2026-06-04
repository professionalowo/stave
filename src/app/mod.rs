use std::{sync::mpsc, time::Duration};

use crate::{
    app::theme::Theme,
    brew::{
        bindings::{BrewList, Info, InfoEntry},
        worker::{Command, ListOption, Response},
    },
};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::Stylize,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

mod theme;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum InstalledFilter {
    All,
    Formula,
    Cask,
}

#[derive(Debug, Clone)]
struct InstalledItem {
    name: String,
    version: String,
    kind: &'static str,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ActiveView {
    Installed,
    Outdated,
}

#[derive(Debug)]
pub struct App {
    should_quit: bool,
    command_tx: mpsc::Sender<Command>,
    output_rx: mpsc::Receiver<Response>,
    theme: Theme,
    active_view: ActiveView,
    filter: InstalledFilter,
    items: Vec<InstalledItem>,
    list_state: ListState,
    status_line: String,
    search_mode: bool,
    search_query: String,
    show_info_popup: bool,
    info_popup_text: String,
}

impl App {
    pub fn new(command_tx: mpsc::Sender<Command>, output_rx: mpsc::Receiver<Response>) -> Self {
        let mut app = Self {
            should_quit: false,
            command_tx,
            output_rx,
            theme: Theme::load(terminal_supports_color()),
            active_view: ActiveView::Installed,
            filter: InstalledFilter::All,
            items: Vec::new(),
            list_state: ListState::default().with_selected(Some(0)),
            status_line: String::from("Loading installed packages..."),
            search_mode: false,
            search_query: String::new(),
            show_info_popup: false,
            info_popup_text: String::new(),
        };
        app.request_list();
        app
    }

    pub fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            self.read_worker_messages();
            terminal.draw(|frame| self.render(frame))?;

            if event::poll(Duration::from_millis(100))?
                && let Event::Key(e) = event::read()?
            {
                self.handle_key_event(e);
            }
        }
        Ok(())
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) {
        if self.show_info_popup {
            match event.code {
                KeyCode::Char('i') | KeyCode::Esc => self.show_info_popup = false,
                KeyCode::Char('q') => self.should_quit = true,
                _ => {}
            }
            return;
        }

        if self.search_mode {
            match event.code {
                KeyCode::Enter | KeyCode::Esc => self.search_mode = false,
                KeyCode::Char('j') | KeyCode::Down => self.select_next(),
                KeyCode::Char('k') | KeyCode::Up => self.select_previous(),
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.clamp_selection_to_visible_items();
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    self.clamp_selection_to_visible_items();
                }
                _ => {}
            }

            let visible_count = self.visible_item_indices().len();
            self.status_line = format!(
                "Search: '{}' ({} results)",
                self.search_query, visible_count
            );
            return;
        }

        match event.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => self.select_next(),
            KeyCode::Char('k') | KeyCode::Up => self.select_previous(),
            KeyCode::Char('/') => {
                self.search_mode = true;
                self.status_line = String::from("Search mode: type to filter, Enter/Esc to finish");
            }
            KeyCode::Char('1') => {
                self.active_view = ActiveView::Installed;
                self.filter = InstalledFilter::All;
                self.request_list();
            }
            KeyCode::Char('2') => {
                self.active_view = ActiveView::Installed;
                self.filter = InstalledFilter::Formula;
                self.request_list();
            }
            KeyCode::Char('3') => {
                self.active_view = ActiveView::Installed;
                self.filter = InstalledFilter::Cask;
                self.request_list();
            }
            KeyCode::Char('r') => self.request_list(),
            KeyCode::Char('o') => {
                self.active_view = ActiveView::Outdated;
                self.request_outdated();
            }
            KeyCode::Char('u') => self.request_update(),
            KeyCode::Char('g') => self.request_upgrade(),
            KeyCode::Char('G') => self.request_upgrade_selected(),
            KeyCode::Char('i') => {
                if self.show_info_popup {
                    self.show_info_popup = false;
                } else {
                    self.request_info_for_selected();
                }
            }
            KeyCode::Esc => self.show_info_popup = false,
            _ => {}
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let [header_area, content_area, help_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(frame.area());

        let header = Paragraph::new("Stave")
            .centered()
            .bold()
            .style(self.theme.header_style())
            .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(header, header_area);

        let [left_area, right_area] =
            Layout::horizontal([Constraint::Length(28), Constraint::Min(0)]).areas(content_area);

        let menu_items = vec![
            ListItem::new(sidebar_label(
                "1",
                "Installed (All)",
                self.active_view == ActiveView::Installed && self.filter == InstalledFilter::All,
            ))
            .style(self.theme.sidebar_style(
                self.active_view == ActiveView::Installed && self.filter == InstalledFilter::All,
            )),
            ListItem::new(sidebar_label(
                "2",
                "Installed (Formulae)",
                self.active_view == ActiveView::Installed
                    && self.filter == InstalledFilter::Formula,
            ))
            .style(self.theme.sidebar_style(
                self.active_view == ActiveView::Installed
                    && self.filter == InstalledFilter::Formula,
            )),
            ListItem::new(sidebar_label(
                "3",
                "Installed (Casks)",
                self.active_view == ActiveView::Installed && self.filter == InstalledFilter::Cask,
            ))
            .style(self.theme.sidebar_style(
                self.active_view == ActiveView::Installed && self.filter == InstalledFilter::Cask,
            )),
            ListItem::new(" "),
            ListItem::new(sidebar_label(
                "o",
                "Outdated",
                self.active_view == ActiveView::Outdated,
            ))
            .style(
                self.theme
                    .sidebar_style(self.active_view == ActiveView::Outdated),
            ),
            ListItem::new("u Update").style(self.theme.sidebar_style(false)),
            ListItem::new("g Upgrade").style(self.theme.sidebar_style(false)),
            ListItem::new("G Upgrade selected").style(self.theme.sidebar_style(false)),
        ];
        let menu = List::new(menu_items).block(
            Block::default()
                .title("Views / Functions")
                .borders(Borders::ALL),
        );
        frame.render_widget(menu, left_area);

        let [search_area, list_area] =
            Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(right_area);

        let search_hint = if self.search_mode {
            format!("/{}", self.search_query)
        } else if self.search_query.is_empty() {
            String::from("Press / to search")
        } else {
            format!("Filter: {}", self.search_query)
        };

        let search = Paragraph::new(search_hint).block(
            Block::default()
                .title("Search")
                .style(self.theme.accent_style())
                .borders(Borders::ALL),
        );
        frame.render_widget(search, search_area);

        let visible_indices = self.visible_item_indices();

        let rows: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .filter(|(idx, _)| visible_indices.contains(idx))
            .map(|(_, item)| {
                ListItem::new(format!(
                    "{:<28} {:<5} {}",
                    item.name, item.kind, item.version
                ))
            })
            .collect();

        let rows = if rows.is_empty() {
            vec![ListItem::new("No matching packages".dim())]
        } else {
            rows
        };

        let list = List::new(rows)
            .block(
                Block::default()
                    .title(format!(
                        "{} ({}/{})",
                        self.current_list_title(),
                        visible_indices.len(),
                        self.items.len()
                    ))
                    .title_bottom(self.status_line.clone())
                    .borders(Borders::ALL),
            )
            .highlight_style(self.theme.selection_style())
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, list_area, &mut self.list_state);

        let help = Paragraph::new(
            "q: Quit  j/k or Arrows: Move  i: Info Popup  /: Search  Enter/Esc: End Search  1/2/3: Installed Filters  o: Outdated  u: Update  g: Upgrade all  G: Upgrade selected  r: Refresh",
        )
            .style(self.theme.accent_style());
        frame.render_widget(help, help_area);

        if self.show_info_popup {
            let popup_area = centered_rect(80, 70, frame.area());
            frame.render_widget(Clear, popup_area);
            let popup = Paragraph::new(self.info_popup_text.clone())
                .block(Block::default().title("brew info").borders(Borders::ALL))
                .wrap(Wrap { trim: true });
            frame.render_widget(popup, popup_area);
        }
    }

    fn request_list(&mut self) {
        let option = match self.filter {
            InstalledFilter::All => ListOption::All,
            InstalledFilter::Formula => ListOption::Formula,
            InstalledFilter::Cask => ListOption::Cask,
        };

        if self.command_tx.send(Command::List(option)).is_err() {
            self.status_line = String::from("Failed to send list command to worker");
        } else {
            self.status_line = String::from("Loading installed packages...");
        }
    }

    fn request_outdated(&mut self) {
        if self.command_tx.send(Command::Outdated).is_err() {
            self.status_line = String::from("Failed to send outdated command to worker");
        } else {
            self.status_line = String::from("Loading outdated packages...");
        }
    }

    fn request_update(&mut self) {
        if self.command_tx.send(Command::Update).is_err() {
            self.status_line = String::from("Failed to send update command to worker");
        } else {
            self.status_line = String::from("Running brew update...");
        }
    }

    fn request_upgrade(&mut self) {
        if self.command_tx.send(Command::Upgrade).is_err() {
            self.status_line = String::from("Failed to send upgrade command to worker");
        } else {
            self.status_line = String::from("Running brew upgrade...");
        }
    }

    fn request_upgrade_selected(&mut self) {
        let Some(item) = self.selected_item() else {
            self.status_line = String::from("No package selected");
            return;
        };

        let name = item.name.clone();
        let is_cask = item.kind == "C";

        if self
            .command_tx
            .send(Command::UpgradePackage {
                name: name.clone(),
                is_cask,
            })
            .is_err()
        {
            self.status_line = String::from("Failed to send package upgrade command to worker");
        } else {
            self.status_line = format!("Upgrading {}...", name);
        }
    }

    fn request_info_for_selected(&mut self) {
        if let Some(item) = self.selected_item() {
            if self
                .command_tx
                .send(Command::Info(item.name.clone()))
                .is_err()
            {
                self.status_line = String::from("Failed to send info command to worker");
            } else {
                self.status_line = format!("Loading info for {}...", item.name);
            }
        } else {
            self.status_line = String::from("No package selected");
        }
    }

    fn read_worker_messages(&mut self) {
        loop {
            match self.output_rx.try_recv() {
                Ok(Response::List(list)) => self.apply_list(list),
                Ok(Response::Info(info)) => {
                    self.info_popup_text = format_info(info);
                    self.show_info_popup = true;
                }
                Ok(Response::Outdated(output)) => {
                    self.apply_outdated_output(output);
                }
                Ok(Response::UpdateResult(output)) => {
                    self.info_popup_text = output;
                    self.show_info_popup = true;
                    self.status_line = String::from("brew update finished");
                }
                Ok(Response::UpgradeResult(output)) => {
                    self.info_popup_text = output;
                    self.show_info_popup = true;
                    self.status_line = String::from("brew upgrade finished");
                    self.refresh_active_view();
                }
                Ok(Response::UpgradePackageResult { name, output }) => {
                    self.info_popup_text = output;
                    self.show_info_popup = true;
                    self.status_line = format!("brew upgrade finished for {}", name);
                    self.refresh_active_view();
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.status_line = String::from("Worker disconnected");
                    break;
                }
            }
        }
    }

    fn apply_list(&mut self, list: BrewList) {
        self.active_view = ActiveView::Installed;
        self.items.clear();

        for formula in list.formulas {
            let version = formula
                .linked_version
                .or_else(|| formula.versions.first().cloned())
                .unwrap_or_else(|| String::from("-"));

            self.items.push(InstalledItem {
                name: formula.name,
                version,
                kind: "F",
            });
        }

        for cask in list.casks {
            let version = cask
                .versions
                .first()
                .cloned()
                .unwrap_or_else(|| String::from("-"));

            self.items.push(InstalledItem {
                name: cask.token,
                version,
                kind: "C",
            });
        }

        self.items
            .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        self.clamp_selection_to_visible_items();
        let visible_count = self.visible_item_indices().len();

        if self.items.is_empty() {
            self.list_state.select(None);
            self.status_line = String::from("No installed packages found");
        } else {
            self.status_line = format!(
                "{} installed packages ({} visible)",
                self.items.len(),
                visible_count
            );
        }
    }

    fn apply_outdated_output(&mut self, output: String) {
        self.items.clear();

        for line in output.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let (name, version) = match trimmed.split_once(' ') {
                Some((name, rest)) => (name.to_string(), rest.trim().to_string()),
                None => (trimmed.to_string(), String::from("outdated")),
            };

            self.items.push(InstalledItem {
                name,
                version,
                kind: "O",
            });
        }

        self.items
            .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        self.clamp_selection_to_visible_items();

        if self.items.is_empty() {
            self.status_line = String::from("No outdated packages");
        } else {
            self.status_line = format!("{} outdated packages", self.items.len());
        }
    }

    fn selected_item(&self) -> Option<&InstalledItem> {
        let idx = self.list_state.selected()?;
        let visible_indices = self.visible_item_indices();
        let item_idx = *visible_indices.get(idx)?;
        self.items.get(item_idx)
    }

    fn select_next(&mut self) {
        let len = self.visible_item_indices().len();
        if len == 0 {
            self.list_state.select(None);
            return;
        }
        let i = self.list_state.selected().unwrap_or(0);
        let next = if i >= len - 1 { 0 } else { i + 1 };
        self.list_state.select(Some(next));
    }

    fn select_previous(&mut self) {
        let len = self.visible_item_indices().len();
        if len == 0 {
            self.list_state.select(None);
            return;
        }
        let i = self.list_state.selected().unwrap_or(0);
        let prev = if i == 0 { len - 1 } else { i - 1 };
        self.list_state.select(Some(prev));
    }

    fn visible_item_indices(&self) -> Vec<usize> {
        let query = self.search_query.trim().to_lowercase();
        self.items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                if query.is_empty()
                    || item.name.to_lowercase().contains(&query)
                    || item.version.to_lowercase().contains(&query)
                {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect()
    }

    fn clamp_selection_to_visible_items(&mut self) {
        let len = self.visible_item_indices().len();
        if len == 0 {
            self.list_state.select(None);
            return;
        }

        let selected = self.list_state.selected().unwrap_or(0).min(len - 1);
        self.list_state.select(Some(selected));
    }

    fn current_list_title(&self) -> &'static str {
        match self.active_view {
            ActiveView::Installed => "Installed Packages",
            ActiveView::Outdated => "Outdated Packages",
        }
    }

    fn refresh_active_view(&mut self) {
        match self.active_view {
            ActiveView::Installed => self.request_list(),
            ActiveView::Outdated => self.request_outdated(),
        }
    }
}

fn terminal_supports_color() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }

    if matches!(std::env::var("TERM").ok().as_deref(), Some("dumb")) {
        return false;
    }

    if let Ok(colorterm) = std::env::var("COLORTERM") {
        if !colorterm.is_empty() {
            return true;
        }
    }

    matches!(
        std::env::var("TERM").ok().as_deref(),
        Some(term)
            if term.contains("color")
                || term.contains("xterm")
                || term.contains("screen")
                || term.contains("tmux")
                || term.contains("kitty")
                || term.contains("wezterm")
                || term.contains("alacritty")
                || term.contains("iterm")
    )
}

fn sidebar_label(key: &str, label: &str, active: bool) -> String {
    if active {
        format!("[{}] {} *", key, label)
    } else {
        format!("[{}] {}", key, label)
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let [area] = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .areas(r);
    let [area] = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .areas(area);
    area
}

fn format_info(info: Info) -> String {
    if info.is_empty() {
        return String::from("No details returned from brew info");
    }

    info.iter()
        .map(format_info_entry)
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn format_info_entry(entry: &InfoEntry) -> String {
    let primary_name = if !entry.full_name.is_empty() {
        entry.full_name.as_str()
    } else if !entry.name.is_empty() {
        entry.name.as_str()
    } else if !entry.token.is_empty() {
        entry.token.as_str()
    } else {
        "(unknown)"
    };

    let mut lines = vec![format!("Name: {}", primary_name)];

    let desc = entry.desc.as_deref().unwrap_or("");
    let homepage = entry.homepage.as_deref().unwrap_or("");
    let caveats = entry.caveats.as_deref().unwrap_or("");

    if !desc.is_empty() {
        lines.push(format!("Description: {}", desc));
    }
    if !homepage.is_empty() {
        lines.push(format!("Homepage: {}", homepage));
    }
    if !entry.installed.is_empty() {
        let versions = entry
            .installed
            .iter()
            .filter_map(|x| x.version.as_deref())
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>()
            .join(", ");

        if !versions.is_empty() {
            lines.push(format!("Installed: {}", versions));
        }
    }
    if !caveats.is_empty() {
        lines.push(String::from(""));
        lines.push(String::from("Caveats:"));
        lines.push(caveats.to_string());
    }

    lines.join("\n")
}
