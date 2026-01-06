## lifeminer – Conway's Life pattern miner

This crate searches for unusual Game of Life seeds with a mix of MAP-Elites and novelty pressure. It records seeds, scores, lineage, and exports RLE for replay elsewhere.

### Quick start

```bash
cargo run -- search --out archive --width 48 --height 48 --iterations 200 --initial-population 64 --density 0.24 --max-steps 512
```

Other commands:

- `cargo run -- replay --id <PATTERN_ID> --steps 120 --stride 4` prints ASCII frames.
- `cargo run -- export --id <PATTERN_ID> --out pattern.rle` writes an RLE.
- `cargo run -- stats` summarizes the archive.

### What it does

- Generates random seeds, simulates them with early termination, and extracts features like lifespan, activity, bounding-box drift, and coarse spaceship events.
- Scores patterns and maps them into MAP-Elites niches bucketed by lifespan, mobility, and late activity.
- Applies localized mutations, patch stamping, and density nudging to explore variations.
- Stores every accepted pattern as JSONL plus an RLE snapshot under `archive/`.

### Testing notes

Unit checks cover canonical Life behaviors (blinker, block) and basic cycle detection. Running `cargo test` will pull crates (`clap`, `rayon`, `rand`, `serde`, `ahash`) from crates.io.
