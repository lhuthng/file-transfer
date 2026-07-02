use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use tiny_http::Request;

use super::handle_request;

pub(super) fn worker_count() -> usize {
    thread::available_parallelism()
        .map(|parallelism| parallelism.get())
        .unwrap_or(4)
        .clamp(4, 16)
}

pub(super) struct WorkerPool {
    sender: mpsc::Sender<Request>,
}

impl WorkerPool {
    pub(super) fn new(
        workers: usize,
        shared_dir: Arc<PathBuf>,
        token: Arc<Option<String>>,
        read_only: bool,
    ) -> Self {
        let (sender, receiver) = mpsc::channel::<Request>();
        let receiver = Arc::new(Mutex::new(receiver));

        for idx in 0..workers {
            let receiver = Arc::clone(&receiver);
            let shared_dir = Arc::clone(&shared_dir);
            let token = Arc::clone(&token);

            thread::Builder::new()
                .name(format!("file-transfer-worker-{idx}"))
                .spawn(move || loop {
                    let request = match receiver.lock().expect("worker receiver poisoned").recv() {
                        Ok(request) => request,
                        Err(_) => break,
                    };

                    if let Err(e) = handle_request(&shared_dir, &token, read_only, request) {
                        eprintln!("Error: {}", e);
                    }
                })
                .expect("failed to spawn worker thread");
        }

        Self { sender }
    }

    pub(super) fn dispatch(
        &self,
        request: Request,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        self.sender.send(request).map_err(|_| {
            Box::<dyn std::error::Error + Send + Sync>::from(
                "worker pool stopped accepting requests",
            )
        })?;
        Ok(())
    }
}
