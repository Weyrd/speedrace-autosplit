use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogCat {
    Wasm,    //  splitter own print_message (log_auto_splitter)
    Runtime, // runtime diagnostics (log_runtime) + tester-side notices
    Var,     // set_variable counter emissions
    Timer,   // start / split / reset / skip / undo
    Trace,   // splitter [trace]/[dbg] lines (--features trace)
}

impl LogCat {
    pub const ALL: [LogCat; 5] = [
        LogCat::Wasm,
        LogCat::Runtime,
        LogCat::Var,
        LogCat::Timer,
        LogCat::Trace,
    ];

    pub fn label(self) -> &'static str {
        match self {
            LogCat::Wasm => "wasm",
            LogCat::Runtime => "runtime",
            LogCat::Var => "var",
            LogCat::Timer => "timer/split",
            LogCat::Trace => "trace",
        }
    }
}

pub struct LogLine {
    pub cat: LogCat,
    pub text: String,
}

// Latest value seen
#[derive(Clone)]
pub struct CounterView {
    pub value: String,
    pub split_index: usize,
}

// 1 row of the loaded .lss segment list
#[derive(Clone)]
pub struct Segment {
    pub name: String,
    // Filled once the wasm fires this split
    pub times: Option<SegmentTimes>,
}

#[derive(Clone, Copy)]
pub struct SegmentTimes {
    pub segment_ms: u128,
    pub total_ms: u128,
}

#[derive(Default)]
pub struct Shared {
    pub run_active: bool,
    pub current_split_index: usize,
    pub attached: bool,
    pub igt_ms: i64,
    pub counters: std::collections::BTreeMap<String, CounterView>,
    pub segments: Vec<Segment>,
}

#[derive(Clone)]
pub struct Handles {
    pub shared: Arc<Mutex<Shared>>,
    pub log: Sender<LogLine>,
}

impl Handles {
    pub fn log(&self, cat: LogCat, text: impl Into<String>) {
        let _ = self.log.send(LogLine {
            cat,
            text: text.into(),
        });
    }
}

pub fn fmt_ms(ms: u128) -> String {
    format!("{}:{:02}.{:03}", ms / 60000, (ms / 1000) % 60, ms % 1000)
}
