//! Thin, dependency-free git access via the `git` CLI.
//!
//! These helpers shell out to the system `git` to discover the repository root,
//! fetch a file's *baseline* content (its index or `HEAD` version), and read its
//! working-tree status. They are the only part of this crate that performs I/O
//! or spawns a process, so callers can cache their results and keep the (much
//! more frequent) line-diff step purely in memory.
//!
//! Every call fails soft: if `git` is missing, the path is outside a repo, or a
//! command errors, the helpers return `None`/defaults rather than panicking, so
//! the UI degrades to "no diff available" instead of breaking.

use std::path::{Path, PathBuf};
use std::process::Command;

/// File-level working-tree status, distilled from `git status --porcelain`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FileStatus {
    /// Whether the file resolves inside a git repository at all.
    pub in_repo: bool,
    /// Whether git tracks the file (committed or staged).
    pub tracked: bool,
    /// Whether the file has staged (index) changes.
    pub staged: bool,
    /// Whether the file has unstaged working-tree changes.
    pub modified: bool,
    /// Whether the file is untracked (new, unknown to git).
    pub untracked: bool,
}

/// Run `git` with `args` in `cwd`, returning trimmed stdout on success.
fn run_git_string(cwd: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git").arg("-C").arg(cwd).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Run `git show <spec>` in `root`, returning raw stdout (file content) on
/// success. Kept separate from [`run_git_string`] so the content is not trimmed.
fn run_git_show(root: &Path, spec: &str) -> Option<String> {
    let out = Command::new("git").arg("-C").arg(root).args(["show", spec]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The directory to run `git -C` in for `path` (its parent if it is a file).
fn anchor_dir(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."))
    }
}

/// Discover the repository root containing `path`, or `None` if it is not in a
/// git repository (or `git` is unavailable).
pub fn discover_repo_root(path: &Path) -> Option<PathBuf> {
    let dir = anchor_dir(path);
    let root = run_git_string(&dir, &["rev-parse", "--show-toplevel"])?;
    let root = root.trim();
    if root.is_empty() { None } else { Some(PathBuf::from(root)) }
}

/// The repo-relative, forward-slash pathspec for `abs_path` under `root`.
fn rel_pathspec(root: &Path, abs_path: &Path) -> Option<String> {
    let abs = abs_path.canonicalize().unwrap_or_else(|_| abs_path.to_path_buf());
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let rel = abs.strip_prefix(&root).ok()?;
    let s = rel.to_string_lossy().replace('\\', "/");
    if s.is_empty() { None } else { Some(s) }
}

/// Fetch the baseline content of `abs_path`: the index version if staged,
/// otherwise the `HEAD` version. Returns `None` when the file is untracked/new
/// (no baseline exists — the whole file is "added") or on error.
pub fn baseline_for_file(root: &Path, abs_path: &Path) -> Option<String> {
    let rel = rel_pathspec(root, abs_path)?;
    // Prefer the index (working-tree diff vs. what is staged), then HEAD.
    run_git_show(root, &format!(":{rel}")).or_else(|| run_git_show(root, &format!("HEAD:{rel}")))
}

/// Read the working-tree [`FileStatus`] of `abs_path`.
pub fn file_status(root: &Path, abs_path: &Path) -> FileStatus {
    let mut status = FileStatus { in_repo: true, ..Default::default() };
    let Some(rel) = rel_pathspec(root, abs_path) else {
        return status;
    };
    let Some(out) = run_git_string(root, &["status", "--porcelain", "--", &rel]) else {
        return status;
    };
    match out.lines().next() {
        // No status line => clean & tracked.
        None => {
            status.tracked = true;
        }
        Some(line) if line.len() >= 2 => {
            let bytes = line.as_bytes();
            let (x, y) = (bytes[0] as char, bytes[1] as char);
            if x == '?' && y == '?' {
                status.untracked = true;
            } else {
                status.tracked = true;
                status.staged = x != ' ' && x != '?';
                status.modified = y != ' ' && y != '?';
            }
        }
        Some(_) => {
            status.tracked = true;
        }
    }
    status
}
