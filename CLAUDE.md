# xtver

CLI tool that queries the terminal's XTVERSION escape sequence and prints the result.

## What it does

Sends `CSI > q` (`ESC [ > q`) to the terminal via `/dev/tty`, reads the DCS response
(`ESC P > | <version string> ESC \`), prints the version string, exits 0.
On any failure (timeout, no tty, unsupported terminal) — prints error to stderr, exits 1.

## Project structure

Single binary crate. Everything lives in `src/main.rs` — it's small enough that splitting
into modules would be pointless ceremony.

```
src/main.rs     all logic + unit tests
flake.nix       dev shell (rust stable + rust-analyzer + cargo-watch)
.envrc          use flake  (direnv auto-activation)
Cargo.toml      one dep: libc
```

## Key implementation details

**TTY access**: we open `/dev/tty` directly with `libc::open`, not via stdin/stdout.
This is intentional — the tool must work when piped.

**Raw mode**: `libc::cfmakeraw` + `libc::tcgetattr`/`tcsetattr`. Original termios is
always restored, even on error path (restored before `libc::close`).

**Timeout**: `libc::poll` with a 2-second deadline tracked via `Instant`. Polling
per-byte in a loop — not pretty but straightforward and correct.

**tmux detection**: `$TMUX` env var. If set, query is wrapped in DCS passthrough:
`ESC P tmux ; ESC ESC [ > q ESC \` (inner ESC doubled per DCS rules).
Requires `set -g allow-passthrough on` in tmux.conf.

**Parsing**: find `>|` marker in the raw bytes, take everything after it, strip
trailing `ESC \` if present.

## Dev workflow

```sh
# enter dev shell (or just cd if direnv is configured)
nix develop

cargo build
cargo test
cargo run          # needs a real tty — will fail in pipes/CI
cargo watch -x run # live reload during dev
```

## What is and isn't tested

`parse_response()` has unit tests — it's the only function with actual logic.
Everything else is thin syscall glue; test it by running the binary in a real terminal.

## Constraints

- No async, no tokio, no heavy deps. One file, one dep (`libc`). Keep it that way.
- Only targets Linux. The `libc::cfmakeraw` / `libc::poll` path is POSIX but
  the flake only sets up `x86_64-linux`. macOS would need a separate flake target.
- No CLI flags by design. The tool does one thing.
