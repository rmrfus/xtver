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
Cargo.toml      deps: libc, clap
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

**--mux flag**: if set AND inside tmux (`$TMUX` is set), runs
`tmux display-message -p '#{version}'` and appends tmux version to output as
`<terminal>,tmux <version>`. Comma delimiter — parseable with `cut -d, -f1/2`.
No-op outside tmux.

**Zellij**: intercepts XTVERSION and responds itself — `xtver` returns the Zellij
version string directly, no extra config needed. Zellij has no DCS passthrough,
so the outer terminal is unreachable from inside Zellij. `--mux` is a no-op there.

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

## macOS / Homebrew

Formula lives in `Formula/xtver.rb` in this repo. Users install via:

```sh
brew tap rmrfus/xtver https://github.com/rmrfus/xtver
brew install xtver
```

Homebrew builds from source using `cargo`. The formula url/sha256 must be updated
on every release. Get the sha256 with:

```sh
curl -sL 'https://github.com/rmrfus/xtver/archive/refs/tags/vX.Y.Z.tar.gz' | sha256sum
```

## Constraints

- No async, no tokio. Deps: `libc` (syscalls) + `clap` (CLI). Keep it that way.
- Targets Linux and macOS. All syscalls used (`cfmakeraw`, `poll`, `tcgetattr`) are
  POSIX. The flake only sets up `x86_64-linux`; macOS users use Homebrew or plain cargo.
- CLI flags via clap derive. Keep the surface minimal.
