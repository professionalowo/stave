use crate::worker::PackageKind;

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

pub fn run_outdated() -> Result<String, CommandError> {
    let Output { stdout, .. } = Command::new("brew").args(["outdated"]).output()?;
    Ok(String::from_utf8_lossy(&stdout).into_owned())
}

pub fn run_update() -> Result<String, CommandError> {
    run_text_command(["update"])
}

pub fn run_upgrade_package(name: &str, kind: PackageKind) -> Result<String, CommandError> {
    let Output {
        stdout,
        stderr,
        status,
    } = if kind == PackageKind::Cask {
        Command::new("brew")
            .args(["upgrade", "--cask", name])
            .output()?
    } else {
        Command::new("brew").args(["upgrade", name]).output()?
    };

    Ok(format_command_output(stdout, stderr, status))
}

pub fn run_upgrade() -> Result<String, CommandError> {
    run_text_command(["upgrade"])
}

pub fn run_search(query: &str) -> Result<String, CommandError> {
    let Output { stdout, .. } = Command::new("brew").args(["search", query]).output()?;
    Ok(String::from_utf8_lossy(&stdout).into_owned())
}

pub fn run_install(name: &str) -> Result<String, CommandError> {
    run_text_command(["install", name])
}

pub fn run_uninstall(name: &str) -> Result<String, CommandError> {
    run_text_command(["uninstall", name])
}

fn run_text_command<const N: usize>(args: [&str; N]) -> Result<String, CommandError> {
    let Output {
        stdout,
        stderr,
        status,
    } = Command::new("brew").args(args).output()?;

    Ok(format_command_output(stdout, stderr, status))
}

fn format_command_output(
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status: std::process::ExitStatus,
) -> String {
    let mut text = String::new();

    if !stdout.is_empty() {
        text.push_str(&String::from_utf8_lossy(&stdout));
    }

    if !stderr.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&String::from_utf8_lossy(&stderr));
    }

    if text.trim().is_empty() {
        text = "Command produced no output".to_string();
    }

    if !status.success() {
        text.push_str(&format!("\n\nExit status: {}", status));
    }

    text
}
