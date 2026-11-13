use eframe::egui::Context;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use crate::common::common_interface::{ScanMessage, ScanReport};
use crate::common::models::script_scanner_model::ScriptScannerModel;

/// Scans on another thread so the window keeps animating while a big project loads.
pub struct ScanController {
    receiver: Option<Receiver<ScanMessage>>,
    pub found: usize,
    pub root: Option<PathBuf>,
}

impl ScanController {
    pub fn new() -> Self {
        Self { receiver: None, found: 0, root: None }
    }

    pub fn is_busy(&self) -> bool {
        self.receiver.is_some()
    }

    pub fn start(&mut self, root: PathBuf, ctx: &Context) {
        let (sender, receiver) = mpsc::channel();
        let ctx = ctx.clone();

        self.receiver = Some(receiver);
        self.found = 0;
        self.root = Some(root.clone());

        thread::spawn(move || {
            let progress_sender = sender.clone();
            let progress_ctx = ctx.clone();

            let report = ScriptScannerModel::scan(&root, &|count| {
                let _ = progress_sender.send(ScanMessage::Progress(count));
                progress_ctx.request_repaint();
            });

            let _ = sender.send(ScanMessage::Finished(Box::new(report)));
            ctx.request_repaint();
        });
    }

    /// Call every frame. Gives back the finished report once.
    pub fn poll(&mut self) -> Option<ScanReport> {
        let receiver = self.receiver.as_ref()?;
        let mut finished = None;

        while let Ok(message) = receiver.try_recv() {
            match message {
                ScanMessage::Progress(count) => self.found = count,
                ScanMessage::Finished(report) => finished = Some(*report),
            }
        }

        if finished.is_some() {
            self.receiver = None;
        }

        finished
    }
}
