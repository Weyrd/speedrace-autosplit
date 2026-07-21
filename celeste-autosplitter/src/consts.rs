//! Everything here is mined from the C# `LiveSplit.Celeste` ASL : https://github.com/ShootMe/LiveSplit.Celeste/tree/master

use asr::signature::Signature;

// ---------------------------------------------------------------------------
// Everest backend  the modded 64-bit game exposes a native info block that
// starts with "EVERESTAUTOSPLIT" + F0 F1 F2 F3 (SplitterMemory.Core.cs)
// ---------------------------------------------------------------------------

pub(crate) const EVEREST_MAGIC: Signature<20> =
    Signature::new("45 56 45 52 45 53 54 41 55 54 4F 53 50 4C 49 54 F0 F1 F2 F3");
pub(crate) const EVEREST_MIN_VERSION: u8 = 3;
// Everest InfoVersion
pub(crate) const EV_VERSION_TORN: u8 = 0xFF;

// Offsets inside Everest's CoreAutoSplitterInfo
pub(crate) const EV_VERSION: u64 = 0x17;
pub(crate) const EV_CHAPTER_ID: u64 = 0x30;
pub(crate) const EV_CHAPTER_MODE: u64 = 0x34;
pub(crate) const EV_ROOM_PTR: u64 = 0x38;
pub(crate) const EV_CHAPTER_FLAGS: u64 = 0x4c;
// CoreAutoSplitterInfo.ChapterStrawberries = lvl.Session.Strawberries.Count (per-run, like vanilla)
pub(crate) const EV_CHAPTER_STRAWBERRIES: u64 = 0x48;

// --- Everest (64-bit .NET Core)  2026-07-21 Everest stable 1.6397.0
pub(crate) const EV_LEVEL_SESSION: u64 = 0x50; // Level.Session field
pub(crate) const S_TIME_FROM_BASE: u64 = 0x78; // i64 tracks ChapterTime -> liveness check
pub(crate) const S_DASHES_FROM_BASE: u64 = 0x84; // i32
pub(crate) const S_DEATHS_FROM_BASE: u64 = 0x8C; // i32
pub(crate) const EV_FILE_STRAWBERRIES: u64 = 0x58;
pub(crate) const EV_FILE_CASSETTES: u64 = 0x60;
pub(crate) const EV_FILE_HEARTS: u64 = 0x64;
// CoreAutoSplitterInfo.{ChapterTime,FileTime}
pub(crate) const EV_CHAPTER_TIME: u64 = 0x40;
pub(crate) const EV_FILE_TIME: u64 = 0x50;

pub(crate) const EV_FILE_FLAGS: u64 = 0x68;
pub(crate) const FILE_FLAG_ACTIVE: u32 = 1 << 31;

pub(crate) const FLAG_STARTED: u32 = 1 << 0;
pub(crate) const FLAG_COMPLETE: u32 = 1 << 1;
pub(crate) const FLAG_CASSETTE: u32 = 1 << 2;
pub(crate) const FLAG_HEART: u32 = 1 << 3;
// Everest-only chapter flag vanilla AutosplitterInfo has no golden field
pub(crate) const FLAG_GRABBED_GOLDEN: u32 = 1 << 4;

// -- vanilla --
pub(crate) const VAN_XNA: Signature<21> =
    Signature::new("83 C6 04 F3 0F 7E 06 66 0F D6 07 8B CB FF 15 ?? ?? ?? ?? 8D 15");
pub(crate) const VAN_OPENGL: Signature<19> =
    Signature::new("8B 55 F0 8B 45 E8 8D 52 74 E8 ?? ?? ?? ?? 8B 45 F0 8D 15");
pub(crate) const VAN_OPENGL14: Signature<17> =
    Signature::new("68 38 04 00 00 68 C0 03 00 00 68 1C 02 00 00 FF 35");
pub(crate) const VAN_ITCH: Signature<21> =
    Signature::new("8D 56 74 E8 ?? ?? ?? ?? 8D 15 ?? ?? ?? ?? E8 ?? ?? ?? ?? C6 05");

// AutosplitterInfo field offsets
pub(crate) const VA_LEVEL: u64 = 0x14;
pub(crate) const VA_CHAPTER: u64 = 0x18;
pub(crate) const VA_MODE: u64 = 0x1c;
pub(crate) const VA_STRAWBERRIES: u64 = 0x24;
pub(crate) const VA_CASSETTES: u64 = 0x28;
pub(crate) const VA_HEARTS: u64 = 0x2c;
pub(crate) const VA_STARTED: u64 = 0x31;
pub(crate) const VA_COMPLETE: u64 = 0x32;
pub(crate) const VA_CH_CASSETTE: u64 = 0x33;
pub(crate) const VA_CH_HEART: u64 = 0x34;
// AutosplitterInfo.{ChapterTime,FileTime}
pub(crate) const VA_CHAPTER_TIME: u64 = 0x4;
pub(crate) const VA_FILE_TIME: u64 = 0xc;
// AutosplitterInfo field Title offset + 0x1c
pub(crate) const VAN_INFO_FROM_TITLE: u64 = 0x1c;

// timers .NET ticks (100ns) = 10_000 ticks = 1ms.
pub(crate) const CELESTE_TICKS_PER_MS: i64 = 10_000;

// Session counters  Celeste.Instance -> scene -> Session -> {Dashes,Deaths}.
pub(crate) const VA_SCENE: u64 = 0x98;
pub(crate) const VA_SESSION: u64 = 0x2c;
pub(crate) const VA_SESSION_DASHES: u64 = 0x44;
pub(crate) const VA_SESSION_DEATHS: u64 = 0x4c;
// Session.Strawberries (HashSet<EntityID>) -> .NET Framework HashSet._count
// FileStrawberries is the per save file total
pub(crate) const VA_SESSION_STRAWBERRIES: u64 = 0x18;
pub(crate) const HASHSET_COUNT: u64 = 0x14;
// Golden held flag Session.GrabbedGolden 1 = true, 0 = false
pub(crate) const VA_SESSION_GOLDEN: u64 = 0x68;

pub(crate) const AREA_MENU: i32 = -1;
pub(crate) const AREA_THE_SUMMIT: i32 = 7;
pub(crate) const MODE_A_SIDE: i32 = 0;
