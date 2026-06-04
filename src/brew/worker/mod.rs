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
    Outdated,
    Update,
    Upgrade,
    UpgradePackage { name: String, is_cask: bool },
    Shutdown,
}

#[derive(Debug)]
pub enum Response {
    List(BrewList),
    Info(Info),
    Outdated(String),
    UpdateResult(String),
    UpgradeResult(String),
    UpgradePackageResult { name: String, output: String },
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
                Command::Outdated => {
                    let output = run::run_outdated()?;
                    self.output_tx.send(Response::Outdated(output))?;
                }
                Command::Update => {
                    let output = run::run_update()?;
                    self.output_tx.send(Response::UpdateResult(output))?;
                }
                Command::Upgrade => {
                    let output = run::run_upgrade()?;
                    self.output_tx.send(Response::UpgradeResult(output))?;
                }
                Command::UpgradePackage { name, is_cask } => {
                    let output = run::run_upgrade_package(&name, is_cask)?;
                    self.output_tx
                        .send(Response::UpgradePackageResult { name, output })?;
                }
            }
        }
        Ok(())
    }
}
