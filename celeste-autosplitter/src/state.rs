#[derive(Clone, Copy)]
pub(crate) struct GameState {
    pub(crate) area: i32,
    pub(crate) mode: i32,
    pub(crate) started: bool,
    pub(crate) complete: bool,
    pub(crate) ch_cassette: bool,
    pub(crate) ch_heart: bool,
    pub(crate) ch_golden: bool, // Everest-only
    pub(crate) cassettes: i32,
    pub(crate) hearts: i32,
    pub(crate) strawberries: i32,

    pub(crate) chapter_strawberries: Option<i32>, // for everest only
    pub(crate) room: [u8; 64],
    pub(crate) room_len: usize,

    pub(crate) chapter_time_ms: i64,
    pub(crate) file_time_ms: i64,

    pub(crate) in_level: bool, // everest
    pub(crate) file_active: bool, // everest (vanilla always true)
}

impl GameState {
    pub(crate) fn room(&self) -> &[u8] {
        &self.room[..self.room_len]
    }
}
