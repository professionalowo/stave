use std::{sync::mpsc, time::Duration};

use crate::brew::{
    bindings::{BrewList, Info, InfoEntry},
    worker::{Command, ListOption, Response},
};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Modifier, Style, Stylize},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

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

#[derive(Debug)]
pub struct App {
    should_quit: bool,
    command_tx: mpsc::Sender<Command>,
    output_rx: mpsc::Receiver<Response>,
    filter: InstalledFilter,
    items: Vec<InstalledItem>,
    list_state: ListState,
    status_line: String,
    show_info_popup: bool,
    info_popup_text: String,
}

impl App {
    pub fn new(command_tx: mpsc::Sender<Command>, output_rx: mpsc::Receiver<Response>) -> Self {
        let mut app = Self {
            should_quit: false,
            command_tx,
            output_rx,
            filter: InstalledFilter::All,
            items: Vec::new(),
            list_state: ListState::default().with_selected(Some(0)),
            status_line: String::from("Loading installed packages..."),
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
        match event.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => self.select_next(),
            KeyCode::Char('k') | KeyCode::Up => self.select_previous(),
            KeyCode::Char('1') => {
                self.filter = InstalledFilter::All;
                self.request_list();
            }
            KeyCode::Char('2') => {
                self.filter = InstalledFilter::Formula;
                self.request_list();
            }
            KeyCode::Char('3') => {
                self.filter = InstalledFilter::Cask;
                self.request_list();
            }
            KeyCode::Char('r') => self.request_list(),
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
        let [header_area, content_area, help_area] =
            Layout::vertical([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)])
                .areas(frame.area());

        let header = Paragraph::new("Stave")
            .centered()
            .bold()
            .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(header, header_area);

        let [left_area, right_area] =
            Layout::horizontal([Constraint::Length(28), Constraint::Min(0)]).areas(content_area);

        let menu_items = vec![
            ListItem::new(sidebar_label("1", "Installed (All)", self.filter == InstalledFilter::All)),
            ListItem::new(sidebar_label(
                "2",
                "Installed (Formulae)",
                self.filter == InstalledFilter::Formula,
            )),
            ListItem::new(sidebar_label("3", "Installed (Casks)", self.filter == InstalledFilter::Cask)),
            ListItem::new(" "),
            ListItem::new("Outdated (todo)".dim()),
            ListItem::new("Search (todo)".dim()),
            ListItem::new("Cleanup (todo)".dim()),
        ];
        let menu = List::new(menu_items).block(Block::default().title("Views / Functions").borders(Borders::ALL));
        frame.render_widget(menu, left_area);

        let rows: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| ListItem::new(format!("{:<28} {:<5} {}", item.name, item.kind, item.version)))
            .collect();

        let list = List::new(rows)
            .block(
                Block::default()
                    .title("Installed Packages")
                    .title_bottom(self.status_line.clone())
                    .borders(Borders::ALL),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, right_area, &mut self.list_state);

        let help = Paragraph::new("q: Quit  j/k or Arrows: Move  i: Info Popup  1/2/3: Switch View  r: Refresh")
            .style(Style::default().add_modifier(Modifier::DIM));
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

    fn request_info_for_selected(&mut self) {
        if let Some(item) = self.selected_item() {
            if self.command_tx.send(Command::Info(item.name.clone())).is_err() {
                self.status_line = String::from("Failed to send info command to worker");
            } else {
                self.status_line = format!("Loading info for {}...", item.name);
            }
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
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.status_line = String::from("Worker disconnected");
                    break;
                }
            }
        }
    }

    fn apply_list(&mut self, list: BrewList) {
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

        if self.items.is_empty() {
            self.list_state.select(None);
            self.status_line = String::from("No installed packages found");
        } else {
            let selected = self.list_state.selected().unwrap_or(0).min(self.items.len() - 1);
            self.list_state.select(Some(selected));
            self.status_line = format!("{} installed packages", self.items.len());
        }
    }

    fn selected_item(&self) -> Option<&InstalledItem> {
        self.list_state.selected().and_then(|i| self.items.get(i))
    }

    fn select_next(&mut self) {
        if self.items.is_empty() {
            self.list_state.select(None);
            return;
        }
        let i = self.list_state.selected().unwrap_or(0);
        let next = if i >= self.items.len() - 1 { 0 } else { i + 1 };
        self.list_state.select(Some(next));
    }

    fn select_previous(&mut self) {
        if self.items.is_empty() {
            self.list_state.select(None);
            return;
        }
        let i = self.list_state.selected().unwrap_or(0);
        let prev = if i == 0 { self.items.len() - 1 } else { i - 1 };
        self.list_state.select(Some(prev));
    }
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
