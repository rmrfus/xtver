use std::env;
use std::mem;
use std::os::unix::io::RawFd;
use std::time::{Duration, Instant};

const QUERY_TIMEOUT: Duration = Duration::from_secs(1);

use clap::Parser;

#[derive(Parser)]
#[command(
    about = "Query the terminal emulator's name and version via XTVERSION",
    version
)]
struct Cli {
    /// Also append the tmux version to the output (no-op outside tmux).
    /// Output format: <terminal>,tmux <version>
    #[arg(long)]
    mux: bool,
}

fn main() {
    let cli = Cli::parse();

    if in_hostile_env() {
        std::process::exit(1);
    }

    match query_xtversion() {
        Ok(terminal) => {
            if cli.mux && in_tmux() {
                match tmux_version() {
                    Ok(mux) => println!("{},{}", terminal, mux),
                    Err(e) => {
                        eprintln!("error: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                println!("{}", terminal);
            }
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

fn tmux_version() -> Result<String, String> {
    let out = std::process::Command::new("tmux")
        .args(["display-message", "-p", "#{version}"])
        .output()
        .map_err(|e| format!("tmux: {}", e))?;

    if !out.status.success() {
        return Err("tmux display-message failed".to_string());
    }

    let v = String::from_utf8(out.stdout)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("tmux: invalid utf-8: {}", e))?;

    if v.is_empty() {
        return Err("tmux: empty version string".to_string());
    }

    Ok(format!("tmux {}", v))
}

fn in_tmux() -> bool {
    env::var("TMUX").is_ok()
}

// Apps that intercept the tty but don't respond to XTVERSION.
// Running inside these means we'd just burn the timeout for nothing.
fn in_hostile_env() -> bool {
    // MC_SID  — Midnight Commander subshell
    // VIM_TERMINAL — Vim :terminal
    // INSIDE_EMACS — Emacs term/ansiterm
    // NVIM    — Neovim (terminal buffer sets this to its socket path)
    ["MC_SID", "VIM_TERMINAL", "INSIDE_EMACS", "NVIM"]
        .iter()
        .any(|var| env::var(var).is_ok())
}

fn query_xtversion() -> Result<String, String> {
    let fd = open_tty()?;
    let orig = get_termios(fd)?;
    set_raw_mode(fd, &orig)?;

    let result = do_query(fd);

    // always restore, even if query failed
    let _ = restore_termios(fd, &orig);
    unsafe { libc::close(fd) };

    result
}

fn open_tty() -> Result<RawFd, String> {
    let path = std::ffi::CString::new("/dev/tty").unwrap();
    let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDWR) };
    if fd < 0 {
        Err(format!("cannot open /dev/tty: {}", std::io::Error::last_os_error()))
    } else {
        Ok(fd)
    }
}

fn get_termios(fd: RawFd) -> Result<libc::termios, String> {
    let mut t: libc::termios = unsafe { mem::zeroed() };
    if unsafe { libc::tcgetattr(fd, &mut t) } != 0 {
        return Err(format!("tcgetattr: {}", std::io::Error::last_os_error()));
    }
    Ok(t)
}

fn set_raw_mode(fd: RawFd, orig: &libc::termios) -> Result<(), String> {
    let mut t = *orig;
    unsafe { libc::cfmakeraw(&mut t) };
    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &t) } != 0 {
        return Err(format!("tcsetattr: {}", std::io::Error::last_os_error()));
    }
    Ok(())
}

fn restore_termios(fd: RawFd, orig: &libc::termios) -> Result<(), String> {
    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, orig) } != 0 {
        return Err(format!("tcsetattr restore: {}", std::io::Error::last_os_error()));
    }
    Ok(())
}

fn do_query(fd: RawFd) -> Result<String, String> {
    // In tmux we need DCS passthrough:
    //   ESC P tmux ; ESC ESC [ > q ESC \
    // The inner ESC is doubled because it's inside a DCS string.
    // Requires `set -g allow-passthrough on` in tmux.conf.
    let query: &[u8] = if in_tmux() {
        b"\x1bPtmux;\x1b\x1b[>q\x1b\\"
    } else {
        b"\x1b[>q"
    };

    let n = unsafe { libc::write(fd, query.as_ptr() as *const _, query.len()) };
    if n < 0 {
        return Err(format!("write: {}", std::io::Error::last_os_error()));
    }

    let response = read_until_st(fd, QUERY_TIMEOUT)?;
    parse_response(&response)
}

// Read bytes from fd until String Terminator (ESC \) or timeout.
fn read_until_st(fd: RawFd, timeout: Duration) -> Result<Vec<u8>, String> {
    let deadline = Instant::now() + timeout;
    let mut buf = Vec::new();

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("timeout: terminal did not respond to XTVERSION query".to_string());
        }

        let mut pfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let timeout_ms = remaining.as_millis().min(i32::MAX as u128) as i32;

        let ret = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
        if ret < 0 {
            return Err(format!("poll: {}", std::io::Error::last_os_error()));
        }
        if ret == 0 {
            return Err("timeout: terminal did not respond to XTVERSION query".to_string());
        }

        let mut byte = 0u8;
        let n = unsafe { libc::read(fd, &mut byte as *mut u8 as *mut _, 1) };
        if n < 0 {
            return Err(format!("read: {}", std::io::Error::last_os_error()));
        }
        if n == 0 {
            return Err("unexpected eof reading terminal response".to_string());
        }

        buf.push(byte);

        // ST = ESC \  (0x1b 0x5c)
        if buf.len() >= 2 && buf[buf.len() - 2] == 0x1b && buf[buf.len() - 1] == b'\\' {
            return Ok(buf);
        }

        if buf.len() > 4096 {
            return Err("response too long".to_string());
        }
    }
}

// Parse DCS response: ESC P > | <version string> ESC \
fn parse_response(data: &[u8]) -> Result<String, String> {
    let pos = data
        .windows(2)
        .position(|w| w == b">|")
        .ok_or_else(|| {
            format!(
                "unexpected response format: {:?}",
                String::from_utf8_lossy(data)
            )
        })?;

    let version_bytes = data[pos + 2..]
        .strip_suffix(b"\x1b\\")
        .unwrap_or(&data[pos + 2..]);

    let version = String::from_utf8(version_bytes.to_vec())
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("invalid utf-8 in response: {}", e))?;

    if version.is_empty() {
        return Err("empty version string in response".to_string());
    }

    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wezterm() {
        // WezTerm response
        let input = b"\x1bP>|WezTerm 20240203-110809-5046fc22\x1b\\";
        assert_eq!(parse_response(input).unwrap(), "WezTerm 20240203-110809-5046fc22");
    }

    #[test]
    fn test_parse_xterm() {
        let input = b"\x1bP>|XTerm(379)\x1b\\";
        assert_eq!(parse_response(input).unwrap(), "XTerm(379)");
    }

    #[test]
    fn test_parse_kitty() {
        let input = b"\x1bP>|kitty(0.35.2)\x1b\\";
        assert_eq!(parse_response(input).unwrap(), "kitty(0.35.2)");
    }

    #[test]
    fn test_parse_no_marker() {
        let input = b"\x1bP\x1b\\";
        assert!(parse_response(input).is_err());
    }

    #[test]
    fn test_parse_empty_version() {
        let input = b"\x1bP>|\x1b\\";
        assert!(parse_response(input).is_err());
    }

    #[test]
    fn test_parse_no_st() {
        // Missing trailing ESC \ — should still extract what's there
        let input = b"\x1bP>|XTerm(379)";
        assert_eq!(parse_response(input).unwrap(), "XTerm(379)");
    }

    #[test]
    fn test_hostile_env_each_var() {
        // Run sequentially — we set/unset a var, check, restore.
        // Each var is unique so parallel test threads won't interfere with each other.
        let hostile_vars = ["MC_SID", "VIM_TERMINAL", "INSIDE_EMACS", "NVIM"];
        for var in hostile_vars {
            let was_set = env::var(var).is_ok();
            if !was_set {
                unsafe { env::set_var(var, "1") };
            }
            assert!(in_hostile_env(), "{var} should trigger hostile detection");
            if !was_set {
                unsafe { env::remove_var(var) };
            }
        }
    }

    #[test]
    fn test_not_hostile_without_vars() {
        // Verify we return false when none of the hostile vars are set.
        // Skip if any of them happen to be set in this test environment.
        let hostile_vars = ["MC_SID", "VIM_TERMINAL", "INSIDE_EMACS", "NVIM"];
        if hostile_vars.iter().any(|v| env::var(v).is_ok()) {
            return; // already in a hostile env, nothing to assert
        }
        assert!(!in_hostile_env());
    }
}
