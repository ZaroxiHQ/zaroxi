use std::path::PathBuf;
use std::process::Command;

// ── Picker request kind (extensible for file/save in future) ────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerKind {
    OpenFolder,
}

// ── Structured diagnostics ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerDiagnostics {
    pub session_type: Option<String>,
    pub desktop_environment: Option<String>,
    pub compositor_hint: Option<String>,
    pub portal_available: bool,
    pub portal_hyprland_available: bool,
    pub portal_gtk_available: bool,
    pub portal_kde_available: bool,
    pub zenity_available: bool,
    pub kdialog_available: bool,
    pub qarma_available: bool,
    pub rfd_attempted: bool,
    pub rfd_succeeded: bool,
    pub any_subprocess_attempted: bool,
    pub any_subprocess_succeeded: bool,
    pub fallback_used: Option<String>,
}

impl PickerDiagnostics {
    pub fn probe() -> Self {
        Self {
            session_type: env_nonempty("XDG_SESSION_TYPE"),
            desktop_environment: env_nonempty("XDG_CURRENT_DESKTOP"),
            compositor_hint: Self::detect_compositor_hint(),
            portal_available: binary_exists("xdg-desktop-portal"),
            portal_hyprland_available: binary_exists("xdg-desktop-portal-hyprland"),
            portal_gtk_available: binary_exists("xdg-desktop-portal-gtk"),
            portal_kde_available: binary_exists("xdg-desktop-portal-kde"),
            zenity_available: binary_exists("zenity"),
            kdialog_available: binary_exists("kdialog"),
            qarma_available: binary_exists("qarma"),
            rfd_attempted: false,
            rfd_succeeded: false,
            any_subprocess_attempted: false,
            any_subprocess_succeeded: false,
            fallback_used: None,
        }
    }

    fn detect_compositor_hint() -> Option<String> {
        if env_nonempty("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
            return Some("hyprland".to_string());
        }
        if env_nonempty("SWAYSOCK").is_some() {
            return Some("sway".to_string());
        }
        if env_nonempty("WAYLAND_DISPLAY").is_some() && env_nonempty("DISPLAY").is_none() {
            return Some("wayland (unknown compositor)".to_string());
        }
        None
    }

    pub fn is_wayland(&self) -> bool {
        self.session_type.as_deref() == Some("wayland")
    }

    pub fn is_hyprland(&self) -> bool {
        self.compositor_hint.as_deref() == Some("hyprland")
            || self.desktop_environment.as_ref().map(|d| d.to_lowercase()).as_deref()
                == Some("hyprland")
    }

    pub fn has_file_chooser_portal(&self) -> bool {
        self.portal_gtk_available || self.portal_kde_available
    }

    fn log_diagnostics(&self) {
        log::info!("ZAROXI_PICKER: session_type={:?}", self.session_type);
        log::info!("ZAROXI_PICKER: desktop_environment={:?}", self.desktop_environment);
        log::info!("ZAROXI_PICKER: compositor_hint={:?}", self.compositor_hint);
        log::info!("ZAROXI_PICKER: portal_available={}", self.portal_available);
        log::info!(
            "ZAROXI_PICKER: portal_hyprland={} portal_gtk={} portal_kde={}",
            self.portal_hyprland_available,
            self.portal_gtk_available,
            self.portal_kde_available
        );
        log::info!(
            "ZAROXI_PICKER: zenity={} kdialog={} qarma={}",
            self.zenity_available,
            self.kdialog_available,
            self.qarma_available
        );
    }
}

// ── Outcome enum ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerOutcome {
    Selected(PathBuf),
    Cancelled,
    Unavailable { reason: String, diagnostics: PickerDiagnostics },
}

impl PickerOutcome {
    pub fn is_selected(&self) -> bool {
        matches!(self, PickerOutcome::Selected(_))
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            PickerOutcome::Unavailable { reason, .. } => Some(reason),
            _ => None,
        }
    }

    pub fn diagnostics(&self) -> Option<&PickerDiagnostics> {
        match self {
            PickerOutcome::Unavailable { diagnostics, .. } => Some(diagnostics),
            _ => None,
        }
    }
}

// ── Folder picker trait (testability) ────────────────────────────────

pub trait FolderPicker: Send + Sync {
    fn pick_folder(&self) -> PickerOutcome;
}

// ── System picker ────────────────────────────────────────────────────

pub struct SystemPicker;

impl SystemPicker {
    // ── Backend helpers ──────────────────────────────────────────

    fn try_rfd_pick_folder(diag: &mut PickerDiagnostics) -> Option<PathBuf> {
        diag.rfd_attempted = true;
        log::info!("ZAROXI_PICKER: backend=rfd (primary)");
        let result = rfd::FileDialog::new().pick_folder();
        diag.rfd_succeeded = result.is_some();
        match result {
            Some(ref p) => log::info!("ZAROXI_PICKER: rfd returned path {:?}", p),
            None => log::info!("ZAROXI_PICKER: rfd returned None"),
        }
        result
    }

    fn try_subprocess_pick(
        tool: &str,
        args: &[&str],
        diag: &mut PickerDiagnostics,
    ) -> Option<PathBuf> {
        diag.any_subprocess_attempted = true;
        log::info!("ZAROXI_PICKER: attempting {} (args={:?})", tool, args);
        match Command::new(tool).args(args).output() {
            Ok(output) => {
                let exit_code = output
                    .status
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".to_string());
                log::info!(
                    "ZAROXI_PICKER: {} exit_code={}, stdout_len={}, stderr_len={}",
                    tool,
                    exit_code,
                    output.stdout.len(),
                    output.stderr.len()
                );
                if output.status.success() {
                    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if raw.is_empty() {
                        log::info!(
                            "ZAROXI_PICKER: {} returned empty stdout (user likely cancelled)",
                            tool
                        );
                        None
                    } else {
                        let path = PathBuf::from(&raw);
                        log::info!("ZAROXI_PICKER: {} returned path {:?}", tool, path);
                        diag.fallback_used = Some(tool.to_string());
                        diag.any_subprocess_succeeded = true;
                        Some(path)
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    log::warn!(
                        "ZAROXI_PICKER: {} exited with code {}: {}",
                        tool,
                        exit_code,
                        stderr.trim()
                    );
                    None
                }
            }
            Err(e) => {
                log::warn!("ZAROXI_PICKER: spawn {} failed: {}", tool, e);
                None
            }
        }
    }

    fn try_zenity(diag: &mut PickerDiagnostics) -> Option<PathBuf> {
        if !diag.zenity_available {
            log::info!("ZAROXI_PICKER: zenity not found in PATH, skipping");
            return None;
        }
        Self::try_subprocess_pick(
            "zenity",
            &["--file-selection", "--directory", "--title=Select Workspace Folder"],
            diag,
        )
    }

    fn try_kdialog(diag: &mut PickerDiagnostics) -> Option<PathBuf> {
        if !diag.kdialog_available {
            log::info!("ZAROXI_PICKER: kdialog not found in PATH, skipping");
            return None;
        }
        Self::try_subprocess_pick(
            "kdialog",
            &["--getexistingdirectory", "--title", "Select Workspace Folder"],
            diag,
        )
    }

    fn try_qarma(diag: &mut PickerDiagnostics) -> Option<PathBuf> {
        if !diag.qarma_available {
            log::info!("ZAROXI_PICKER: qarma not found in PATH, skipping");
            return None;
        }
        Self::try_subprocess_pick(
            "qarma",
            &["--file-selection", "--directory", "--title=Select Workspace Folder"],
            diag,
        )
    }

    fn try_env_fallback() -> Option<PathBuf> {
        match std::env::var("ZAROXI_WORKSPACE_PATH") {
            Ok(raw) if !raw.is_empty() => {
                let path = PathBuf::from(&raw);
                log::info!("ZAROXI_PICKER: using ZAROXI_WORKSPACE_PATH env var: {:?}", path);
                Some(path)
            }
            _ => None,
        }
    }

    // ── Unified pick ────────────────────────────────────────────

    pub fn pick(kind: PickerKind, _diag: &mut PickerDiagnostics) -> PickerOutcome {
        match kind {
            PickerKind::OpenFolder => {
                // On all platforms, try rfd first (native on Windows/macOS;
                // portal/GTK on Linux/BSD).
                if let Some(path) = Self::try_rfd_pick_folder(_diag) {
                    return PickerOutcome::Selected(path);
                }

                // Unix subprocess fallbacks (not applicable on Windows).
                if cfg!(unix) {
                    if let Some(path) = Self::try_zenity(_diag) {
                        return PickerOutcome::Selected(path);
                    }
                    if let Some(path) = Self::try_kdialog(_diag) {
                        return PickerOutcome::Selected(path);
                    }
                    if let Some(path) = Self::try_qarma(_diag) {
                        return PickerOutcome::Selected(path);
                    }
                }

                // Env-var fallback (all platforms).
                if let Some(path) = Self::try_env_fallback() {
                    _diag.fallback_used = Some("env:ZAROXI_WORKSPACE_PATH".to_string());
                    return PickerOutcome::Selected(path);
                }

                PickerOutcome::Unavailable {
                    reason: build_unavailable_reason(_diag),
                    diagnostics: _diag.clone(),
                }
            }
        }
    }
}

impl FolderPicker for SystemPicker {
    fn pick_folder(&self) -> PickerOutcome {
        let mut diag = PickerDiagnostics::probe();

        if std::env::var("ZAROXI_DEBUG_CLICK").as_deref() == Ok("1") {
            eprintln!("ZAROXI_PICKER: SystemPicker::pick_folder() called");
        }
        diag.log_diagnostics();

        let outcome = SystemPicker::pick(PickerKind::OpenFolder, &mut diag);

        match &outcome {
            PickerOutcome::Unavailable { reason, .. } => {
                log::warn!("ZAROXI_PICKER: all backends exhausted, returning Unavailable");
                log::warn!("ZAROXI_PICKER: reason: {}", reason);
                eprintln!("ZAROXI_PICKER: UNAVAILABLE — {}", reason);
            }
            _ => {}
        }

        outcome
    }
}

// ── Unavailable reason builder ───────────────────────────────────────

fn build_unavailable_reason(diag: &PickerDiagnostics) -> String {
    let mut parts: Vec<String> = Vec::new();

    let was_attempted = diag.rfd_attempted || diag.any_subprocess_attempted;

    if was_attempted {
        parts.push("folder picker did not produce a path".to_string());
    } else {
        parts.push("no folder picker backend available".to_string());
    }

    // Hyprland/Wayland special guidance.
    if diag.is_hyprland() {
        parts.push(
            "Hyprland detected: xdg-desktop-portal-hyprland does not provide a file chooser"
                .to_string(),
        );
        if !diag.has_file_chooser_portal() {
            parts.push(
                "install xdg-desktop-portal-gtk or xdg-desktop-portal-kde for file chooser support"
                    .to_string(),
            );
        } else {
            parts.push("restart user portal services or your Wayland session".to_string());
        }
    } else if diag.is_wayland() && !diag.has_file_chooser_portal() {
        parts.push(
            "Wayland session: install xdg-desktop-portal-gtk or xdg-desktop-portal-kde".to_string(),
        );
    }

    // Unix-specific suggestions (capability-based, not OS-assuming).
    if cfg!(unix) {
        let mut suggestions: Vec<&str> = Vec::new();
        if !diag.zenity_available && !diag.kdialog_available && !diag.qarma_available {
            suggestions.push("install zenity, kdialog, or qarma for CLI fallback");
        }
        if !diag.portal_available {
            suggestions.push("install xdg-desktop-portal");
        }
        if !suggestions.is_empty() {
            parts.push(format!("suggestions: {}", suggestions.join("; ")));
        }
    }

    if was_attempted {
        parts.push("check logs (ZAROXI_PICKER:) for per-backend details".to_string());
    }

    parts.push("set ZAROXI_WORKSPACE_PATH env var as a workaround".to_string());

    parts.join(". ")
}

// ── Utility helpers ──────────────────────────────────────────────────

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

fn binary_exists(name: &str) -> bool {
    Command::new("which").arg(name).output().map(|o| o.status.success()).unwrap_or(false)
}

// ── Test double ──────────────────────────────────────────────────────

pub struct FakeFolderPicker {
    outcome: PickerOutcome,
}

impl FakeFolderPicker {
    pub fn new(outcome: PickerOutcome) -> Self {
        Self { outcome }
    }

    pub fn selected(path: PathBuf) -> Self {
        Self { outcome: PickerOutcome::Selected(path) }
    }

    pub fn cancelled() -> Self {
        Self { outcome: PickerOutcome::Cancelled }
    }

    pub fn unavailable(reason: &str) -> Self {
        Self {
            outcome: PickerOutcome::Unavailable {
                reason: reason.to_string(),
                diagnostics: PickerDiagnostics::probe(),
            },
        }
    }
}

impl FolderPicker for FakeFolderPicker {
    fn pick_folder(&self) -> PickerOutcome {
        self.outcome.clone()
    }
}

// ── Type alias ───────────────────────────────────────────────────────

pub type DynFolderPicker = std::sync::Arc<dyn FolderPicker>;
