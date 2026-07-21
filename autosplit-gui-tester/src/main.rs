mod lss;
mod runtime;
mod shared;
mod timer;
mod ui;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use shared::{Handles, LogCat, LogLine, Segment, Shared};

const MAX_LOG_LINES: usize = 5000;

pub struct App {
    pub(crate) shared: Arc<Mutex<Shared>>,
    pub(crate) log_tx: Sender<LogLine>,
    pub(crate) log_rx: Receiver<LogLine>,
    pub(crate) logs: Vec<LogLine>,
    pub(crate) enabled: HashSet<LogCat>,
    pub(crate) runner: Option<runtime::Runner>,
    pub(crate) wasm_path: Option<PathBuf>,
    pub(crate) lss_path: Option<PathBuf>,
    pub(crate) settings_xml: Option<String>,
    pub(crate) autoscroll: bool,
}

impl App {
    fn new() -> Self {
        let (log_tx, log_rx) = channel();
        Self {
            shared: Arc::new(Mutex::new(Shared::default())),
            log_tx,
            log_rx,
            logs: Vec::new(),
            enabled: LogCat::ALL.into_iter().collect(),
            runner: None,
            wasm_path: None,
            lss_path: None,
            settings_xml: None,
            autoscroll: true,
        }
    }

    fn handles(&self) -> Handles {
        Handles {
            shared: self.shared.clone(),
            log: self.log_tx.clone(),
        }
    }

    pub(crate) fn load_wasm(&mut self, path: PathBuf, ctx: &eframe::egui::Context) {
        let wasm = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                let _ = self.log_tx.send(LogLine {
                    cat: LogCat::Runtime,
                    text: format!("[rt]    cannot read {}: {e}", path.display()),
                });
                return;
            }
        };

        {
            let mut g = self.shared.lock().unwrap();
            g.run_active = false;
            g.current_split_index = 0;
            g.attached = false;
            g.igt_ms = 0;
            g.counters.clear();
            for seg in g.segments.iter_mut() {
                seg.times = None;
            }
        }
        self.runner = None;
        self.runner = Some(runtime::spawn(
            wasm,
            self.handles(),
            self.settings_xml.clone(),
            ctx.clone(),
        ));
        self.wasm_path = Some(path);
    }

    pub(crate) fn load_lss(&mut self, path: PathBuf) {
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                let _ = self.log_tx.send(LogLine {
                    cat: LogCat::Runtime,
                    text: format!("[rt]    cannot read {}: {e}", path.display()),
                });
                return;
            }
        };
        let names = lss::parse_split_names(&content);
        {
            let mut g = self.shared.lock().unwrap();
            g.segments = names
                .into_iter()
                .map(|name| Segment { name, times: None })
                .collect();
            g.current_split_index = 0;
        }
        self.settings_xml = lss::autosplitter_settings(&content).map(str::to_string);
        if let (Some(runner), Some(xml)) = (&self.runner, &self.settings_xml) {
            runner.push_settings(xml.clone());
        }
        self.lss_path = Some(path);
    }

    fn drain_logs(&mut self) {
        while let Ok(line) = self.log_rx.try_recv() {
            self.logs.push(line);
        }
        if self.logs.len() > MAX_LOG_LINES {
            let drop = self.logs.len() - MAX_LOG_LINES;
            self.logs.drain(0..drop);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.drain_logs();
        self.ui(ctx);
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1100.0, 680.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Speedrace Autosplit Tester",
        options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}
