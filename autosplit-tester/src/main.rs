use std::fmt;
use std::sync::Arc;
use livesplit_auto_splitting::{settings, Config, LogLevel, Runtime, Timer, TimerState};

fn parse_split_names(content: &str) -> Vec<String> {
    if content.is_empty() {
        return vec![];
    }
    let doc = match roxmltree::Document::parse(content) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("warning: cannot parse splits file: {e}");
            return vec![];
        }
    };
    doc.descendants()
        .filter(|n| n.tag_name().name() == "Segment")
        .filter_map(|seg| {
            seg.children()
                .find(|n| n.tag_name().name() == "Name")
                .and_then(|n| n.text())
                .map(str::to_string)
        })
        .collect()
}

// Inner XML of <AutoSplitterSettings>, as momentum-app forwards it to the wasm
fn autosplitter_settings(content: &str) -> Option<&str> {
    let start = content.find("<AutoSplitterSettings>")? + "<AutoSplitterSettings>".len();
    let len = content[start..].find("</AutoSplitterSettings>")?;
    Some(&content[start..start + len])
}

struct DebugTimer {
    state: TimerState,
    splits: usize,
    names: Vec<String>,
    started_at: Option<std::time::Instant>,
    last_split_at: Option<std::time::Instant>,
}

fn fmt_dur(d: std::time::Duration) -> String {
    let ms = d.as_millis();
    format!("{}:{:02}.{:03}", ms / 60000, (ms / 1000) % 60, ms % 1000)
}

impl DebugTimer {
    fn label(&self, idx: usize) -> String {
        self.names
            .get(idx)
            .map(|n| format!("{n} (#{idx})"))
            .unwrap_or_else(|| format!("#{idx}"))
    }
}

impl Timer for DebugTimer {
    fn state(&self) -> TimerState {
        self.state
    }

    fn current_split_index(&self) -> Option<usize> {
        Some(self.splits)
    }

    fn segment_splitted(&self, idx: usize) -> Option<bool> {
        Some(self.splits > idx)
    }

    fn start(&mut self) {
        self.state = TimerState::Running;
        let now = std::time::Instant::now();
        self.started_at = Some(now);
        self.last_split_at = Some(now);
        println!("[timer] START");
    }

    fn split(&mut self) {
        if self.state != TimerState::Running {
            println!("[timer] split ignored (not running)");
            return;
        }
        let now = std::time::Instant::now();
        let segment = self.last_split_at.map(|t| now - t).unwrap_or_default();
        let total = self.started_at.map(|t| now - t).unwrap_or_default();
        self.last_split_at = Some(now);
        println!(
            "[timer] SPLIT  {:<20} segment={}  total={}",
            self.label(self.splits),
            fmt_dur(segment),
            fmt_dur(total)
        );
        self.splits += 1;
    }

    fn skip_split(&mut self) {
        println!("[timer] SKIP   {}", self.label(self.splits));
        self.splits += 1;
    }

    fn undo_split(&mut self) {
        if self.splits > 0 {
            self.splits -= 1;
        }
        println!("[timer] UNDO   (back to {})", self.label(self.splits));
    }

    fn reset(&mut self) {
        self.state = TimerState::NotRunning;
        self.splits = 0;
        self.started_at = None;
        self.last_split_at = None;
        println!("[timer] RESET");
    }

    fn set_game_time(&mut self, _: livesplit_auto_splitting::time::Duration) {}
    fn pause_game_time(&mut self) {}
    fn resume_game_time(&mut self) {}

    fn set_variable(&mut self, key: &str, value: &str) {
        println!("[var]   {} = {}", key, value);
    }

    fn log_auto_splitter(&mut self, msg: fmt::Arguments) {
        println!("[wasm]  {msg}");
    }

    fn log_runtime(&mut self, msg: fmt::Arguments, _: LogLevel) {
        println!("[rt]    {msg}");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let wasm_path = args.get(1).cloned().unwrap_or_else(|| {
        eprintln!("usage: {} <wasm-file> [splits.lss]", args[0]);
        std::process::exit(1);
    });

    let lss = args.get(2).map(|p| {
        std::fs::read_to_string(p).unwrap_or_else(|e| {
            eprintln!("warning: cannot read splits file {p}: {e}");
            String::new()
        })
    });

    let names = lss
        .as_deref()
        .map(|content| {
            let names = parse_split_names(content);
            if !names.is_empty() {
                println!("Splits: {}", names.join(" → "));
            }
            names
        })
        .unwrap_or_default();

    let settings_map = lss.as_deref().and_then(autosplitter_settings).map(|xml| {
        println!("AutoSplitterSettings: {} bytes", xml.len());
        let mut map = settings::Map::new();
        map.insert(
            Arc::from("autosplitter_settings"),
            settings::Value::String(Arc::from(xml)),
        );
        map
    });

    let wasm = std::fs::read(&wasm_path).unwrap_or_else(|e| {
        eprintln!("error: cannot read {wasm_path}: {e}");
        std::process::exit(1);
    });

    println!("Loaded {} bytes from {}", wasm.len(), wasm_path);

    let mut cfg = Config::default();
    cfg.optimize = true;

    let runtime = Runtime::new(cfg).expect("runtime init failed");
    let compiled = runtime.compile(&wasm).expect("compile failed");
    let splitter = compiled
        .instantiate(
            DebugTimer {
                state: TimerState::NotRunning,
                splits: 0,
                names,
                started_at: None,
                last_split_at: None,
            },
            settings_map,
            None,
        )
        .expect("instantiate failed");

    println!("Running — start Celeste to see events\n");

    loop {
        let tick_rate = splitter.tick_rate();
        std::thread::sleep(tick_rate);
        if let Some(mut exec) = splitter.try_lock() {
            if let Err(e) = exec.update() {
                eprintln!("update error: {e}");
                break;
            }
        }
    }
}
