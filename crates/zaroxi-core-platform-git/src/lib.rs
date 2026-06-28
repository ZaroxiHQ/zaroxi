//! Git diff data provider for the Zaroxi UI.
//!
//! This crate turns the live editor buffer plus the file's git baseline into
//! UI-consumable change data: line-range [`DiffHunk`]s, per-line [`ChangedLine`]
//! markers, and a file-level [`FileStatus`]. It is the source future widgets
//! (editor change gutter, semantic minimap lane, cockpit diff overlay, review
//! panels) read from.
//!
//! Design goals (kept deliberately narrow and stable):
//! - **Lightweight / non-blocking:** the only expensive step — asking git for a
//!   file's baseline — runs at most once per file and is cached by
//!   [`GitDiffProvider`]. The per-edit cost is the pure in-memory line diff
//!   ([`diff::diff_lines`]), which trims common prefix/suffix so small edits in
//!   large files stay cheap.
//! - **Fail soft:** missing `git`, non-repo paths, or git errors yield "no diff"
//!   rather than panics.
//! - **Reusable output:** [`FileDiff`] serves both editor overlays (per-line
//!   markers) and summary panels (hunk ranges + status).

pub mod diff;
pub mod repo;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use diff::{ChangeKind, ChangedLine, DiffHunk, changed_lines, diff_lines};
pub use repo::FileStatus;

/// Whether git-diff diagnostics are enabled (`ZAROXI_GIT_TRACE=1`).
pub fn git_trace_enabled() -> bool {
    std::env::var("ZAROXI_GIT_TRACE").as_deref() == Ok("1")
}

/// Strip a single trailing line terminator (`\n`, or `\r\n`) from `s`.
///
/// The git baseline blob virtually always ends in `\n`, but the editor buffer's
/// serialization may not (the open worker strips the trailing newline and the
/// rope does not re-add it). Diffing the two verbatim then reports a phantom
/// `Removed` hunk on the last line of every clean tracked file. Git itself treats
/// a missing final newline specially rather than as a line edit, so normalizing
/// both sides here keeps the change gutter free of that noise.
fn strip_eof_newline(s: &str) -> &str {
    match s.strip_suffix('\n') {
        Some(t) => t.strip_suffix('\r').unwrap_or(t),
        None => s,
    }
}

/// The complete diff result for one file: status plus changed regions.
#[derive(Debug, Clone, Default)]
pub struct FileDiff {
    /// Working-tree status of the file.
    pub status: FileStatus,
    /// Changed regions as line ranges (for hunk navigation / summaries).
    pub hunks: Vec<DiffHunk>,
    /// Flattened per-line markers (for gutter / minimap rendering).
    pub changed_lines: Vec<ChangedLine>,
}

/// Cached git metadata for a single file. `None` baseline means the file is
/// untracked (no baseline → the whole buffer is treated as added).
#[derive(Debug, Clone)]
struct RepoEntry {
    baseline: Option<String>,
    status: FileStatus,
}

/// A caching git diff provider.
///
/// Holds, per file, the discovered repo baseline and status so repeated diffs
/// (e.g. on every keystroke) only re-run the cheap line diff. Call
/// [`GitDiffProvider::invalidate`] after a save/commit to refresh a file's
/// baseline, or [`GitDiffProvider::clear`] to drop all caches.
#[derive(Debug, Default)]
pub struct GitDiffProvider {
    /// `None` value = path resolved but is not inside a git repository.
    cache: HashMap<PathBuf, Option<RepoEntry>>,
}

impl GitDiffProvider {
    /// Create an empty provider.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stable cache key for a path (canonicalized when possible).
    fn key(path: &Path) -> PathBuf {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    }

    /// Diff `current_text` for the file at `abs_path` against its git baseline.
    ///
    /// Returns `None` if the path is not inside a git repository. The first call
    /// for a path performs the git baseline/status lookup and caches it; later
    /// calls reuse the cache and only recompute the in-memory line diff.
    pub fn diff_file(&mut self, abs_path: &Path, current_text: &str) -> Option<FileDiff> {
        let key = Self::key(abs_path);

        if !self.cache.contains_key(&key) {
            let entry = load_entry(abs_path);
            self.cache.insert(key.clone(), entry);
        }

        let entry = self.cache.get(&key)?.as_ref()?;
        let raw_baseline = entry.baseline.as_deref().unwrap_or("");
        // Ignore EOF-newline-only differences (see `strip_eof_newline`): without
        // this, every clean tracked file ending in a newline shows a phantom
        // `Removed` marker on its last line.
        let baseline = strip_eof_newline(raw_baseline);
        let current = strip_eof_newline(current_text);
        let hunks = diff_lines(baseline, current);
        let lines = changed_lines(&hunks);

        if git_trace_enabled() {
            let adds = lines.iter().filter(|c| c.added).count();
            let rems = lines.len() - adds;
            eprintln!(
                "ZAROXI_GIT_TRACE: file={} hunks={} changed_lines={} (+{} -{}) tracked={} modified={} untracked={} baseline_eof_nl={} buffer_eof_nl={}",
                key.display(),
                hunks.len(),
                lines.len(),
                adds,
                rems,
                entry.status.tracked,
                entry.status.modified,
                entry.status.untracked,
                raw_baseline.ends_with('\n'),
                current_text.ends_with('\n'),
            );
        }

        Some(FileDiff { status: entry.status, hunks, changed_lines: lines })
    }

    /// Drop the cached baseline/status for `abs_path` so the next
    /// [`diff_file`](Self::diff_file) re-reads it from git.
    pub fn invalidate(&mut self, abs_path: &Path) {
        self.cache.remove(&Self::key(abs_path));
    }

    /// Drop all cached baselines/status.
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

/// Resolve the repo baseline + status for a path, or `None` if not in a repo.
fn load_entry(abs_path: &Path) -> Option<RepoEntry> {
    let root = repo::discover_repo_root(abs_path)?;
    let baseline = repo::baseline_for_file(&root, abs_path);
    let status = repo::file_status(&root, abs_path);
    Some(RepoEntry { baseline, status })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// Run a git command in `dir`, returning success.
    fn git(dir: &Path, args: &[&str]) -> bool {
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Init a throwaway repo; returns `None` (test skips) if git is unavailable.
    fn temp_repo() -> Option<tempfile::TempDir> {
        let dir = tempfile::tempdir().ok()?;
        if !git(dir.path(), &["init", "-q"]) {
            return None;
        }
        git(dir.path(), &["config", "user.email", "t@t.t"]);
        git(dir.path(), &["config", "user.name", "t"]);
        Some(dir)
    }

    #[test]
    fn non_repo_path_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        // A bare temp dir (no `git init`) is almost certainly outside any repo;
        // if it happens to be inside one, diff_file returns Some — which is also
        // valid — so we only assert it does not panic.
        let mut p = GitDiffProvider::new();
        let file = dir.path().join("nope.txt");
        let _ = p.diff_file(&file, "hello\n");
    }

    #[test]
    fn diffs_committed_file_against_working_tree() {
        let Some(repo) = temp_repo() else {
            eprintln!("git unavailable; skipping git-integration test");
            return;
        };
        let file = repo.path().join("a.txt");
        std::fs::write(&file, "one\ntwo\nthree\n").unwrap();
        assert!(git(repo.path(), &["add", "a.txt"]));
        assert!(git(repo.path(), &["commit", "-qm", "init"]));

        // Working-tree edit: change line 2.
        let current = "one\nTWO\nthree\n";
        let mut provider = GitDiffProvider::new();
        let fd = provider.diff_file(&file, current).expect("file is in a repo");

        assert!(fd.status.tracked, "committed file must read as tracked");
        assert_eq!(fd.hunks.len(), 1, "exactly one changed hunk: {:?}", fd.hunks);
        assert_eq!(fd.hunks[0].kind, ChangeKind::Modified);
        assert_eq!(fd.hunks[0].new_start, 1);
        assert!(fd.changed_lines.iter().any(|c| c.line == 1 && c.added));
    }

    #[test]
    fn trailing_newline_only_difference_is_clean() {
        let Some(repo) = temp_repo() else {
            return;
        };
        let file = repo.path().join("eof.txt");
        // Committed baseline ends in a newline (the normal case).
        std::fs::write(&file, "one\ntwo\nthree\n").unwrap();
        assert!(git(repo.path(), &["add", "eof.txt"]));
        assert!(git(repo.path(), &["commit", "-qm", "eof"]));

        // The editor buffer serializes WITHOUT the trailing newline (the open
        // worker strips it). This must NOT be reported as a removed last line.
        let mut provider = GitDiffProvider::new();
        let fd = provider.diff_file(&file, "one\ntwo\nthree").expect("file is in a repo");
        assert!(fd.hunks.is_empty(), "EOF-newline-only diff must be clean: {:?}", fd.hunks);
        assert!(
            fd.changed_lines.is_empty(),
            "no phantom removed marker on a clean file: {:?}",
            fd.changed_lines
        );
    }

    #[test]
    fn untracked_file_is_all_added() {
        let Some(repo) = temp_repo() else {
            return;
        };
        let file = repo.path().join("new.txt");
        std::fs::write(&file, "x\ny\n").unwrap();
        let mut provider = GitDiffProvider::new();
        let fd = provider.diff_file(&file, "x\ny\n").expect("path is in the repo");
        assert!(fd.status.untracked, "new file must read as untracked");
        // No baseline => whole buffer is added.
        assert_eq!(fd.hunks.len(), 1);
        assert_eq!(fd.hunks[0].kind, ChangeKind::Added);
    }

    #[test]
    fn cache_is_reused_and_invalidatable() {
        let Some(repo) = temp_repo() else {
            return;
        };
        let file = repo.path().join("c.txt");
        std::fs::write(&file, "a\n").unwrap();
        git(repo.path(), &["add", "c.txt"]);
        git(repo.path(), &["commit", "-qm", "c"]);

        let mut provider = GitDiffProvider::new();
        let _ = provider.diff_file(&file, "a\nb\n");
        assert_eq!(provider.cache.len(), 1, "baseline cached after first diff");
        provider.invalidate(&file);
        assert!(provider.cache.is_empty(), "invalidate drops the cache entry");
    }
}
