# RustView — CLAUDE.md

## Project
- **Name**: rustview
- **Level**: Starter (데스크탑 앱, 백엔드 없음)
- **Lang**: Rust 2021 edition
- **GUI**: egui 0.30 + eframe

## Build
```bash
cargo build            # debug
cargo build --release  # release (exe/dmg)
cargo run              # dev run
cargo run -- path/to/image.jpg  # open specific file
```

## Structure
```
src/
├── main.rs    — entry point, CLI arg handling
├── app.rs     — RustViewApp + eframe::App (rendering)
├── types.rs   — shared types (ViewState, LoadedImage, etc.)
├── loader.rs  — image decoding, EXIF, thumbnails
└── nav.rs     — folder navigation helpers
```

## Key Shortcuts (reference)
← → navigate | +/- zoom | 0 fit | 1 original | I info | G gallery | T checker | L/R rotate | F11 fullscreen

## Docs
- `docs/01-plan/plan.md` — project plan
- `docs/01-plan/schema.md` — data model
- `docs/01-plan/market-analysis.md` — market research
- `docs/02-design/mockup-spec.md` — UI spec
- `mockup/index.html` — interactive HTML mockup
