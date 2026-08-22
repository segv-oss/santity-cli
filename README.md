# santity-cli

Native control plane, package manager, and Ratatui TUI dashboard for Santity.

## Architecture

`santity-cli` is the local-first operator tool and package manager for the Santity WebAssembly runtime. It communicates with `santity-core` over Unix Domain Socket IPC (`/tmp/santity.sock`).

## Commands

### 1. `santity up [--configure]`
Runs the interactive setup wizard (if `~/.config/santity/config.toml` is unconfigured) and boots `santity-core` as a detached background process with PID tracking.

### 2. `santity down`
Cleanly terminates the running `santity-core` daemon process.

### 3. `santity plugin add <url|file>`
Downloads a `.component.wasm` plugin binary, prompts for capability domain whitelisting (`allowed_domains`), updates `config.toml`, and performs a POSIX atomic move into `~/.config/santity/plugins/` to trigger zero-downtime hot-reloading.

### 4. `santity plugin list`
Lists all installed plugins and their granted capability domains.

### 5. `santity ui [--socket <path>]`
Launches an interactive, split-pane **Ratatui TUI dashboard** streaming real-time IPC logs, active WASM plugin actors, memory stats, and permit pool gauges.

### 6. `santity new <name>`
Scaffolds a new PDK guest WebAssembly plugin project from template.

### 7. `santity build [--release]`
Compiles guest Rust code to WASM (`wasm32-unknown-unknown`) and automatically translates it into a Component Model binary using `wasm-tools`.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE).
