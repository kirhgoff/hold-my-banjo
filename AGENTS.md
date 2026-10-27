# hold-my-banjo

Rust workspace plus an Astro site.

- `crates/core` simulation, `crates/cli` scenario runner, `crates/wasm` browser build; `scenarios/` inputs, `audio/` clips.
- `web/` Astro site that loads the wasm build.
- Deploy: `cd web && bun run deploy` (refuses with uncommitted changes).
- Checks: `mise run check` runs everything (fmt, clippy, test, cargo-deny, cargo-machete, web typecheck and build, secrets, typos, workflow lint, agnix); CI runs the same. `mise run setup` installs the git hooks. Advisories run weekly in `advisories.yml`.
