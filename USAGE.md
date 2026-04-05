# Claw Code Usage

This guide covers the current Rust workspace under `rust/` and the `claw` CLI binary.

For the full runbook, use [docs/SETUP_AND_OPERATIONS.md](/Users/michaelvolkov/projects/claw-code/docs/SETUP_AND_OPERATIONS.md). This file stays focused on quick-start commands.

## Prerequisites

- Rust toolchain with `cargo`
- One of:
  - `ANTHROPIC_API_KEY` for direct API access
  - `claw login` for OAuth-based auth
- Optional: `ANTHROPIC_BASE_URL` when targeting a proxy or local service

## Build the workspace

```bash
cd rust
cargo build --workspace
```

The CLI binary is available at `rust/target/debug/claw` after a debug build.

## Run with Docker

The repository now includes a top-level `Dockerfile` for running `claw` in a containerized dev environment.

Build the image from the repository root:

```bash
cd /Users/michaelvolkov/projects/claw-code
docker build -t claw-code .
```

Run the CLI interactively against the current repository mounted at `/workspace`:

```bash
cd /Users/michaelvolkov/projects/claw-code
read -s ANTHROPIC_API_KEY
echo

docker run --rm -it \
  -e ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  -v "$PWD":/workspace \
  claw-code
```

Run a one-shot prompt:

```bash
cd /Users/michaelvolkov/projects/claw-code
read -s ANTHROPIC_API_KEY
echo

docker run --rm -it \
  -e ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  -v "$PWD":/workspace \
  claw-code prompt "summarize this repository"
```

The image keeps the Rust toolchain, `git`, `python3`, and `rg` installed so the container can be used as a practical `claw` workspace rather than a binary-only wrapper.

### Run the web interface

The Rust workspace also includes a browser UI served by the new `claw-web` binary.

Run it from the repository root:

```bash
cd /Users/michaelvolkov/projects/claw-code
mkdir -p "$HOME/.claw-docker"

docker run --rm -it \
  -p 4545:4545 \
  -p 8787:8787 \
  -v "$PWD":/workspace \
  -v "$HOME/.claw-docker:/root/.claw" \
  --entrypoint claw-web \
  claw-code \
  --cwd /workspace
```

Then open `http://localhost:8787` in the browser.

The web interface streams assistant deltas, tool activity, usage updates, and final turn completion over the `/api/chat/stream` SSE endpoint.

If you want to use browser-based Claude OAuth from the web UI, do not pass `ANTHROPIC_API_KEY` or `ANTHROPIC_AUTH_TOKEN` into the container, because environment credentials override saved OAuth tokens.
Claw Web now completes OAuth through the same loopback callback as `claw login`: `http://localhost:4545/callback`.
When running in Docker, the callback port must be published with `-p 4545:4545`, otherwise the browser cannot deliver the OAuth redirect back into the container.
Saved Claude OAuth credentials are persisted correctly, but this runtime still sends direct requests to the Anthropic Messages API by default. For direct inference against `https://api.anthropic.com`, you still need `ANTHROPIC_API_KEY`; OAuth-only inference transport is not implemented in this project yet.

For local non-Docker runs:

```bash
cd rust
cargo run -p web-api -- --cwd ..
```

## Quick start

### Interactive REPL

```bash
cd rust
./target/debug/claw
```

### One-shot prompt

```bash
cd rust
./target/debug/claw prompt "summarize this repository"
```

### Shorthand prompt mode

```bash
cd rust
./target/debug/claw "explain rust/crates/runtime/src/lib.rs"
```

### JSON output for scripting

```bash
cd rust
./target/debug/claw --output-format json prompt "status"
```

## Model and permission controls

```bash
cd rust
./target/debug/claw --model sonnet prompt "review this diff"
./target/debug/claw --permission-mode read-only prompt "summarize Cargo.toml"
./target/debug/claw --permission-mode workspace-write prompt "update README.md"
./target/debug/claw --allowedTools read,glob "inspect the runtime crate"
```

Supported permission modes:

- `read-only`
- `workspace-write`
- `danger-full-access`

Model aliases currently supported by the CLI:

- `opus` → `claude-opus-4-6`
- `sonnet` → `claude-sonnet-4-6`
- `haiku` → `claude-haiku-4-5-20251213`

## Authentication

### API key

```bash
read -s ANTHROPIC_API_KEY
echo
export ANTHROPIC_API_KEY
```

### OAuth

```bash
cd rust
./target/debug/claw login
./target/debug/claw logout
```

## Common operational commands

```bash
cd rust
./target/debug/claw status
./target/debug/claw sandbox
./target/debug/claw agents
./target/debug/claw mcp
./target/debug/claw skills
./target/debug/claw system-prompt --cwd .. --date 2026-04-04
```

## Session management

REPL turns are persisted under `.claw/sessions/` in the current workspace.

```bash
cd rust
./target/debug/claw --resume latest
./target/debug/claw --resume latest /status /diff
```

Useful interactive commands include `/help`, `/status`, `/cost`, `/config`, `/session`, `/model`, `/permissions`, and `/export`.

## Config file resolution order

Runtime config is loaded in this order, with later entries overriding earlier ones:

1. `~/.claw.json`
2. `~/.config/claw/settings.json`
3. `<repo>/.claw.json`
4. `<repo>/.claw/settings.json`
5. `<repo>/.claw/settings.local.json`

## Mock parity harness

The workspace includes a deterministic Anthropic-compatible mock service and parity harness.

```bash
cd rust
./scripts/run_mock_parity_harness.sh
```

Manual mock service startup:

```bash
cd rust
cargo run -p mock-anthropic-service -- --bind 127.0.0.1:0
```

## Verification

```bash
cd rust
cargo test --workspace
```

## Workspace overview

Current Rust crates:

- `api`
- `commands`
- `compat-harness`
- `mock-anthropic-service`
- `plugins`
- `runtime`
- `rusty-claude-cli`
- `telemetry`
- `tools`
