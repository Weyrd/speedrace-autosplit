use std::fmt;
use std::time::Instant;

use livesplit_auto_splitting::{LogLevel, Timer, TimerState};

use crate::shared::{fmt_ms, CounterView, Handles, LogCat, SegmentTimes};

pub struct TesterTimer {
    pub h: Handles,
    started_at: Option<Instant>,
    last_split_at: Option<Instant>,
}

impl TesterTimer {
    pub fn new(h: Handles) -> Self {
        Self {
            h,
            started_at: None,
            last_split_at: None,
        }
    }

    fn label(&self, idx: usize) -> String {
        let g = self.h.shared.lock().unwrap();
        match g.segments.get(idx) {
            Some(seg) => format!("{} (#{idx})", seg.name),
            None => format!("#{idx}"),
        }
    }
}

impl Timer for TesterTimer {
    fn state(&self) -> TimerState {
        if self.h.shared.lock().unwrap().run_active {
            TimerState::Running
        } else {
            TimerState::NotRunning
        }
    }

    fn current_split_index(&self) -> Option<usize> {
        Some(self.h.shared.lock().unwrap().current_split_index)
    }

    fn segment_splitted(&self, idx: usize) -> Option<bool> {
        Some(self.h.shared.lock().unwrap().current_split_index > idx)
    }

    fn start(&mut self) {
        let now = Instant::now();
        self.started_at = Some(now);
        self.last_split_at = Some(now);
        self.h.shared.lock().unwrap().run_active = true;
        self.h.log(LogCat::Timer, "[timer] START");
    }

    fn split(&mut self) {
        if !self.h.shared.lock().unwrap().run_active {
            self.h
                .log(LogCat::Timer, "[timer] split ignored (not running)");
            return;
        }
        let now = Instant::now();
        let segment = self
            .last_split_at
            .map(|t| (now - t).as_millis())
            .unwrap_or(0);
        let total = self.started_at.map(|t| (now - t).as_millis()).unwrap_or(0);
        self.last_split_at = Some(now);

        let idx = self.h.shared.lock().unwrap().current_split_index;
        let label = self.label(idx);
        {
            let mut g = self.h.shared.lock().unwrap();
            if let Some(seg) = g.segments.get_mut(idx) {
                seg.times = Some(SegmentTimes {
                    segment_ms: segment,
                    total_ms: total,
                });
            }
            g.current_split_index = idx + 1;
        }
        self.h.log(
            LogCat::Timer,
            format!(
                "[timer] SPLIT  {label:<20} segment={}  total={}",
                fmt_ms(segment),
                fmt_ms(total)
            ),
        );
    }

    fn skip_split(&mut self) {
        let idx = self.h.shared.lock().unwrap().current_split_index;
        let label = self.label(idx);
        self.h.shared.lock().unwrap().current_split_index += 1;
        self.h.log(LogCat::Timer, format!("[timer] SKIP   {label}"));
    }

    fn undo_split(&mut self) {
        let idx = {
            let mut g = self.h.shared.lock().unwrap();
            g.current_split_index = g.current_split_index.saturating_sub(1);
            g.current_split_index
        };
        self.h.log(
            LogCat::Timer,
            format!("[timer] UNDO   (back to {})", self.label(idx)),
        );
    }

    fn reset(&mut self) {
        self.started_at = None;
        self.last_split_at = None;
        {
            let mut g = self.h.shared.lock().unwrap();
            g.run_active = false;
            g.current_split_index = 0;
            for seg in g.segments.iter_mut() {
                seg.times = None;
            }
        }
        self.h.log(LogCat::Timer, "[timer] RESET");
    }

    fn set_game_time(&mut self, t: livesplit_auto_splitting::time::Duration) {
        self.h.shared.lock().unwrap().igt_ms = t.whole_milliseconds() as i64;
    }

    fn pause_game_time(&mut self) {}
    fn resume_game_time(&mut self) {}

    fn set_variable(&mut self, key: &str, value: &str) {
        let split_index = {
            let mut g = self.h.shared.lock().unwrap();
            let idx = g.current_split_index;
            g.counters.insert(
                key.to_string(),
                CounterView {
                    value: value.to_string(),
                    split_index: idx,
                },
            );
            idx
        };
        self.h.log(
            LogCat::Var,
            format!("[var]   {key} = {value}  (split #{split_index})"),
        );
    }

    fn log_auto_splitter(&mut self, msg: fmt::Arguments) {
        let text = format!("{msg}");
        let cat = if text.starts_with("[trace]") || text.starts_with("[dbg]") {
            LogCat::Trace
        } else {
            LogCat::Wasm
        };
        self.h.log(cat, format!("[wasm]  {text}"));
    }

    fn log_runtime(&mut self, msg: fmt::Arguments, _: LogLevel) {
        self.h.log(LogCat::Runtime, format!("[rt]    {msg}"));
    }
}
