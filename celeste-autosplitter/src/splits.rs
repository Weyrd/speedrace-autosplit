use alloc::{format, string::String, vec::Vec};
use asr::settings;
use autosplit_engine::settings_xml::{xml_texts, xml_unescape};
use autosplit_engine::SETTINGS_KEY;

use crate::consts::{AREA_MENU, AREA_THE_SUMMIT, MODE_A_SIDE};
use crate::state::GameState;

pub(crate) enum SplitDef {
    Manual,
    // Any chapter complete (SplitType.ChapterA)
    CompleteAny,
    // Prologue/Chapter1..9/Epilogue complete
    Complete {
        area: i32,
    },
    // AreaComplete  "Area" or "Area Mode" value
    CompleteArea {
        area: i32,
        mode: Option<i32>,
    },
    AreaEnter {
        area: i32,
        mode: Option<i32>,
    },
    AreaExit {
        area: i32,
        mode: Option<i32>,
    },
    // Chapter checkpoints -> room differs between A-side and B-side
    Checkpoint {
        area: i32,
        a_room: &'static str,
        b_room: &'static str,
    },
    Cassette {
        area: i32,
    },
    HeartGem {
        area: i32,
    },
    HeartGemAny,
    StrawberryPickup,
    GoldenBerry,
    RoomEnter {
        name: String,
    },
    RoomExit {
        name: String,
    },
}

pub(crate) struct RunConfig {
    pub(crate) splits: Vec<SplitDef>,
    pub(crate) every_room: bool,
    pub(crate) il_splits: bool,
    pub(crate) chapter_splits: bool,
    pub(crate) file_time_offset: bool,
    pub(crate) add_amount: usize,
}

fn area_by_name(s: &str) -> Option<i32> {
    let names: &[(&str, i32)] = &[
        ("Menu", -1),
        ("Prologue", 0),
        ("ForsakenCity", 1),
        ("OldSite", 2),
        ("CelestialResort", 3),
        ("GoldenRidge", 4),
        ("MirrorTemple", 5),
        ("Reflection", 6),
        ("TheSummit", 7),
        ("Epilogue", 8),
        ("Core", 9),
        ("Farewell", 10),
    ];
    names
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(s))
        .map(|(_, id)| *id)
}

fn mode_by_name(s: &str) -> Option<i32> {
    let names: &[(&str, i32)] = &[("None", -1), ("ASide", 0), ("BSide", 1), ("CSide", 2)];
    names
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(s))
        .map(|(_, id)| *id)
}

fn parse_area_mode_value(value: &str) -> Option<(i32, Option<i32>)> {
    let mut parts = value.splitn(2, '-');
    let area = area_by_name(parts.next()?.trim())?;
    let mode = parts.next().and_then(|m| mode_by_name(m.trim()));
    Some((area, mode))
}

struct Counts {
    chapter: u32,
    heart: u32,
    area: u32,
    cassette: u32,
}

fn parse_split(text: &str, counts: &mut Counts) -> SplitDef {
    let (name, value) = match text.split_once(',') {
        Some((n, v)) => (n, v),
        None => (text, ""),
    };

    let checkpoint = |area: i32, a_room: &'static str, b_room: &'static str| SplitDef::Checkpoint {
        area,
        a_room,
        b_room,
    };

    let def = match name {
        "Manual" => SplitDef::Manual,
        "ChapterA" => SplitDef::CompleteAny,
        "Prologue" => SplitDef::Complete { area: 0 },
        "Chapter1" => SplitDef::Complete { area: 1 },
        "Chapter2" => SplitDef::Complete { area: 2 },
        "Chapter3" => SplitDef::Complete { area: 3 },
        "Chapter4" => SplitDef::Complete { area: 4 },
        "Chapter5" => SplitDef::Complete { area: 5 },
        "Chapter6" => SplitDef::Complete { area: 6 },
        "Chapter7" => SplitDef::Complete { area: 7 },
        "Epilogue" => SplitDef::Complete { area: 8 },
        "Chapter8" => SplitDef::Complete { area: 9 },
        "Chapter9" => SplitDef::Complete { area: 10 },
        "AreaComplete" => match parse_area_mode_value(value) {
            Some((area, mode)) => SplitDef::CompleteArea { area, mode },
            None => SplitDef::Manual,
        },
        "AreaOnEnter" => match parse_area_mode_value(value) {
            Some((area, mode)) => SplitDef::AreaEnter { area, mode },
            None => SplitDef::Manual,
        },
        "AreaOnExit" => match parse_area_mode_value(value) {
            Some((area, mode)) => SplitDef::AreaExit { area, mode },
            None => SplitDef::Manual,
        },
        "LevelEnter" => SplitDef::RoomEnter {
            name: String::from(value),
        },
        "LevelExit" => SplitDef::RoomExit {
            name: String::from(value),
        },
        "HeartGemAny" => SplitDef::HeartGemAny,
        "StrawberryPickup" => SplitDef::StrawberryPickup,
        "GoldenBerry" => SplitDef::GoldenBerry,
        "Chapter1Checkpoint1" => checkpoint(1, "6", "04"),
        "Chapter1Checkpoint2" => checkpoint(1, "9b", "08"),
        "Chapter2Checkpoint1" => checkpoint(2, "3", "03"),
        "Chapter2Checkpoint2" => checkpoint(2, "end_3", "08b"),
        "Chapter3Checkpoint1" => checkpoint(3, "08-a", "06"),
        "Chapter3Checkpoint2" => checkpoint(3, "09-d", "11"),
        "Chapter3Checkpoint3" => checkpoint(3, "00-d", "16"),
        "Chapter4Checkpoint1" => checkpoint(4, "b-00", "b-00"),
        "Chapter4Checkpoint2" => checkpoint(4, "c-00", "c-00"),
        "Chapter4Checkpoint3" => checkpoint(4, "d-00", "d-00"),
        "Chapter5Checkpoint1" => checkpoint(5, "b-00", "b-00"),
        "Chapter5Checkpoint2" => checkpoint(5, "c-00", "c-00"),
        "Chapter5Checkpoint3" => checkpoint(5, "d-00", "d-00"),
        "Chapter5Checkpoint4" => checkpoint(5, "e-00", "e-00"),
        "Chapter6Checkpoint1" => checkpoint(6, "00", "b-00"),
        "Chapter6Checkpoint2" => checkpoint(6, "04", "c-00"),
        "Chapter6Checkpoint3" => checkpoint(6, "b-00", "d-00"),
        "Chapter6Checkpoint4" => checkpoint(6, "boss-00", "boss-00"),
        "Chapter6Checkpoint5" => checkpoint(6, "after-00", "after-00"),
        "Chapter7Checkpoint1" => checkpoint(7, "b-00", "b-00"),
        "Chapter7Checkpoint2" => checkpoint(7, "c-00", "c-01"),
        "Chapter7Checkpoint3" => checkpoint(7, "d-00", "d-00"),
        "Chapter7Checkpoint4" => checkpoint(7, "e-00b", "e-00"),
        "Chapter7Checkpoint5" => checkpoint(7, "f-00", "f-00"),
        "Chapter7Checkpoint6" => checkpoint(7, "g-00", "g-00"),
        "Chapter8Checkpoint1" => checkpoint(9, "a-00", "a-00"),
        "Chapter8Checkpoint2" => checkpoint(9, "c-00", "b-00"),
        "Chapter8Checkpoint3" => checkpoint(9, "d-00", "c-01"),
        "Chapter9Checkpoint1" => checkpoint(10, "a-00", "a-00"),
        "Chapter9Checkpoint2" => checkpoint(10, "c-00", "c-00"),
        "Chapter9Checkpoint3" => checkpoint(10, "e-00z", "e-00z"),
        "Chapter9Checkpoint4" => checkpoint(10, "f-door", "f-door"),
        "Chapter9Checkpoint5" => checkpoint(10, "h-00b", "h-00b"),
        "Chapter9Checkpoint6" => checkpoint(10, "i-00", "i-00"),
        "Chapter9Checkpoint7" => checkpoint(10, "j-00", "j-00"),
        "Chapter9Checkpoint8" => checkpoint(10, "j-16", "j-16"),
        "Chapter1Cassette" => SplitDef::Cassette { area: 1 },
        "Chapter2Cassette" => SplitDef::Cassette { area: 2 },
        "Chapter3Cassette" => SplitDef::Cassette { area: 3 },
        "Chapter4Cassette" => SplitDef::Cassette { area: 4 },
        "Chapter5Cassette" => SplitDef::Cassette { area: 5 },
        "Chapter6Cassette" => SplitDef::Cassette { area: 6 },
        "Chapter7Cassette" => SplitDef::Cassette { area: 7 },
        "Chapter8Cassette" => SplitDef::Cassette { area: 9 },
        "Chapter1HeartGem" => SplitDef::HeartGem { area: 1 },
        "Chapter2HeartGem" => SplitDef::HeartGem { area: 2 },
        "Chapter3HeartGem" => SplitDef::HeartGem { area: 3 },
        "Chapter4HeartGem" => SplitDef::HeartGem { area: 4 },
        "Chapter5HeartGem" => SplitDef::HeartGem { area: 5 },
        "Chapter6HeartGem" => SplitDef::HeartGem { area: 6 },
        "Chapter7HeartGem" => SplitDef::HeartGem { area: 7 },
        "Chapter8HeartGem" => SplitDef::HeartGem { area: 9 },
        other => {
            asr::print_message(&format!("Unknown split type '{other}', treating as Manual"));
            SplitDef::Manual
        }
    };

    if name.len() == 8 {
        counts.chapter += 1;
    } else if name.contains("HeartGem") {
        counts.heart += 1;
    } else if name.contains("AreaComplete") {
        counts.area += 1;
    } else if name.contains("Cassette") {
        counts.cassette += 1;
    }

    def
}

pub(crate) fn parse_config(map: &settings::Map) -> Option<RunConfig> {
    let xml = map.get(SETTINGS_KEY)?.get_string()?;

    let file_time_offset = xml_texts(&xml, "FileTimeOffset")
        .next()
        .is_some_and(|t| t.trim().eq_ignore_ascii_case("true"));

    let mut counts = Counts {
        chapter: 0,
        heart: 0,
        area: 0,
        cassette: 0,
    };
    let mut splits = Vec::new();
    for text in xml_texts(&xml, "Split") {
        splits.push(parse_split(xml_unescape(text.trim()).trim(), &mut counts));
    }
    let every_room = splits.is_empty();

    let il_splits =
        counts.chapter <= 1 && counts.heart <= 1 && counts.area <= 1 && counts.cassette <= 1;
    let chapter_splits =
        counts.chapter > 0 || counts.heart > 0 || counts.area > 0 || counts.cassette > 0;
    let add_amount = if !splits.is_empty() && !chapter_splits {
        1
    } else {
        0
    };

    asr::print_message(&format!(
        "Config: {} splits, every_room={every_room} il={il_splits} chapter={chapter_splits} fto={file_time_offset} add={add_amount}",
        splits.len()
    ));

    Some(RunConfig {
        splits,
        every_room,
        il_splits,
        chapter_splits,
        file_time_offset,
        add_amount,
    })
}

fn chapter_split(
    exiting: &mut bool,
    il_splits: bool,
    chapter_area: Option<i32>,
    s: &GameState,
    p: &GameState,
) -> bool {
    if !*exiting {
        let area_ok = chapter_area.is_none_or(|a| s.area == a);
        let credits_ok = chapter_area != Some(AREA_THE_SUMMIT)
            || (s.room_len > 0 && !starts_with_ignore_case(s.room(), b"credits"));
        *exiting = area_ok && s.complete && !p.complete && credits_ok;
        *exiting && il_splits
    } else {
        !s.complete && p.complete
    }
}

pub(crate) fn eval(
    cfg: &RunConfig,
    exiting: &mut bool,
    idx: usize,
    s: &GameState,
    p: &GameState,
) -> bool {
    // Empty <Splits /> -> every room-to-room transition, plus chapter complete
    if cfg.every_room {
        let room_change =
            s.area != AREA_MENU && s.room_len > 0 && p.room_len > 0 && s.room() != p.room();
        return room_change || (s.complete && !p.complete);
    }
    let il = cfg.il_splits;
    let il_or_fto = il || cfg.file_time_offset;
    let Some(def) = cfg.splits.get(idx) else {
        return false;
    };
    match def {
        SplitDef::Manual => false,
        SplitDef::CompleteAny => chapter_split(exiting, il, None, s, p),
        SplitDef::Complete { area } => chapter_split(exiting, il, Some(*area), s, p),
        SplitDef::CompleteArea { area, mode } => {
            let (area, mode) = (*area, *mode);
            chapter_split(exiting, il, Some(area), s, p) && mode.is_none_or(|m| p.mode == m)
        }
        SplitDef::AreaEnter { area, mode } => {
            s.area != p.area
                && s.area == *area
                && mode.is_none_or(|m| s.mode != p.mode && s.mode == m)
        }
        SplitDef::AreaExit { area, mode } => {
            s.area != p.area
                && p.area == *area
                && mode.is_none_or(|m| s.mode != p.mode && p.mode == m)
        }
        SplitDef::Checkpoint {
            area,
            a_room,
            b_room,
        } => {
            let expected = if s.mode == MODE_A_SIDE { a_room } else { b_room };
            s.area == *area && s.room() == expected.as_bytes()
        }
        SplitDef::Cassette { area } => {
            s.area == *area && ((il_or_fto && s.ch_cassette) || s.cassettes == p.cassettes + 1)
        }
        SplitDef::HeartGem { area } => {
            s.area == *area && ((il_or_fto && s.ch_heart) || s.hearts == p.hearts + 1)
        }
        SplitDef::HeartGemAny => (il_or_fto && s.ch_heart) || s.hearts == p.hearts + 1,
        SplitDef::StrawberryPickup => s.strawberries == p.strawberries + 1,
        // Rising edge of the held-golden flag (Everest flag or vanilla session byte).
        SplitDef::GoldenBerry => s.ch_golden && !p.ch_golden,
        SplitDef::RoomEnter { name } => {
            s.area != AREA_MENU && s.room() != p.room() && eq_ignore_case(s.room(), name.as_bytes())
        }
        SplitDef::RoomExit { name } => {
            s.area != AREA_MENU && s.room() != p.room() && eq_ignore_case(p.room(), name.as_bytes())
        }
    }
}

// Start trigger (test only)
pub(crate) fn should_start(
    cfg: &RunConfig,
    exiting: &mut bool,
    s: &GameState,
    p: &GameState,
) -> bool {
    if !cfg.chapter_splits && !cfg.splits.is_empty() {
        eval(cfg, exiting, 0, s, p)
    } else {
        s.started && !p.started
    }
}

fn eq_ignore_case(a: &[u8], b: &[u8]) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn starts_with_ignore_case(s: &[u8], prefix: &[u8]) -> bool {
    s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix)
}
