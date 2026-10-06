# hold-my-banjo

Rust workspace plus an Astro site.

- `crates/core` simulation, `crates/cli` scenario runner, `crates/wasm` browser build; `scenarios/` inputs, `audio/` clips.
- `web/` Astro site that loads the wasm build.
- Deploy: `cd web && bun run deploy` (refuses with uncommitted changes).
- CI: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `bunx tsc --noEmit` in `web/`.
