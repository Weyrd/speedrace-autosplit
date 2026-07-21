//! Everest (64-bit .NET Core)
//! `[anchor] -> Level -> +EV_LEVEL_SESSION -> Session -> {deaths, dashes}` (probed 2026-07-21)

use alloc::vec;
use alloc::vec::Vec;
use asr::{Address, MemoryRangeFlags, Process};

use crate::consts::*;

const CHUNK: usize = 64 << 10;
const BUDGET_PER_TICK: usize = 16 << 20;
const MAX_LEVELS: usize = 32;

const MAX_ANCHORS: usize = 1024;
const PRUNE_INTERVAL: u32 = 600;

const MISS_LIMIT: u32 = 180;
// Failed full pass -> retry delay (5s)
const COOLDOWN_TICKS: u32 = 600;
// Anchor slots
const ANCHOR_REGION_MAX: u64 = 4 << 20;

fn validate_session(process: &Process, base: Address, sbase: u64) -> Option<(i32, i32)> {
    if sbase < 0x1_0000 || sbase & 7 != 0 {
        return None;
    }
    let mt = process.read::<u64>(Address::new(sbase)).ok()?;
    if mt & 7 != 0 || !(0x1_0000..0x0000_7FFF_FFFF_FFFF).contains(&mt) {
        return None;
    }
    let sz = process.read::<u32>(Address::new(mt + 4)).ok()?;
    if sz % 8 != 0 || !(0x80..=0x1000).contains(&sz) {
        return None;
    }
    let ct = process.read::<i64>(base + EV_CHAPTER_TIME).ok()?;
    let t = process
        .read::<i64>(Address::new(sbase + S_TIME_FROM_BASE))
        .ok()?;
    if (t - ct).abs() >= 500_000 {
        return None;
    }
    let d = process
        .read::<i32>(Address::new(sbase + S_DEATHS_FROM_BASE))
        .ok()?;
    let da = process
        .read::<i32>(Address::new(sbase + S_DASHES_FROM_BASE))
        .ok()?;
    ((0..=1_000_000).contains(&d) && (0..=1_000_000).contains(&da)).then_some((d, da))
}

fn plausible_level(process: &Process, lvl: u64) -> Option<bool> {
    if lvl < 0x1_0000 || lvl & 7 != 0 {
        return None;
    }
    let mt = process.read::<u64>(Address::new(lvl)).ok()?;
    if mt & 7 != 0 || !(0x1_0000..0x0000_7FFF_FFFF_FFFF).contains(&mt) {
        return None;
    }
    let sz = process.read::<u32>(Address::new(mt + 4)).ok()?;
    if sz % 8 != 0 || !(0x100..=0x1000).contains(&sz) {
        return None;
    }
    Some(mt >= 0x7FF0_0000_0000)
}

struct ScanCursor {
    ranges: Vec<(u64, u64)>,
    idx: usize,
    off: u64,
}

impl ScanCursor {
    fn snapshot(process: &Process, max_range: u64) -> Self {
        let mut ranges = Vec::new();
        for range in process.memory_ranges() {
            let Ok((b, sz)) = range.range() else { continue };
            let writable = range
                .flags()
                .map(|f| {
                    f.contains(MemoryRangeFlags::WRITE) && !f.contains(MemoryRangeFlags::EXECUTE)
                })
                .unwrap_or(true);
            if writable && sz <= max_range {
                ranges.push((b.value(), sz));
            }
        }
        ScanCursor {
            ranges,
            idx: 0,
            off: 0,
        }
    }

    fn next_chunk(&mut self, process: &Process, buf: &mut [u8]) -> Option<(u64, usize)> {
        while self.idx < self.ranges.len() {
            let (b, sz) = self.ranges[self.idx];
            if self.off >= sz {
                self.idx += 1;
                self.off = 0;
                continue;
            }
            let n = core::cmp::min(buf.len() as u64, sz - self.off) as usize;
            let addr = b + self.off;
            if process
                .read_into_buf(Address::new(addr), &mut buf[..n])
                .is_ok()
            {
                self.off += n as u64;
                return Some((addr, n));
            }
            // unreadable page -> skip the rest
            self.idx += 1;
            self.off = 0;
        }
        None
    }
}

enum State {
    Idle,
    FindSession(ScanCursor),
    FindLevels {
        sbase: u64,
        cursor: ScanCursor,
        // (candidate Level base, strict MT)
        levels: Vec<(u64, bool)>,
    },
    FindAnchors {
        levels: Vec<u64>,
        cursor: ScanCursor,
        anchors: Vec<u64>,
    },
    Ready(Vec<u64>),
    Cooldown(u32),
}

pub(crate) struct EvChain {
    state: State,
    buf: Vec<u8>,
    miss: u32,

    prev_anchors: Vec<u64>,
    prune_in: u32,
    bench: Vec<u64>,
    pub(crate) last: Option<(i32, i32)>,
}

impl EvChain {
    pub(crate) fn new() -> Self {
        EvChain {
            state: State::Idle,
            buf: vec![0u8; CHUNK],
            miss: 0,
            prev_anchors: Vec::new(),
            prune_in: PRUNE_INTERVAL,
            bench: Vec::new(),
            last: None,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.miss = 0;
        self.last = None;
        if !matches!(self.state, State::Ready(_)) {
            self.state = State::Idle;
        }
    }

    pub(crate) fn idle_tick(&mut self) {
        self.last = None;
    }

    pub(crate) fn tick(&mut self, process: &Process, base: Address) {
        self.last = None;
        let state = core::mem::replace(&mut self.state, State::Idle);
        self.state = match state {
            State::Idle => State::FindSession(ScanCursor::snapshot(process, 0x2_0000_0000)),
            State::Cooldown(n) => {
                if n <= 1 {
                    State::Idle
                } else {
                    State::Cooldown(n - 1)
                }
            }
            State::FindSession(cursor) => self.step_session(process, base, cursor),
            State::FindLevels {
                sbase,
                cursor,
                levels,
            } => self.step_levels(process, base, sbase, cursor, levels),
            State::FindAnchors {
                levels,
                cursor,
                anchors,
            } => self.step_anchors(process, base, levels, cursor, anchors),
            State::Ready(anchors) => self.step_ready(process, base, anchors),
        };
    }

    fn chain_read(process: &Process, base: Address, slot: u64) -> Option<(i32, i32)> {
        let lvl = process.read::<u64>(Address::new(slot)).ok()?;
        if lvl < 0x1_0000 || lvl & 7 != 0 {
            return None;
        }
        let sb = process
            .read::<u64>(Address::new(lvl + EV_LEVEL_SESSION))
            .ok()?;
        validate_session(process, base, sb)
    }

    fn step_ready(&mut self, process: &Process, base: Address, mut anchors: Vec<u64>) -> State {
        for i in 0..anchors.len() {
            let slot = anchors[i];
            if let Some(dd) = Self::chain_read(process, base, slot) {
                self.miss = 0;
                self.last = Some(dd);
                anchors.swap(0, i); // MRU live slot goes first
                self.prune_in = self.prune_in.saturating_sub(1);
                if self.prune_in == 0 {
                    self.prune_in = PRUNE_INTERVAL;
                    let lvl = process.read::<u64>(Address::new(anchors[0])).unwrap_or(0);
                    let bench = &mut self.bench;
                    anchors.retain(|&s| {
                        let keep =
                            matches!(process.read::<u64>(Address::new(s)), Ok(v) if v == lvl);
                        if !keep && bench.len() < MAX_ANCHORS && !bench.contains(&s) {
                            bench.push(s);
                        }
                        keep
                    });
                    #[cfg(feature = "trace")]
                    asr::print_message(&alloc::format!(
                        "[ev] pruned to {} anchors ({} benched)",
                        anchors.len(),
                        self.bench.len()
                    ));
                }
                return State::Ready(anchors);
            }
        }
        // Main set missed
        for i in 0..self.bench.len() {
            let slot = self.bench[i];
            if let Some(dd) = Self::chain_read(process, base, slot) {
                self.miss = 0;
                self.last = Some(dd);
                self.bench.swap_remove(i);
                anchors.insert(0, slot);
                #[cfg(feature = "trace")]
                asr::print_message(&alloc::format!("[ev] promoted benched anchor @{slot:X}"));
                return State::Ready(anchors);
            }
        }
        self.miss += 1;
        if self.miss >= MISS_LIMIT {
            asr::print_message("Everest chain: anchors lost, rediscovering");
            self.miss = 0;
            self.bench.clear();
            return State::Idle;
        }
        State::Ready(anchors)
    }

    fn step_session(&mut self, process: &Process, base: Address, mut cursor: ScanCursor) -> State {
        let Ok(ct) = process.read::<i64>(base + EV_CHAPTER_TIME) else {
            return State::FindSession(cursor);
        };
        let (lo, hi) = (ct - 300_000, ct + 80_000_000);
        let mut spent = 0usize;
        while spent < BUDGET_PER_TICK {
            let Some((addr, n)) = cursor.next_chunk(process, &mut self.buf) else {
                return State::Cooldown(COOLDOWN_TICKS);
            };
            spent += n;
            let mut i = 0;
            while i + 8 <= n {
                let v = i64::from_le_bytes(self.buf[i..i + 8].try_into().unwrap());
                if v >= lo && v <= hi {
                    let sbase = (addr + i as u64).wrapping_sub(S_TIME_FROM_BASE);
                    if validate_session(process, base, sbase).is_some() {
                        #[cfg(feature = "trace")]
                        asr::print_message(&alloc::format!("[ev] session @{sbase:X}"));
                        return State::FindLevels {
                            sbase,
                            cursor: ScanCursor::snapshot(process, 0x2_0000_0000),
                            levels: Vec::new(),
                        };
                    }
                }
                i += 8;
            }
        }
        State::FindSession(cursor)
    }

    fn step_levels(
        &mut self,
        process: &Process,
        base: Address,
        sbase: u64,
        mut cursor: ScanCursor,
        mut levels: Vec<(u64, bool)>,
    ) -> State {
        // Session died mid scan (GC move / chapter change) -> restart discovery
        if validate_session(process, base, sbase).is_none() {
            return State::Idle;
        }
        let target = sbase.to_le_bytes();
        let mut spent = 0usize;
        while spent < BUDGET_PER_TICK {
            let Some((addr, n)) = cursor.next_chunk(process, &mut self.buf) else {
                let strict: Vec<u64> = levels.iter().filter(|(_, s)| *s).map(|(l, _)| *l).collect();
                let levels: Vec<u64> = if strict.is_empty() {
                    levels.iter().map(|(l, _)| *l).collect()
                } else {
                    strict
                };
                return if levels.is_empty() {
                    State::Cooldown(COOLDOWN_TICKS)
                } else {
                    #[cfg(feature = "trace")]
                    asr::print_message(&alloc::format!("[ev] {} level candidate(s)", levels.len()));
                    let mut cursor = ScanCursor::snapshot(process, ANCHOR_REGION_MAX);
                    cursor.ranges.sort_by_key(|&(_, sz)| sz);
                    State::FindAnchors {
                        levels,
                        cursor,
                        anchors: Vec::new(),
                    }
                };
            };
            spent += n;
            let mut i = 0;
            while i + 8 <= n {
                if self.buf[i..i + 8] == target {
                    let lvl = (addr + i as u64).wrapping_sub(EV_LEVEL_SESSION);
                    if levels.len() < MAX_LEVELS && !levels.iter().any(|(l, _)| *l == lvl) {
                        if let Some(strict) = plausible_level(process, lvl) {
                            #[cfg(feature = "trace")]
                            asr::print_message(&alloc::format!(
                                "[ev] level candidate @{lvl:X} strict={strict}"
                            ));
                            levels.push((lvl, strict));
                        }
                    }
                }
                i += 8;
            }
        }
        State::FindLevels {
            sbase,
            cursor,
            levels,
        }
    }

    fn step_anchors(
        &mut self,
        process: &Process,
        base: Address,
        levels: Vec<u64>,
        mut cursor: ScanCursor,
        mut anchors: Vec<u64>,
    ) -> State {
        let (min_l, max_l) = levels
            .iter()
            .fold((u64::MAX, 0u64), |(lo, hi), &v| (lo.min(v), hi.max(v)));
        let mut spent = 0usize;
        while spent < BUDGET_PER_TICK {
            let Some((addr, n)) = cursor.next_chunk(process, &mut self.buf) else {
                if anchors.is_empty() {
                    return State::Cooldown(COOLDOWN_TICKS);
                }
                let golden: Vec<u64> = anchors
                    .iter()
                    .copied()
                    .filter(|a| self.prev_anchors.contains(a))
                    .collect();
                let intersected = !golden.is_empty();
                let anchors = if intersected { golden } else { anchors };
                asr::print_message(&alloc::format!(
                    "Everest chain: {} anchors ready{}",
                    anchors.len(),
                    if intersected { " (intersected)" } else { "" }
                ));
                self.prev_anchors = anchors.clone();
                return State::Ready(anchors);
            };
            spent += n;
            let mut i = 0;
            while i + 8 <= n {
                let v = u64::from_le_bytes(self.buf[i..i + 8].try_into().unwrap());
                if v >= min_l && v <= max_l && levels.contains(&v) {
                    let slot = addr + i as u64;
                    if anchors.len() < MAX_ANCHORS && !anchors.contains(&slot) {
                        if let Ok(sb) = process.read::<u64>(Address::new(v + EV_LEVEL_SESSION)) {
                            if validate_session(process, base, sb).is_some() {
                                anchors.push(slot);
                            }
                        }
                    }
                }
                i += 8;
            }
        }
        State::FindAnchors {
            levels,
            cursor,
            anchors,
        }
    }
}
