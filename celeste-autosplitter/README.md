# celeste-autosplitter

Celeste `.wasm` auto splitter for Momentum -> one wasm for the whole game, split
sequence at runtime by `.lss` file's `<AutoSplitterSettings>`.

```bash
cargo build --release
# output -> target/wasm32-unknown-unknown/release/celeste_autosplitter.wasm
```

Test in game with `../autosplit-tester`:

```bash
cd ../autosplit-tester
cargo run --release -- \
  ../celeste-autosplitter/target/wasm32-unknown-unknown/release/celeste_autosplitter.wasm \
  "path/to/category.lss"
```

## Structure

The reusable lifecycle (attach loop, timer/IGT contract, settings parsing, run counters, .NET/Mono
memory helpers) lives in `../autosplit-engine`. This crate only supplies Celeste's specifics:

| File            | What                                                                                  |
| --------------- | ------------------------------------------------------------------------------------- |
| `src/lib.rs`    | crate shell (allocator, `async_main!`, `panic_handler!`) → `engine::run::<Celeste>()` |
| `src/consts.rs` | every memory address / signature / offset                                             |
| `src/state.rs`  | the backend-agnostic `GameState` snapshot                                             |
| `src/memory.rs` | backend detection (vanilla sig-scan + Everest) + `read_state` + session reads         |
| `src/splits.rs` | the split grammar + evaluator (parsed from `<AutoSplitterSettings>`)                  |
| `src/game.rs`   | `Celeste` — implements `autosplit_engine::Game`, owns the run counters                |
