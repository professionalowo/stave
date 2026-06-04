use std::sync::mpsc;

use super::worker;
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{DefaultTerminal, Frame};

#[derive(Debug)]
pub struct App {
    should_quit: bool,
    command_tx: mpsc::Sender<worker::Command>,
    output_rx: mpsc::Receiver<String>,
}

impl App {
    pub fn new(
        command_tx: mpsc::Sender<worker::Command>,
        output_rx: mpsc::Receiver<String>,
    ) -> Self {
        Self {
            should_quit: false,
            command_tx,
            output_rx,
        }
    }

    pub fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.render(frame))?;
            if matches!(
                event::read()?,
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    ..
                })
            ) {
                self.should_quit = true;
            }
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        frame.render_widget("hello world", frame.area());
    }
}
