# Claw Code Setup And Operations

This document is the canonical runbook for installation, startup, configuration, authentication, and operating modes in the current repository state.

## Scope

The active implementation lives in the Rust workspace under `rust/`.

Primary binaries:

- `claw` — CLI and REPL runtime
- `claw-web` — local browser UI over the same Rust runtime

Primary support surfaces:

- `.claw/sessions/` — per-workspace saved conversations
- `$HOME/.claw/credentials.json` — saved OAuth credentials on local runs
- mounted `/root/.claw/credentials.json` — saved OAuth credentials in Docker
- `mock-anthropic-service` — deterministic local service for parity and test runs

## Startup Matrix

Use this matrix to choose the correct entrypoint.

| Goal | Binary | Recommended auth | Recommended command |
| --- | --- | --- | --- |
| Interactive terminal agent | `claw` | `ANTHROPIC_API_KEY` | `cargo run -p rusty-claude-cli --` |
| One-shot scripted prompt | `claw` | `ANTHROPIC_API_KEY` | `cargo run -p rusty-claude-cli -- prompt "summarize this repository"` |
| Browser UI | `claw-web` | `ANTHROPIC_API_KEY` | `cargo run -p web-api -- --cwd ..` |
| OAuth credential bootstrap only | `claw login` | browser OAuth | `cargo run -p rusty-claude-cli -- login` |
| Local deterministic test harness | `mock-anthropic-service` | none | `./scripts/run_mock_parity_harness.sh` |

Important limitation:

- Saved Claude OAuth credentials persist correctly.
- Direct inference against `https://api.anthropic.com/v1/messages` still requires `ANTHROPIC_API_KEY`.
- OAuth-only inference transport is not implemented in this repository yet.
- If you point `ANTHROPIC_BASE_URL` at a compatible proxy that accepts bearer auth, saved OAuth or `ANTHROPIC_AUTH_TOKEN` may still be usable there.

## Local Setup

### 1. Install the Rust toolchain

On macOS:

```bash
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup default stable
rustup component add clippy rustfmt
```

### 2. Build the workspace

From the repository root:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
cargo build --workspace
```

### 3. Check available commands

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw --help
./target/debug/claw-web --help
```

## Docker Setup

The repository includes a top-level [Dockerfile](../Dockerfile). The image contains:

- Rust toolchain
- `cargo`
- `clippy`
- `rustfmt`
- `git`
- `python3`
- `ripgrep`
- installed `claw`
- installed `claw-web`

### Build the image

```bash
cd "$(git rev-parse --show-toplevel)"
docker build -t claw-code .
```

### Run the CLI in Docker

```bash
cd "$(git rev-parse --show-toplevel)"
read -s ANTHROPIC_API_KEY
echo

docker run --rm -it \
  -e ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  -v "$PWD":/workspace \
  claw-code
```

### Run the web UI in Docker

```bash
cd "$(git rev-parse --show-toplevel)"
read -s ANTHROPIC_API_KEY
echo
mkdir -p "$HOME/.claw-docker"

docker run --rm -it \
  --name claw-web \
  -p 8787:8787 \
  -p 4545:4545 \
  -e ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  -v "$PWD":/workspace \
  -v "$HOME/.claw-docker:/root/.claw" \
  --entrypoint claw-web \
  claw-code \
  --cwd /workspace
```

Open `http://localhost:8787`.

### Docker OAuth bootstrap

If you want to persist Claude OAuth credentials inside the Docker-mounted auth directory:

```bash
cd "$(git rev-parse --show-toplevel)"
mkdir -p "$HOME/.claw-docker"

docker run --rm -it \
  -p 4545:4545 \
  -v "$PWD":/workspace \
  -v "$HOME/.claw-docker:/root/.claw" \
  claw-code login
```

Notes:

- `-p 4545:4545` is required for the browser callback `http://localhost:4545/callback`.
- For the default Anthropic direct API this OAuth login does not make inference ready by itself.
- The saved credentials are still useful if you later use a compatible bearer-capable upstream via `ANTHROPIC_BASE_URL`.

## Authentication Modes

### Mode 1. `ANTHROPIC_API_KEY`

This is the standard working mode for direct Anthropic API access.

Local run:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
read -s ANTHROPIC_API_KEY
echo
export ANTHROPIC_API_KEY
./target/debug/claw prompt "summarize this repository"
```

### Mode 2. `ANTHROPIC_AUTH_TOKEN`

This provides a bearer token from the environment.

Use it only when your configured upstream accepts bearer auth:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
read -s ANTHROPIC_AUTH_TOKEN
echo
export ANTHROPIC_AUTH_TOKEN
export ANTHROPIC_BASE_URL="http://127.0.0.1:8080"
./target/debug/claw prompt "status"
```

### Mode 3. Saved OAuth credentials

`claw login` opens a browser OAuth flow and stores credentials in:

- local: `$HOME/.claw/credentials.json`
- Docker: mounted `/root/.claw/credentials.json`

Commands:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw login
./target/debug/claw logout
```

Current behavior:

- OAuth credentials are saved and loaded correctly.
- The web UI can display OAuth status.
- Default direct inference to Anthropic still requires `ANTHROPIC_API_KEY`.
- When only saved OAuth is present and the base URL is the default Anthropic API, the UI marks auth as not inference-ready and blocks sending.

### Credential precedence

The active source resolves in this order:

1. `ANTHROPIC_API_KEY`
2. `ANTHROPIC_AUTH_TOKEN`
3. saved OAuth credentials
4. no auth

Environment credentials override saved OAuth in both CLI and web modes.

## Configuration

### Environment variables

Supported operational variables:

- `ANTHROPIC_API_KEY` — direct Anthropic API key
- `ANTHROPIC_AUTH_TOKEN` — bearer token for a compatible upstream
- `ANTHROPIC_BASE_URL` — custom API base URL or proxy

### Config file resolution

Runtime config is loaded in this order, with later files overriding earlier ones:

1. `~/.claw.json`
2. `~/.config/claw/settings.json`
3. `"$(git rev-parse --show-toplevel)/.claw.json"`
4. `"$(git rev-parse --show-toplevel)/.claw/settings.json"`
5. `"$(git rev-parse --show-toplevel)/.claw/settings.local.json"`

Use `.claw/settings.local.json` only for machine-local overrides.

## Operating Modes

### 1. Interactive REPL

Best for long-lived interactive work with saved sessions.

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw
```

You can also override the model on startup:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw --model claude-opus-4-6
./target/debug/claw --model claude-sonnet-4-6
```

### 2. One-shot prompt mode

Best for shell automation or a single answer.

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw prompt "summarize this repository"
./target/debug/claw "explain rust/crates/runtime/src/lib.rs"
```

### 3. JSON automation mode

Best for scripts, wrappers, and CI adapters.

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw --output-format json prompt "status"
```

### 4. Resume and maintenance mode

Best for inspecting or maintaining a saved session without entering the REPL.

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw --resume latest
./target/debug/claw --resume latest /status /diff
./target/debug/claw --resume latest /compact
```

### 5. Browser UI mode

Best when you want a persistent local console in the browser with streaming text and tool events.

Local run:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
cargo run -p web-api -- --cwd ..
```

Features exposed in the current web UI:

- model input
- permission mode selector
- allowed tools field
- enable/disable tool use for the current turn
- session list
- manual session refresh
- new session creation
- session compaction
- OAuth status panel
- SSE streaming over `/api/chat/stream`

### 6. Mock parity harness mode

Best for deterministic verification without talking to the live Anthropic API.

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./scripts/run_mock_parity_harness.sh
```

Manual mock service:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
cargo run -p mock-anthropic-service -- --bind 127.0.0.1:0
```

## Permission Modes

The runtime supports three permission modes:

- `read-only` — read/search tools only
- `workspace-write` — can modify files under the workspace
- `danger-full-access` — unrestricted local access

CLI examples:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw --permission-mode read-only prompt "summarize Cargo.toml"
./target/debug/claw --permission-mode workspace-write prompt "update the README"
./target/debug/claw --permission-mode danger-full-access prompt "inspect and fix the failing tests"
```

Web UI:

- the permission selector uses the same three values
- the request is rejected if an unsupported permission mode is supplied

## Tool Modes

### All default tools enabled

Normal behavior unless you restrict them.

### Restricted tool set

Use `--allowedTools` in the CLI or `Allowed Tools` in the web UI.

CLI:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw --allowedTools read,glob,grep "inspect the runtime crate"
```

Web UI:

- type a comma-separated tool list such as `read,glob,grep`
- clear the field to remove the restriction

### Tool use disabled for a turn

Web UI only:

- uncheck `Enable tool use for this chat turn`
- the turn will run as a pure model request without tool execution

## Models

Supported short aliases in the CLI:

- `opus` -> `claude-opus-4-6`
- `sonnet` -> `claude-sonnet-4-6`
- `haiku` -> `claude-haiku-4-5-20251213`

Equivalent CLI examples:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
./target/debug/claw --model opus prompt "review this diff"
./target/debug/claw --model claude-opus-4-6 prompt "review this diff"
```

The web UI model field expects the full model identifier.

## Sessions And Storage

### Session storage

Saved conversations live under the current workspace:

- `"$(git rev-parse --show-toplevel)/.claw/sessions/"`

The REPL auto-saves turns there.

### Credential storage

Local runs:

- `$HOME/.claw/credentials.json`

Docker runs with the documented volume mount:

- `$HOME/.claw-docker/credentials.json` on the host
- `/root/.claw/credentials.json` inside the container

### Session commands

Useful slash commands:

- `/help`
- `/status`
- `/sandbox`
- `/model`
- `/permissions`
- `/cost`
- `/resume`
- `/session`
- `/config`
- `/memory`
- `/diff`
- `/export`
- `/agents`
- `/mcp`
- `/skills`

## Web UI Controls

The current [index.html](../rust/crates/web-api/static/index.html) exposes these controls:

- `Login With Claude` — starts browser OAuth flow against the loopback callback
- `Clear Saved OAuth` — removes persisted OAuth credentials
- `Model` — full model ID
- `Permission Mode` — `danger-full-access`, `workspace-write`, `read-only`
- `Allowed Tools` — comma-separated runtime tool allow-list
- `Enable tool use for this chat turn` — disables tool execution for the current turn
- `Refresh` — reloads saved sessions
- `New` — starts a fresh local session
- `Compact Current Session` — compacts the loaded session
- `Send Prompt` — starts an SSE-backed streaming turn

The web UI status areas show:

- auth source
- whether auth is inference-ready
- workspace root
- runtime version
- runtime date context
- current session ID
- per-message usage if present

## Troubleshooting

### `invalid x-api-key`

Cause:

- `ANTHROPIC_API_KEY` is missing, empty, or invalid.

Fix:

```bash
cd "$(git rev-parse --show-toplevel)"
read -s ANTHROPIC_API_KEY
echo
export ANTHROPIC_API_KEY
```

### `OAuth authentication is currently not supported`

Cause:

- only saved OAuth is available
- the runtime is still talking directly to the default Anthropic Messages API

Fix:

- use `ANTHROPIC_API_KEY` for direct Anthropic access
- or set `ANTHROPIC_BASE_URL` to a compatible upstream that supports bearer auth

### Browser callback resets on `http://localhost:4545/callback`

Cause:

- the callback port is not published from Docker
- or an old container/image is still running

Fix:

```bash
cd "$(git rev-parse --show-toplevel)"
docker rm -f claw-web 2>/dev/null || true
docker build -t claw-code .
```

Then rerun the container with:

```bash
cd "$(git rev-parse --show-toplevel)"
docker run --rm -it \
  -p 4545:4545 \
  -p 8787:8787 \
  -v "$PWD":/workspace \
  -v "$HOME/.claw-docker:/root/.claw" \
  --entrypoint claw-web \
  claw-code \
  --cwd /workspace
```

### Web UI says auth is saved, but `Send Prompt` is disabled

Cause:

- saved OAuth exists
- direct default Anthropic inference is not supported through OAuth-only auth in this repository

Fix:

- launch with `ANTHROPIC_API_KEY`
- or configure a bearer-capable upstream with `ANTHROPIC_BASE_URL`

### Stale sessions or auth state in Docker

Clean the host-mounted auth state:

```bash
rm -f "$HOME/.claw-docker/credentials.json"
```

Clean the local workspace sessions:

```bash
rm -rf "$(git rev-parse --show-toplevel)/.claw/sessions"
```

## Verification

Run formatting, linting, and tests from the Rust workspace:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

When the full workspace test suite is too heavy for the current Docker memory limit, at minimum run:

```bash
cd "$(git rev-parse --show-toplevel)/rust"
cargo test -p api -p web-api
```

## Canonical Entry Points

Use these files as the current documentation entry points:

- [README.md](../README.md) — repository overview
- [USAGE.md](../USAGE.md) — concise quick-start guide
- [SETUP_AND_OPERATIONS.md](./SETUP_AND_OPERATIONS.md) — detailed runbook
- [rust/README.md](../rust/README.md) — Rust workspace overview
