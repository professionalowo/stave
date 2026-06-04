use std::sync::mpsc;
use app::App;
use color_eyre::Result;
use brew::worker;

mod app;
mod brew;

fn main() -> Result<()> {
    let (main_tx, main_rx) = mpsc::channel::<worker::Command>();
    let (worker_tx, worker_rx) = mpsc::channel::<String>();
    let worker_handle = std::thread::spawn(move || {
        let mut worker = worker::Worker::new(main_rx, worker_tx);
        worker.run();
    });
    color_eyre::install()?;
    let terminal = ratatui::init();
    let mut app = App::new(main_tx.clone(), worker_rx);
    let result = app.run(terminal);
    main_tx.send(worker::Command::Shutdown)?;
    worker_handle.join().expect("Worker thread panicked");
    ratatui::restore();
    result
}
