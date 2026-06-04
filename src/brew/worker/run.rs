use super::{BrewList, Info, ListOption, Response};
use std::{
    io,
    process::{Command, Output},
    sync::mpsc::SendError,
};

#[derive(Debug)]
pub enum CommandError {
    Io(io::Error),
    Serde(serde_json::Error),
}

impl From<io::Error> for CommandError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for CommandError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serde(err)
    }
}

#[derive(Debug)]
pub enum RunError {
    Command(CommandError),
    Send(SendError<Response>),
}

impl From<CommandError> for RunError {
    fn from(err: CommandError) -> Self {
        Self::Command(err)
    }
}

impl From<SendError<Response>> for RunError {
    fn from(err: SendError<Response>) -> Self {
        Self::Send(err)
    }
}

pub fn run_info<S: AsRef<str>>(name: S) -> Result<Info, CommandError> {
    let Output { ref stdout, .. } = Command::new("brew")
        .args(&["info", "--json", name.as_ref()])
        .output()?;
    serde_json::from_slice(stdout).map_err(CommandError::Serde)
}

pub fn run_list(option: ListOption) -> Result<BrewList, CommandError> {
    let mut cmd = Command::new("brew");
    cmd.args(&["list", "--versions", "--json"]);
    match option {
        ListOption::All => {}
        ListOption::Formula => {
            cmd.arg("--formula");
        }
        ListOption::Cask => {
            cmd.arg("--cask");
        }
    }
    let Output { ref stdout, .. } = cmd.output()?;
    serde_json::from_slice(stdout).map_err(CommandError::Serde)
}
