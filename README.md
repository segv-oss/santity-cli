# santity-cli

Native control plane, package manager, binary installer, and Ratatui TUI dashboard for Santity.

---

## 🚀 Quickstart: Get Your Bot Live in 60 Seconds

### 1. Install `santity`
```bash
cargo install santity
```

### 2. Boot the Runtime Daemon
Run `santity up` to trigger the interactive setup wizard. It will prompt for your Discord Bot Token and automatically install `santity-core` if it's missing:
```bash
santity up
```

### 3. Install an Official Pre-Compiled WASM Plugin (or Scaffold Your Own)
You can install official compiled plugins directly from GitHub Releases:
```bash
santity plugin add https://github.com/segv-oss/santity-plugins/releases/latest/download/ping_pong.component.wasm
```

Or scaffold a custom plugin:
```bash
santity new my_bot_plugin
cd my_bot_plugin
santity build --release
santity plugin add target/wasm32-unknown-unknown/release/my_bot_plugin.component.wasm
```

### 4. Launch Live Ratatui TUI Dashboard
Stream real-time IPC logs, active WASM actors, slash command routers, memory stats, and permit pool gauges:
```bash
santity ui
```

---

## 🏛️ Architecture

`santity` (packaged as `santity`, source repo `santity-cli`) is the local-first operator tool and package manager for the Santity WebAssembly runtime. It communicates with `santity-core` over Unix Domain Socket IPC (`/tmp/santity.sock`).

---

## 📋 Command Reference

### `santity up [--configure]`
Runs the interactive setup wizard (if `~/.config/santity/config.toml` is unconfigured), auto-installs `santity-core` if missing, and boots `santity-core` via systemd user service or background daemon.

### `santity core <install|status>`
Manages the `santity-core` runtime engine binary (auto-installing via `cargo install santity-core` or displaying status/location).

### `santity down`
Cleanly terminates the running `santity-core` daemon process.

### `santity plugin add <url|file>`
Downloads a `.component.wasm` plugin binary (from a local file path or GitHub Release URL), prompts for capability domain whitelisting (`allowed_domains`), updates `config.toml`, and performs a POSIX atomic move into `~/.config/santity/plugins/` to trigger zero-downtime hot-reloading.

### `santity plugin list`
Lists all installed plugins and their granted capability domains.

### `santity ui [--socket <path>]`
Launches an interactive, split-pane **Ratatui TUI dashboard** streaming real-time IPC logs, active WASM plugin actors, memory stats, and permit pool gauges.

### `santity new <name>`
Scaffolds a new PDK guest WebAssembly plugin project from template.

### `santity build [--release]`
Compiles guest Rust code to WASM (`wasm32-unknown-unknown`) and automatically translates it into a Component Model binary using `wasm-tools`.

---

## 📜 License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE).
