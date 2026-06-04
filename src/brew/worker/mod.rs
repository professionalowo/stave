use std::sync::mpsc;

use crate::brew::bindings::{BrewList, Info};

mod run;

#[derive(Debug)]
pub enum ListOption {
    All,
    Formula,
    Cask,
}

#[derive(Debug)]
pub enum Command {
    List(ListOption),
    Info(String),
    Shutdown,
}

#[derive(Debug)]
pub enum Response {
    List(BrewList),
    Info(Info),
}

pub struct Worker {
    command_rx: mpsc::Receiver<Command>,
    output_tx: mpsc::Sender<Response>,
}

impl Worker {
    pub fn new(command_rx: mpsc::Receiver<Command>, output_tx: mpsc::Sender<Response>) -> Self {
        Self {
            command_rx,
            output_tx,
        }
    }

    pub fn run(&mut self) -> Result<(), run::RunError> {
        while let Ok(command) = self.command_rx.recv() {
            match command {
                Command::Shutdown => {
                    break;
                }
                Command::Info(name) => {
                    let info = run::run_info(name)?;
                    self.output_tx.send(Response::Info(info))?;
                }
                Command::List(option) => {
                    let brew_list = run::run_list(option)?;
                    self.output_tx.send(Response::List(brew_list))?;
                }
            }
        }
        Ok(())
    }
}
