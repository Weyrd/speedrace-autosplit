use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use livesplit_auto_splitting::{AutoSplitter, CompiledAutoSplitter, Config, Runtime};

use crate::lss::build_settings_map;
use crate::shared::{Handles, LogCat};
use crate::timer::TesterTimer;

type SettingsSlot = Arc<Mutex<(u64, Option<String>)>>;

pub struct Runner {
    cancel: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    settings: SettingsSlot,
}

impl Runner {
    pub fn push_settings(&self, inner_xml: String) {
        let mut s = self.settings.lock().unwrap();
        s.0 += 1;
        s.1 = Some(inner_xml);
    }

    pub fn stop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn spawn(
    wasm: Vec<u8>,
    handles: Handles,
    initial_xml: Option<String>,
    ctx: eframe::egui::Context,
) -> Runner {
    let cancel = Arc::new(AtomicBool::new(false));
    let settings: SettingsSlot = Arc::new(Mutex::new((1, initial_xml)));
    let (c, s) = (cancel.clone(), settings.clone());
    let handle = thread::spawn(move || run(wasm, handles, s, c, ctx));
    Runner {
        cancel,
        handle: Some(handle),
        settings,
    }
}

enum Tick {
    Ran { attached: bool },
    Trapped(String),
    Busy,
}

fn run(
    wasm: Vec<u8>,
    handles: Handles,
    settings: SettingsSlot,
    cancel: Arc<AtomicBool>,
    ctx: eframe::egui::Context,
) {
    let mut cfg = Config::default();
    cfg.optimize = true;

    let runtime = match Runtime::new(cfg) {
        Ok(r) => r,
        Err(e) => {
            handles.log(LogCat::Runtime, format!("[rt]    Runtime::new error: {e}"));
            return;
        }
    };
    let compiled = match runtime.compile(&wasm) {
        Ok(c) => c,
        Err(e) => {
            handles.log(LogCat::Runtime, format!("[rt]    compile error: {e}"));
            return;
        }
    };
    handles.log(
        LogCat::Runtime,
        format!("[rt]    loaded {} bytes, instantiating", wasm.len()),
    );

    let mut applied_version = 0u64;
    let mut splitter = match instantiate(&compiled, &handles, &settings, &mut applied_version) {
        Some(s) => s,
        None => return,
    };

    let mut last_attached: Option<bool> = None;

    loop {
        if cancel.load(Ordering::SeqCst) {
            break;
        }

        // lss changed
        {
            let (ver, xml) = {
                let g = settings.lock().unwrap();
                (g.0, g.1.clone())
            };
            if ver != applied_version {
                if let Some(x) = &xml {
                    splitter.set_settings_map(build_settings_map(x));
                    handles.log(LogCat::Runtime, "[rt]    autosplitter settings pushed");
                }
                applied_version = ver;
            }
        }

        let tick_rate = splitter.tick_rate();

        let tick = match splitter.try_lock() {
            Some(mut exec) => match exec.update() {
                Ok(()) => Tick::Ran {
                    attached: exec.attached_processes().next().is_some(),
                },
                Err(e) => Tick::Trapped(e.to_string()),
            },
            None => Tick::Busy,
        };

        match tick {
            Tick::Ran { attached } => {
                if last_attached != Some(attached) {
                    last_attached = Some(attached);
                    handles.shared.lock().unwrap().attached = attached;
                }
                cancellable_sleep(tick_rate, &cancel);
            }
            Tick::Trapped(e) => {
                handles.log(
                    LogCat::Runtime,
                    format!("[rt]    update trapped: {e} — re-instantiating"),
                );
                if last_attached != Some(false) {
                    last_attached = Some(false);
                    handles.shared.lock().unwrap().attached = false;
                }
                match instantiate(&compiled, &handles, &settings, &mut applied_version) {
                    Some(s) => splitter = s,
                    None => break,
                }
                cancellable_sleep(Duration::from_millis(1000), &cancel);
            }
            Tick::Busy => cancellable_sleep(tick_rate, &cancel),
        }

        ctx.request_repaint();
    }

    handles.log(LogCat::Runtime, "[rt]    runner stopped");
}

fn instantiate(
    compiled: &CompiledAutoSplitter,
    handles: &Handles,
    settings: &SettingsSlot,
    applied_version: &mut u64,
) -> Option<AutoSplitter<TesterTimer>> {
    let (ver, map) = {
        let g = settings.lock().unwrap();
        (g.0, g.1.as_deref().map(build_settings_map))
    };
    match compiled.instantiate(TesterTimer::new(handles.clone()), map, None) {
        Ok(s) => {
            *applied_version = ver;
            Some(s)
        }
        Err(e) => {
            handles.log(LogCat::Runtime, format!("[rt]    instantiate error: {e}"));
            None
        }
    }
}

fn cancellable_sleep(dur: Duration, cancel: &AtomicBool) {
    let mut left = dur;
    let chunk = Duration::from_millis(50);
    while left > Duration::ZERO {
        if cancel.load(Ordering::SeqCst) {
            return;
        }
        let step = left.min(chunk);
        thread::sleep(step);
        left -= step;
    }
}
