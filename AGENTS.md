# Repository Guidelines

## Project Structure & Module Organization
- `Cargo.toml`/`Cargo.lock` define the Rust crate `lifeminer` (edition 2021).
- `src/main.rs` is the CLI entrypoint with `search`, `replay`, `export`, and `stats` subcommands.
- `src/lib.rs` re-exports core modules: `life_core.rs` (Life simulation), `eval.rs`, `detect.rs`, `search.rs`, `mutate.rs`, and `storage.rs`.
- Tests live inline under `#[cfg(test)] mod tests` within source files (no separate `tests/` directory).
- Generated artifacts go under `archive/` (created by the CLI): `records.jsonl` and `patterns/*.rle`.

## Build, Test, and Development Commands
- `cargo build` compiles the crate.
- `cargo run -- search --out archive --width 48 --height 48 --iterations 200 --initial-population 64 --density 0.24 --max-steps 512` runs a search and writes an archive.
- `cargo run -- replay --id <PATTERN_ID> --steps 120 --stride 4` prints ASCII frames for a stored pattern.
- `cargo run -- export --id <PATTERN_ID> --out pattern.rle` exports an RLE file.
- `cargo run -- stats` summarizes the archive.
- `cargo test` runs unit tests.
- `cargo fmt` formats with rustfmt (no custom config in repo).

## Coding Style & Naming Conventions
- Follow rustfmt defaults (4-space indentation, no tabs).
- Use `snake_case` for functions/vars/files, `UpperCamelCase` for types, and `SCREAMING_SNAKE_CASE` for constants.
- Keep CLI parsing in `src/main.rs` and reusable logic in `src/` modules; prefer returning `anyhow::Result` from fallible CLI paths.

## Testing Guidelines
- Current tests cover core Life behavior in `src/life_core.rs` and cycle detection in `src/eval.rs`.
- Add tests alongside the module you change; name tests by behavior (e.g., `blinker_period_two`).
- Run `cargo test` before submitting changes that touch simulation or evaluation logic.

## Commit & Pull Request Guidelines
- This checkout has no `.git` history, so no commit message convention is established here.
- Use a short, imperative subject when committing (e.g., "Improve archive stats").
- PRs should include a brief summary, commands run, and notes about generated data; avoid committing `archive/` outputs unless explicitly required.

## Configuration & Data Tips
- Runtime behavior is controlled by CLI flags (width/height, iteration counts, RNG seed).
- Archives are append-only JSONL plus RLE snapshots; keep them out of source control unless you are intentionally versioning datasets.
