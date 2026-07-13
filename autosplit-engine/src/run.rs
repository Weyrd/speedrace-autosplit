//!  tick loop

use asr::{
    future::next_tick,
    settings,
    timer::{self, TimerState},
    Process,
};

use crate::Game;

// ~5s of failed reads
const STALE_TICKS: u32 = 600;

pub async fn run<G: Game>() {
    asr::print_message("Momentum autosplitter loaded");
    let mut game = G::new();

    loop {
        let process = Process::wait_attach(G::PROCESS_NAME).await;
        asr::print_message("Attached to process");

        process
            .until_closes(async {
                // Attach first (so the host reports "connected" immediately), then wait
                // for the .lss: settings load in parallel with startup and are pushed
                // late by the host. Splits/counters hold until the config lands.
                let cfg = loop {
                    if let Some(cfg) = G::parse_config(&settings::Map::load()) {
                        break cfg;
                    }
                    next_tick().await;
                };

                'redetect: loop {
                    let backend = loop {
                        if let Some(b) = game.detect_backend(&process) {
                            break b;
                        }
                        // Game data not up yet -> retry ~1s
                        for _ in 0..120 {
                            next_tick().await;
                        }
                    };

                    game.reset_counters();
                    game.reset_session();

                    let mut last: Option<G::State> = None;
                    let mut last_idx: Option<usize> = None;
                    let mut stale = 0u32;

                    loop {
                        let Some(state) = game.read_state(&process, &backend) else {
                            stale += 1;
                            if stale >= STALE_TICKS {
                                asr::print_message("Pointers stale, re-detecting");
                                continue 'redetect;
                            }
                            next_tick().await;
                            continue;
                        };
                        stale = 0;

                        if !game.is_consistent(&state) {
                            next_tick().await;
                            continue;
                        }

                        if let Some(prev) = &last {
                            game.observe(&state, prev);
                            match timer::state() {
                                TimerState::NotRunning => {
                                    game.reset_counters();
                                    game.reset_split_state();
                                    if game.should_start(&cfg, &state, prev) {
                                        timer::start();
                                    }
                                }
                                TimerState::Running => {
                                    if game.should_reset(&state) {
                                        timer::reset();
                                    }
                                    let idx = timer::current_split_index().unwrap_or(0) as usize;
                                    // Index moved externally (skip/undo) -> drop split-machine state
                                    if last_idx != Some(idx) {
                                        game.reset_split_state();
                                        last_idx = Some(idx);
                                    }
                                    if game.eval(&cfg, idx, &state, prev) {
                                        timer::split();
                                        game.reset_split_state();
                                    }
                                    game.update_counters(&process, &backend, &state);
                                }
                                _ => {}
                            }
                        }

                        // report igt every tick to reconstruct mid-attached runs
                        if let Some(igt) = game.igt(&state, &cfg) {
                            timer::set_game_time(igt);
                        }

                        last = Some(state);
                        next_tick().await;
                    }
                }
            })
            .await;

        asr::print_message("Process closed");
    }
}
