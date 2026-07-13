use alloc::format;
use asr::{settings, time::Duration, timer, Address, Process};
use autosplit_engine::counter::{Counter, EdgeCounter};
use autosplit_engine::Game;

use crate::consts::{AREA_MENU, VA_SESSION_DASHES, VA_SESSION_DEATHS};
use crate::memory::{self, Backend};
use crate::splits::{self, RunConfig};
use crate::state::GameState;

pub(crate) struct Celeste {
    exiting_chapter: bool,

    deaths: Counter,
    dashes: Counter,
    straws: Counter,
    cassettes: EdgeCounter,
    hearts: EdgeCounter,
    goldens: EdgeCounter,
    last_session: Option<Address>, // Last Session address seen (for stable read)
}

impl Game for Celeste {
    type State = GameState;
    type Backend = Backend;
    type Config = RunConfig;

    const PROCESS_NAME: &'static str = "Celeste.exe";

    fn new() -> Self {
        Celeste {
            exiting_chapter: false,
            deaths: Counter::new(),
            dashes: Counter::new(),
            straws: Counter::new(),
            cassettes: EdgeCounter::new(),
            hearts: EdgeCounter::new(),
            goldens: EdgeCounter::new(),
            last_session: None,
        }
    }

    fn parse_config(map: &settings::Map) -> Option<RunConfig> {
        splits::parse_config(map)
    }

    fn detect_backend(&self, process: &Process) -> Option<Backend> {
        memory::find_vanilla(process).or_else(|| memory::find_everest(process))
    }

    fn read_state(&self, process: &Process, backend: &Backend) -> Option<GameState> {
        memory::read_state(process, backend)
    }

    // guard: in a chapter the room name is never empty
    fn is_consistent(&self, s: &GameState) -> bool {
        !(s.area != AREA_MENU && s.room_len == 0)
    }

    fn should_start(&mut self, cfg: &RunConfig, s: &GameState, p: &GameState) -> bool {
        splits::should_start(cfg, &mut self.exiting_chapter, s, p)
    }

    // Player returned to the main menu -> clear states
    fn should_reset(&self, s: &GameState) -> bool {
        s.area == AREA_MENU
    }

    fn eval(&mut self, cfg: &RunConfig, idx: usize, s: &GameState, p: &GameState) -> bool {
        splits::eval(cfg, &mut self.exiting_chapter, idx + cfg.add_amount, s, p)
    }

    // IL runs report chapter time
    fn igt(&self, s: &GameState, cfg: &RunConfig) -> Option<Duration> {
        if s.area == AREA_MENU {
            return None;
        }
        let elapsed_ms = if cfg.il_splits {
            s.chapter_time_ms
        } else {
            s.file_time_ms
        };
        (elapsed_ms > 0).then(|| Duration::milliseconds(elapsed_ms))
    }

    fn update_counters(&mut self, process: &Process, backend: &Backend, s: &GameState) {
        if s.area != AREA_MENU {
            let session = memory::read_session(process, backend);
            // Trust the read when the Session pointer is at the same adress during two tick (avoid garbage collector)
            if session.is_some() && session == self.last_session {
                self.deaths.feed(memory::read_session_i32(
                    process,
                    session,
                    VA_SESSION_DEATHS,
                ));
                self.dashes.feed(memory::read_session_i32(
                    process,
                    session,
                    VA_SESSION_DASHES,
                ));
                self.straws
                    .feed(memory::read_session_strawberries(process, session));
            }
            self.last_session = session;
        }
        if let Some(v) = self.deaths.take_emit() {
            timer::set_variable("deaths", &format!("{v}"));
        }
        if let Some(v) = self.dashes.take_emit() {
            timer::set_variable("dashes", &format!("{v}"));
        }

        // Collectibles come straight off AutosplitterInfo -> no need to read session
        if s.area != AREA_MENU {
            if matches!(backend, Backend::Everest { .. }) {
                self.straws.feed(Some(s.strawberries));
            }
            self.cassettes.feed(s.ch_cassette);
            self.hearts.feed(s.ch_heart);
            self.goldens.feed(s.ch_golden);
        }
        if let Some(v) = self.straws.take_emit() {
            timer::set_variable("strawberries", &format!("{v}"));
        }
        if let Some(v) = self.cassettes.take_emit() {
            timer::set_variable("cassettes", &format!("{v}"));
        }
        if let Some(v) = self.hearts.take_emit() {
            timer::set_variable("hearts", &format!("{v}"));
        }
        if let Some(v) = self.goldens.take_emit() {
            timer::set_variable("golden", &format!("{v}"));
        }
    }

    fn reset_counters(&mut self) {
        self.deaths.reset();
        self.dashes.reset();
        self.straws.reset();
        self.cassettes.reset();
        self.hearts.reset();
        self.goldens.reset();
    }

    fn reset_session(&mut self) {
        self.last_session = None;
    }

    fn reset_split_state(&mut self) {
        self.exiting_chapter = false;
    }

    // Dev trace -> log the raw chapter state on every
    #[cfg(feature = "trace")]
    fn observe(&mut self, s: &GameState, p: &GameState) {
        let changed = s.area != p.area
            || s.mode != p.mode
            || s.started != p.started
            || s.complete != p.complete
            || s.room() != p.room();
        if changed {
            let room = core::str::from_utf8(s.room()).unwrap_or("?");
            asr::print_message(&format!(
                "[trace] area={} mode={} started={} complete={} room={room:?}",
                s.area, s.mode, s.started, s.complete
            ));
        }
    }
}
