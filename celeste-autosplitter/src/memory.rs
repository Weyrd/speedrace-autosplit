//! Turning Celeste process into a normalized [`GameState`]

use alloc::format;
use asr::{Address, MemoryRangeFlags, Process};
use autosplit_engine::dotnet::{read_net_string, read_u32_ptr};

use crate::consts::*;
use crate::state::GameState;

pub(crate) enum Backend {
    Everest {
        base: Address,
    },
    Vanilla {
        static_addr: Address,
        field_offs: u64,
    },
}

pub(crate) fn find_everest(process: &Process) -> Option<Backend> {
    for range in process.memory_ranges() {
        let Ok(r) = range.range() else { continue };
        if let Some(base) = EVEREST_MAGIC.scan_process_range(process, r) {
            if process.read::<u8>(base + EV_VERSION).unwrap_or(0) >= EVEREST_MIN_VERSION {
                asr::print_message(&format!("Everest info block @ {:X}", base.value()));
                return Some(Backend::Everest { base });
            }
        }
    }
    None
}

// Scans the Celeste instance backwards for "Celeste"
fn find_title_offset(process: &Process, instance: Address) -> Option<u64> {
    let mt = read_u32_ptr(process, instance)?;
    let size = process.read::<u32>(mt + 4u32).ok()?;
    if !(8..=0x1000).contains(&size) {
        return None;
    }
    let mut offs = (size as u64) - 4;
    while offs >= 4 {
        if let Some(obj) = read_u32_ptr(process, instance + offs) {
            let mut buf = [0u8; 64];
            if read_net_string(process, obj, &mut buf) == b"Celeste" {
                return Some(offs);
            }
        }
        offs -= 4;
    }
    None
}

fn scan_vanilla_sig(process: &Process) -> Option<Address> {
    for range in process.memory_ranges() {
        let Ok(r) = range.range() else { continue };
        // JIT code lives in RWX regions
        let is_jit = range
            .flags()
            .map(|f| f.contains(MemoryRangeFlags::EXECUTE | MemoryRangeFlags::WRITE))
            .unwrap_or(true);
        if !is_jit {
            continue;
        }
        let found = VAN_XNA
            .scan_process_range(process, r)
            .map(|a| a + 21u32)
            .or_else(|| VAN_OPENGL.scan_process_range(process, r).map(|a| a + 19u32))
            .or_else(|| VAN_ITCH.scan_process_range(process, r).map(|a| a + 10u32))
            .or_else(|| {
                VAN_OPENGL14
                    .scan_process_range(process, r)
                    .map(|a| a + 99u32)
            });
        if let Some(operand_addr) = found {
            // imm32 operand embedded in the instruction is the static field address
            if let Some(static_addr) = read_u32_ptr(process, operand_addr) {
                return Some(static_addr);
            }
        }
    }
    None
}

pub(crate) fn find_vanilla(process: &Process) -> Option<Backend> {
    let static_addr = scan_vanilla_sig(process)?;
    let instance = read_u32_ptr(process, static_addr)?;
    let field_offs = find_title_offset(process, instance)?;
    asr::print_message(&format!(
        "Vanilla: Instance static @ {:X}, Title offs {:X}",
        static_addr.value(),
        field_offs
    ));
    Some(Backend::Vanilla {
        static_addr,
        field_offs,
    })
}

pub(crate) fn read_state(process: &Process, backend: &Backend) -> Option<GameState> {
    let mut room = [0u8; 64];
    match backend {
        Backend::Everest { base } => {
            let area = process.read::<i32>(*base + EV_CHAPTER_ID).ok()?;
            let mode = process.read::<i32>(*base + EV_CHAPTER_MODE).ok()?;
            let flags = process.read::<u32>(*base + EV_CHAPTER_FLAGS).ok()?;
            let cassettes = process.read::<i32>(*base + EV_FILE_CASSETTES).ok()?;
            let hearts = process.read::<i32>(*base + EV_FILE_HEARTS).ok()?;
            let strawberries = process.read::<i32>(*base + EV_FILE_STRAWBERRIES).ok()?;
            // A bad time read must not drop the whole sample.
            let chapter_time_ms = process
                .read::<i64>(*base + EV_CHAPTER_TIME)
                .map(|t| t / CELESTE_TICKS_PER_MS)
                .unwrap_or(0);
            let file_time_ms = process
                .read::<i64>(*base + EV_FILE_TIME)
                .map(|t| t / CELESTE_TICKS_PER_MS)
                .unwrap_or(0);
            // Room name: u16 length
            let mut room_len = 0usize;
            if let Ok(str_ptr) = process.read::<u64>(*base + EV_ROOM_PTR) {
                if str_ptr != 0 {
                    if let Ok(len) = process.read::<u16>(Address::new(str_ptr - 2)) {
                        let len = (len as usize).min(64);
                        if process
                            .read_into_buf(Address::new(str_ptr), &mut room[..len])
                            .is_ok()
                        {
                            room_len = len;
                        }
                    }
                }
            }
            Some(GameState {
                area,
                mode,
                started: flags & FLAG_STARTED != 0,
                complete: flags & FLAG_COMPLETE != 0,
                ch_cassette: flags & FLAG_CASSETTE != 0,
                ch_heart: flags & FLAG_HEART != 0,
                ch_golden: flags & FLAG_GRABBED_GOLDEN != 0,
                cassettes,
                hearts,
                strawberries,
                room,
                room_len,
                chapter_time_ms,
                file_time_ms,
            })
        }
        Backend::Vanilla {
            static_addr,
            field_offs,
        } => {
            let instance = read_u32_ptr(process, *static_addr)?;
            let info = read_u32_ptr(process, instance + *field_offs + VAN_INFO_FROM_TITLE)?;
            let area = process.read::<i32>(info + VA_CHAPTER).ok()?;
            let mode = process.read::<i32>(info + VA_MODE).ok()?;
            let cassettes = process.read::<i32>(info + VA_CASSETTES).ok()?;
            let hearts = process.read::<i32>(info + VA_HEARTS).ok()?;
            let strawberries = process.read::<i32>(info + VA_STRAWBERRIES).ok()?;
            let started = process.read::<u8>(info + VA_STARTED).ok()? != 0;
            let complete = process.read::<u8>(info + VA_COMPLETE).ok()? != 0;
            let ch_cassette = process.read::<u8>(info + VA_CH_CASSETTE).ok()? != 0;
            let ch_heart = process.read::<u8>(info + VA_CH_HEART).ok()? != 0;
            let ch_golden = read_u32_ptr(process, instance + VA_SCENE)
                .and_then(|scene| read_u32_ptr(process, scene + VA_SESSION))
                .and_then(|session| process.read::<u8>(session + VA_SESSION_GOLDEN).ok())
                .is_some_and(|b| b != 0);
            let chapter_time_ms = process
                .read::<i64>(info + VA_CHAPTER_TIME)
                .map(|t| t / CELESTE_TICKS_PER_MS)
                .unwrap_or(0);
            let file_time_ms = process
                .read::<i64>(info + VA_FILE_TIME)
                .map(|t| t / CELESTE_TICKS_PER_MS)
                .unwrap_or(0);
            let mut room_len = 0usize;
            if let Some(str_obj) = read_u32_ptr(process, info + VA_LEVEL) {
                let mut buf = [0u8; 64];
                let s = read_net_string(process, str_obj, &mut buf);
                room_len = s.len();
                room[..room_len].copy_from_slice(s);
            }
            Some(GameState {
                area,
                mode,
                started,
                complete,
                ch_cassette,
                ch_heart,
                ch_golden,
                cassettes,
                hearts,
                strawberries,
                room,
                room_len,
                chapter_time_ms,
                file_time_ms,
            })
        }
    }
}

// Session object (vanilla) in Celeste.Instance -> scene -> Session
pub(crate) fn read_session(process: &Process, backend: &Backend) -> Option<Address> {
    let Backend::Vanilla { static_addr, .. } = backend else {
        return None;
    };
    let instance = read_u32_ptr(process, *static_addr)?;
    let scene = read_u32_ptr(process, instance + VA_SCENE)?;
    read_u32_ptr(process, scene + VA_SESSION)
}

pub(crate) fn read_session_i32(
    process: &Process,
    session: Option<Address>,
    offset: u64,
) -> Option<i32> {
    let v = process.read::<i32>(session? + offset).ok()?;
    (0..=1_000_000).contains(&v).then_some(v)
}

pub(crate) fn read_session_strawberries(
    process: &Process,
    session: Option<Address>,
) -> Option<i32> {
    let set = read_u32_ptr(process, session? + VA_SESSION_STRAWBERRIES)?;
    let n = process.read::<i32>(set + HASHSET_COUNT).ok()?;
    (0..=1000).contains(&n).then_some(n)
}
