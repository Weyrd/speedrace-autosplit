#![no_std]

extern crate alloc;

pub mod counter;
pub mod dotnet;
mod run;
pub mod settings_xml;

pub use run::run;

use asr::{settings, time::Duration, Process};

pub const SETTINGS_KEY: &str = "autosplitter_settings";

pub trait Game: Sized {
    type State; // Per tick game snapshot the engine
    type Backend; // The games memory-reading backend (Mono, .NET, native)
    type Config; // The per-run config (split points, counter flags...)

    /// Process to attach to
    const PROCESS_NAME: &'static str;

    fn new() -> Self;

    fn parse_config(map: &settings::Map) -> Option<Self::Config>;

    fn detect_backend(&self, process: &Process) -> Option<Self::Backend>;

    fn read_state(&self, process: &Process, backend: &Self::Backend) -> Option<Self::State>;

    // prevent false positives
    fn is_consistent(&self, _state: &Self::State) -> bool {
        true
    }

    // Tester-only start trigger
    fn should_start(
        &mut self,
        _cfg: &Self::Config,
        _state: &Self::State,
        _prev: &Self::State,
    ) -> bool {
        false
    }

    fn should_reset(&self, _state: &Self::State) -> bool {
        false
    }

    fn eval(
        &mut self,
        cfg: &Self::Config,
        idx: usize,
        state: &Self::State,
        prev: &Self::State,
    ) -> bool;

    // In Game Time (mandatory)
    fn igt(&self, state: &Self::State, cfg: &Self::Config) -> Option<Duration>;

    // Read + emit per-run counters
    fn update_counters(
        &mut self,
        _process: &Process,
        _backend: &Self::Backend,
        _state: &Self::State,
    ) {
    }

    fn reset_counters(&mut self) {}

    fn reset_session(&mut self) {}

    fn reset_split_state(&mut self) {}

    // Per-tick debugging hook
    fn observe(&mut self, _state: &Self::State, _prev: &Self::State) {}
}
