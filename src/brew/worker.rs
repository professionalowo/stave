use std::sync::mpsc;

#[derive(Debug)]
pub enum Command {
    List,
    Shutdown,
}


pub struct Worker {
    command_rx: mpsc::Receiver<Command>,
    output_tx: mpsc::Sender<String>,
}

impl Worker {
    pub fn new(command_rx: mpsc::Receiver<Command>, output_tx: mpsc::Sender<String>) -> Self {
        Self {
            command_rx,
            output_tx,
        }
    }

    pub fn run(&mut self) {
        while let Ok(command) = self.command_rx.recv() {
            match command {
                Command::Shutdown => {
                    break;
                }
                _ => {}
            }
        }
    }
}
