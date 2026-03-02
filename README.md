# xtver

Query your terminal's XTVERSION and get the result as plain text.

```
$ xtver
WezTerm 20240203-110809-5046fc22
```

Exit code 0 on success, 1 on failure (terminal doesn't support XTVERSION, timeout, no TTY).

## Use cases

The main use case is shell profile scripts that need to apply terminal-specific
configuration without hardcoding `$TERM` or `$TERM_PROGRAM` (which are unreliable
over SSH and don't carry version info).

### Terminal-specific shell profile

```zsh
# ~/.zshrc or ~/.bashrc
if _term=$(xtver 2>/dev/null); then
    case "$_term" in
        kitty*)
            # kitten ssh propagates kitty terminfo to the remote host automatically.
            # For everything else (rsync, ansible, plain ssh in scripts) fall back
            # to xterm-256color if kitty terminfo is not available on this machine.
            alias ssh='kitten ssh'
            alias icat='kitten icat'
            infocmp "$TERM" &>/dev/null || export TERM=xterm-256color
            ;;
        Ghostty*)
            # Ghostty has no kitten-style terminfo propagation — fall back when
            # xterm-ghostty terminfo is missing (common on remote hosts via SSH).
            infocmp "$TERM" &>/dev/null || export TERM=xterm-256color
            ;;
        iTerm2*)
            source ~/.iterm2_shell_integration.zsh 2>/dev/null
            alias icat='imgcat'
            ;;
        WezTerm*)
            alias icat='wezterm imgcat'
            ;;
    esac
fi
unset _term
```

This works correctly over SSH because `xtver` queries the actual terminal via
`/dev/tty`, not from environment variables.

## What is XTVERSION

XTVERSION (`CSI > q`) is an escape sequence that asks the terminal to identify itself. The terminal replies with a DCS string containing its name and version. Defined by XTerm, supported by most modern terminals: WezTerm, kitty, Alacritty, foot, XTerm, iTerm2, and others.

Older terminals and terminal multiplexers acting as terminals will not respond — in that case `xtver` exits with code 1 after a 2-second timeout.

## Usage

```
xtver [--mux]
```

Writes the version string to stdout and exits.

Useful in scripts that need to branch on terminal capabilities:

```sh
if version=$(xtver 2>/dev/null); then
    echo "terminal: $version"
else
    echo "terminal does not support XTVERSION"
fi
```

### --mux

When running inside tmux, `--mux` appends the tmux version to the output,
separated by a comma:

```
$ xtver --mux
WezTerm 20240203-110809-5046fc22,tmux 3.3a
```

Easy to parse with `cut`:

```sh
xtver --mux | cut -d, -f1   # terminal
xtver --mux | cut -d, -f2   # tmux
```

Has no effect if used outside tmux — output is the same as without the flag.

## tmux

Inside tmux the query is wrapped in a DCS passthrough sequence so it reaches the outer terminal. This requires `allow-passthrough` to be enabled in your tmux config:

```
# tmux.conf
set -g allow-passthrough on
```

Without this, tmux silently drops the passthrough and `xtver` will time out with exit code 1.

Nested tmux sessions are not supported.

## Zellij

Zellij intercepts XTVERSION and responds with its own version string, so `xtver`
works inside Zellij out of the box — you get the Zellij version:

```
$ xtver
Zellij(4301)
```

However, Zellij does not implement DCS passthrough, so there is currently no way
to query the outer terminal from inside a Zellij session.

`--mux` has no effect inside Zellij — it only appends tmux version when running
inside tmux.

## Install

### Linux (static binary)

Pre-built static musl binaries for x86\_64 and aarch64 (Raspberry Pi) are
attached to every [GitHub release](https://github.com/rmrfus/xtver/releases).

```sh
# amd64
curl -L https://github.com/rmrfus/xtver/releases/latest/download/xtver-x86_64-linux \
  -o /usr/local/bin/xtver && chmod +x /usr/local/bin/xtver

# aarch64 (Raspberry Pi, etc.)
curl -L https://github.com/rmrfus/xtver/releases/latest/download/xtver-aarch64-linux \
  -o /usr/local/bin/xtver && chmod +x /usr/local/bin/xtver
```

### From source

Requires Rust stable (1.70+).

```sh
cargo install --git https://github.com/rmrfus/xtver
```

### macOS (Homebrew)

```sh
brew tap rmrfus/xtver https://github.com/rmrfus/xtver
brew install xtver
```

### NixOS / nix

```sh
nix profile install github:rmrfus/xtver
```

Or run without installing:

```sh
nix run github:rmrfus/xtver
```

Use as a flake input in your NixOS config:

```nix
inputs.xtver.url = "github:rmrfus/xtver";

# then in your packages:
inputs.xtver.packages.${system}.default
```

**Dev shell** (for hacking on xtver itself):

```sh
nix develop       # or just cd in if direnv is configured
cargo build
cargo test
cargo watch -x run
```

## How it works

1. Opens `/dev/tty` directly — works regardless of stdin/stdout redirection.
2. Puts the terminal in raw mode, saves original settings.
3. Detects tmux via `$TMUX`; if present, wraps the query in DCS passthrough.
4. Sends `ESC [ > q` and waits up to 2 seconds for a DCS response (`ESC P > | <version> ESC \`).
5. Restores terminal settings unconditionally, parses and prints the version.

Single file, two dependencies (`libc`, `clap`), no async, no tokio, nothing clever.

Runs on Linux and macOS — all syscalls used (`cfmakeraw`, `poll`, `tcgetattr`) are POSIX.

## License

GPL-3.0-only
