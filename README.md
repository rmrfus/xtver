# xtver

Query your terminal's XTVERSION and get the result as plain text.

```
$ xtver
WezTerm 20240203-110809-5046fc22
```

Exit code 0 on success, 1 on failure (terminal doesn't support XTVERSION, timeout, no TTY).

## What is XTVERSION

XTVERSION (`CSI > q`) is an escape sequence that asks the terminal to identify itself. The terminal replies with a DCS string containing its name and version. Defined by XTerm, supported by most modern terminals: WezTerm, kitty, Alacritty, foot, XTerm, iTerm2, and others.

Older terminals and terminal multiplexers acting as terminals will not respond — in that case `xtver` exits with code 1 after a 2-second timeout.

## Usage

```
xtver
```

No flags, no arguments. Writes the version string to stdout and exits.

Useful in scripts that need to branch on terminal capabilities:

```sh
if version=$(xtver 2>/dev/null); then
    echo "terminal: $version"
else
    echo "terminal does not support XTVERSION"
fi
```

## tmux

Inside tmux the query is wrapped in a DCS passthrough sequence so it reaches the outer terminal. This requires `allow-passthrough` to be enabled in your tmux config:

```
# tmux.conf
set -g allow-passthrough on
```

Without this, tmux silently drops the passthrough and `xtver` will time out with exit code 1.

Nested tmux sessions are not supported.

## Install

### From source

Requires Rust stable (1.70+).

```sh
cargo install --git https://github.com/rmrfus/xtver
```

### NixOS / nix

A dev shell is provided via `flake.nix`:

```sh
nix develop
cargo build --release
```

Or with direnv:

```sh
echo 'use flake' > .envrc
direnv allow
cargo build --release
```

## How it works

1. Opens `/dev/tty` directly — works regardless of stdin/stdout redirection.
2. Puts the terminal in raw mode, saves original settings.
3. Detects tmux via `$TMUX`; if present, wraps the query in DCS passthrough.
4. Sends `ESC [ > q` and waits up to 2 seconds for a DCS response (`ESC P > | <version> ESC \`).
5. Restores terminal settings unconditionally, parses and prints the version.

Single file, one dependency (`libc`), no async, no tokio, nothing clever.

## License

MIT
