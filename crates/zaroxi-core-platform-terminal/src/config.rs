//! Terminal session configuration and shell resolution.

use std::path::PathBuf;

/// Configuration for spawning a [`crate::session::TerminalSession`].
///
/// All fields have sane defaults; the desktop layer typically only overrides
/// `cwd` (to the active workspace root) and the initial `rows`/`cols`.
#[derive(Debug, Clone)]
pub struct TerminalConfig {
    /// Explicit shell program. When `None` the shell is resolved from the
    /// environment (`$SHELL` on Unix, `%COMSPEC%` on Windows) with a safe
    /// fallback.
    pub shell: Option<String>,
    /// Arguments passed to the shell program.
    pub args: Vec<String>,
    /// Working directory for the spawned shell. `None` inherits the parent.
    pub cwd: Option<PathBuf>,
    /// Extra environment variables layered on top of the inherited environment.
    pub env: Vec<(String, String)>,
    /// Initial terminal height in character rows.
    pub rows: u16,
    /// Initial terminal width in character columns.
    pub cols: u16,
    /// Number of scrollback lines the emulator retains above the visible grid.
    pub scrollback: usize,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            shell: None,
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
            rows: 24,
            cols: 80,
            scrollback: 10_000,
        }
    }
}

impl TerminalConfig {
    /// Resolve the shell program + argument list for this configuration,
    /// reading the process environment for defaults.
    pub fn resolved_shell(&self) -> (String, Vec<String>) {
        let is_windows = cfg!(windows);
        let env_shell = std::env::var("SHELL").ok();
        let env_comspec = std::env::var("COMSPEC").ok();
        let program = resolve_shell_program(
            self.shell.as_deref(),
            is_windows,
            env_shell.as_deref(),
            env_comspec.as_deref(),
        );
        (program, self.args.clone())
    }
}

/// Pure, testable shell-program resolution.
///
/// Priority: explicit override → platform environment variable → platform
/// fallback (`/bin/sh` on Unix, `cmd.exe` on Windows). Kept free of any real
/// environment access so it can be unit-tested deterministically on every
/// platform.
pub fn resolve_shell_program(
    shell_override: Option<&str>,
    is_windows: bool,
    env_shell: Option<&str>,
    env_comspec: Option<&str>,
) -> String {
    if let Some(s) = shell_override.filter(|s| !s.trim().is_empty()) {
        return s.to_string();
    }
    if is_windows {
        env_comspec
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "cmd.exe".to_string())
    } else {
        env_shell
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "/bin/sh".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_override_wins_on_every_platform() {
        assert_eq!(
            resolve_shell_program(Some("/usr/bin/fish"), false, Some("/bin/bash"), None),
            "/usr/bin/fish"
        );
        assert_eq!(
            resolve_shell_program(Some("pwsh.exe"), true, None, Some("cmd.exe")),
            "pwsh.exe"
        );
    }

    #[test]
    fn unix_prefers_shell_env_then_falls_back() {
        assert_eq!(resolve_shell_program(None, false, Some("/bin/zsh"), None), "/bin/zsh");
        assert_eq!(resolve_shell_program(None, false, None, None), "/bin/sh");
        // Blank env is treated as absent.
        assert_eq!(resolve_shell_program(None, false, Some("  "), None), "/bin/sh");
    }

    #[test]
    fn windows_prefers_comspec_then_falls_back() {
        assert_eq!(
            resolve_shell_program(None, true, None, Some("C:\\Windows\\System32\\cmd.exe")),
            "C:\\Windows\\System32\\cmd.exe"
        );
        assert_eq!(resolve_shell_program(None, true, None, None), "cmd.exe");
    }

    #[test]
    fn default_config_is_sane() {
        let c = TerminalConfig::default();
        assert_eq!(c.rows, 24);
        assert_eq!(c.cols, 80);
        assert!(c.scrollback >= 1000);
        assert!(c.shell.is_none());
    }
}
