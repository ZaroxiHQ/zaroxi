// Small presenter-facing view models moved into application-editor
// This file intentionally mirrors the minimal TabEntry/TabStrip API used by
// the presenter in `zaroxi-interface-desktop` so the desktop can consume
// authoritative types from the application layer.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabEntry {
    pub id: String,
    pub display: String,
    pub active: bool,
    pub focused: bool,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TabStrip {
    pub tabs: Vec<TabEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BreadcrumbState {
    pub segments: Vec<String>,
}

impl std::fmt::Display for BreadcrumbState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.segments.is_empty() { Ok(()) } else { write!(f, "{}", self.segments.join(" > ")) }
    }
}

impl BreadcrumbState {
    /// Sample breadcrumb helper used by the desktop until real breadcrumb sources are wired.
    pub fn sample() -> Self {
        BreadcrumbState {
            segments: vec!["src".into(), "app".into(), "desktop".into(), "main.rs".into()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MinimapState {
    pub summary: String,
}

impl TabStrip {
    /// Sample helper: return a small sample TabStrip to be used by the desktop
    /// composer until real application state is wired in. This belongs to the
    /// application crate (owner of the model) and provides a migration-friendly
    /// way for desktop to consume authoritative types.
    pub fn sample() -> Self {
        let opened = vec![
            ("buffer://main.rs".to_string(), "main.rs".to_string()),
            ("buffer://lib.rs".to_string(), "lib.rs".to_string()),
            ("buffer://mod.rs".to_string(), "mod.rs".to_string()),
        ];
        TabStrip::from_opened_and_active(&opened, Some("buffer://main.rs"))
    }
    pub fn from_opened_and_active(opened: &[(String, String)], active: Option<&str>) -> Self {
        let mut tabs: Vec<TabEntry> = Vec::with_capacity(opened.len());
        for (i, pair) in opened.iter().enumerate() {
            let id = pair.0.clone();
            let display = pair.1.clone();
            let active_flag = active.map(|a| a == id).unwrap_or(false);
            tabs.push(TabEntry { id, display, active: active_flag, focused: false, index: i });
        }
        TabStrip { tabs }
    }

    pub fn active_index(&self) -> Option<usize> {
        self.tabs.iter().position(|t| t.active)
    }

    pub fn focused_index(&self) -> Option<usize> {
        self.tabs.iter().position(|t| t.focused)
    }

    pub fn next_active_id(&self, wrap: bool) -> Option<String> {
        let len = self.tabs.len();
        if len == 0 {
            return None;
        }
        if len == 1 {
            return Some(self.tabs[0].id.clone());
        }

        if let Some(idx) = self.active_index() {
            if idx + 1 < len {
                return Some(self.tabs[idx + 1].id.clone());
            } else if wrap {
                return Some(self.tabs[0].id.clone());
            } else {
                return Some(self.tabs[idx].id.clone());
            }
        }

        Some(self.tabs[0].id.clone())
    }

    pub fn prev_active_id(&self, wrap: bool) -> Option<String> {
        let len = self.tabs.len();
        if len == 0 {
            return None;
        }
        if len == 1 {
            return Some(self.tabs[0].id.clone());
        }

        if let Some(idx) = self.active_index() {
            if idx > 0 {
                return Some(self.tabs[idx - 1].id.clone());
            } else if wrap {
                return Some(self.tabs[len - 1].id.clone());
            } else {
                return Some(self.tabs[idx].id.clone());
            }
        }

        Some(self.tabs[len - 1].id.clone())
    }

    pub fn with_active_id(&self, id: &str) -> Self {
        let mut new = self.clone();
        let mut found = false;
        for t in new.tabs.iter_mut() {
            if t.id == id {
                t.active = true;
                found = true;
            } else {
                t.active = false;
            }
        }
        if found { new } else { self.clone() }
    }

    pub fn with_focused_id(&self, id: &str) -> Self {
        let mut new = self.clone();
        let mut found = false;
        for t in new.tabs.iter_mut() {
            if t.id == id {
                t.focused = true;
                found = true;
            } else {
                t.focused = false;
            }
        }
        if found { new } else { self.clone() }
    }

    pub fn focus_next(&self, wrap: bool) -> Self {
        let len = self.tabs.len();
        if len == 0 {
            return self.clone();
        }
        if len == 1 {
            let mut n = self.clone();
            n.tabs[0].focused = true;
            return n;
        }

        if let Some(idx) = self.focused_index() {
            let next_idx = if idx + 1 < len {
                idx + 1
            } else if wrap {
                0
            } else {
                idx
            };
            let mut n = self.clone();
            for (i, t) in n.tabs.iter_mut().enumerate() {
                t.focused = i == next_idx;
            }
            return n;
        }

        let mut n = self.clone();
        for (i, t) in n.tabs.iter_mut().enumerate() {
            t.focused = i == 0;
        }
        n
    }

    pub fn focus_prev(&self, wrap: bool) -> Self {
        let len = self.tabs.len();
        if len == 0 {
            return self.clone();
        }
        if len == 1 {
            let mut n = self.clone();
            n.tabs[0].focused = true;
            return n;
        }

        if let Some(idx) = self.focused_index() {
            let prev_idx = if idx > 0 {
                idx - 1
            } else if wrap {
                len - 1
            } else {
                idx
            };
            let mut n = self.clone();
            for (i, t) in n.tabs.iter_mut().enumerate() {
                t.focused = i == prev_idx;
            }
            return n;
        }

        let mut n = self.clone();
        for (i, t) in n.tabs.iter_mut().enumerate() {
            t.focused = i + 1 == len;
        }
        n
    }
}
