use std::sync::mpsc;

use super::worker::{Command, Response};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
    style::{Modifier, Style},
    widgets::Paragraph,
};

#[derive(Debug)]
pub struct App {
    should_quit: bool,
    command_tx: mpsc::Sender<Command>,
    output_rx: mpsc::Receiver<Response>,
}

impl App {
    pub fn new(command_tx: mpsc::Sender<Command>, output_rx: mpsc::Receiver<Response>) -> Self {
        Self {
            should_quit: false,
            command_tx,
            output_rx,
        }
    }

    pub fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.render(frame))?;
            if let Event::Key(e) = event::read()? {
                self.handle_key_event(e);
            }
        }
        Ok(())
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Char('q') => self.should_quit = true,
            _ => {}
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let [content_area, help_area] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());

        frame.render_widget("hello world", content_area);

        let help = Paragraph::new("q: Quit").style(Style::default().add_modifier(Modifier::DIM));
        frame.render_widget(help, help_area);
    }
}
