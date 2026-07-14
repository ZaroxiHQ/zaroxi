//! Process spawning and OS-level operations for the infrastructure layer.
//!
//! Provides helpers for opening URLs/browsers, spawning external processes,
//! and platform detection. Used by the AI panel for provider account setup
//! and by other infrastructure adapters.

/// Open a URL in the default system browser.
///
/// Uses platform-appropriate mechanisms:
/// - Linux: `xdg-open`
/// - macOS: `open`
/// - Windows: `cmd /c start`
///
/// Returns an error if the URL is invalid or the platform opener fails.
pub fn open_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("URL must not be empty".to_string());
    }

    let result = if cfg!(target_os = "linux") {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Failed to open URL with xdg-open: {e}"))
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Failed to open URL with open: {e}"))
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/c", "start", url])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Failed to open URL on Windows: {e}"))
    } else {
        Err("Unsupported platform for URL opening".to_string())
    };

    if result.is_ok() {
        log::info!("Opened URL in browser: {url}");
    }

    result
}

/// Check if a URL is likely valid (basic format check).
pub fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

/// Spawn a detached child process.
pub fn spawn_detached(command: &str, args: &[&str]) -> Result<std::process::Child, String> {
    std::process::Command::new(command)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn {command}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_valid_url_accepts_http_and_https() {
        assert!(is_valid_url("https://example.com"));
        assert!(is_valid_url("http://localhost:8080"));
        assert!(!is_valid_url("ftp://example.com"));
        assert!(!is_valid_url(""));
    }

    #[test]
    fn open_url_errors_on_empty() {
        assert!(open_url("").is_err());
    }
}
