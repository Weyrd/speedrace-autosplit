#![no_std]

extern crate alloc;

#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

asr::async_main!(stable);
asr::panic_handler!();

mod consts;
mod game;
mod memory;
mod splits;
mod state;

mod everest;

async fn main() {
    autosplit_engine::run::<game::Celeste>().await;
}
