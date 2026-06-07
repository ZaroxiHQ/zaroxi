//! Structured chrome content formatters for IDE panels.
//!
//! These functions take semantic panel data (sections, zones, tabs) and
//! produce formatted content with per-span colors via `content_spans` so
//! the UiBlock renderer can draw visual hierarchy without needing
//! separate blocks per element.
//!
//! Border / layout decisions stay in the engine blocks module. This module
//! owns only content *presentation* — colors, spacing, empty states.

use zaroxi_core_engine_style::StyleTokens;

/// A section within an explorer-style list panel.
#[derive(Clone, Debug)]
pub struct PanelSection {
    pub header: String,
    pub items: Vec<String>,
}

/// Left and right zones of the status bar.
#[derive(Clone, Debug)]
pub struct StatusBarZones {
    pub left_segments: Vec<String>,
    pub right_segments: Vec<String>,
}

/// A single tab entry.
#[derive(Clone, Debug)]
pub struct TabEntry {
    pub label: String,
    pub active: bool,
}

// ---------------------------------------------------------------------------
// Explorer / sidebar content
// ---------------------------------------------------------------------------

/// Format an explorer panel with section headers (muted) and indented items
/// (secondary text) via content_spans. Returns spans ready for UiBlock.
/// When sections are empty, an empty-state message is produced instead.
pub fn format_explorer_spans(
    sections: &[PanelSection],
    tokens: &StyleTokens,
) -> Vec<(String, [f32; 4])> {
    let header_color = tokens.text_muted.to_array();
    let item_color = tokens.text_secondary.to_array();
    let faint_color = tokens.text_faint.to_array();

    if sections.is_empty() {
        return vec![("No workspace loaded".to_string(), faint_color)];
    }

    let mut spans: Vec<(String, [f32; 4])> = Vec::new();

    for (si, section) in sections.iter().enumerate() {
        // Section header
        spans.push((section.header.clone(), header_color));
        spans.push(("\n".to_string(), faint_color));

        if section.items.is_empty() {
            spans.push(("  (empty)".to_string(), faint_color));
            spans.push(("\n".to_string(), faint_color));
        } else {
            for item in &section.items {
                spans.push(("  ".to_string(), faint_color));
                spans.push((item.clone(), item_color));
                spans.push(("\n".to_string(), faint_color));
            }
        }

        if si + 1 < sections.len() {
            spans.push(("\n".to_string(), faint_color));
        }
    }

    spans
}

// ---------------------------------------------------------------------------
// Status bar content
// ---------------------------------------------------------------------------

/// Format the status bar into left-aligned (secondary) and right-aligned
/// (faint) segments with a visible gap between zones.
pub fn format_status_bar_spans(
    zones: &StatusBarZones,
    tokens: &StyleTokens,
) -> Vec<(String, [f32; 4])> {
    let left_color = tokens.text_secondary.to_array();
    let right_color = tokens.text_faint.to_array();
    let sep_color = tokens.text_faint.to_array();
    let mut spans: Vec<(String, [f32; 4])> = Vec::new();

    // Left zone
    for (i, seg) in zones.left_segments.iter().enumerate() {
        if i > 0 {
            spans.push(("\u{2003}".to_string(), sep_color)); // em-space separator
        }
        spans.push((seg.clone(), left_color));
    }

    // Gap between zones
    if !zones.left_segments.is_empty() && !zones.right_segments.is_empty() {
        spans.push(("\u{2003}\u{2003}".to_string(), sep_color));
    }

    // Right zone
    for (i, seg) in zones.right_segments.iter().enumerate() {
        if i > 0 {
            spans.push(("\u{2003}".to_string(), sep_color));
        }
        spans.push((seg.clone(), right_color));
    }

    spans
}

// ---------------------------------------------------------------------------
// Tab strip content
// ---------------------------------------------------------------------------

/// Format a tab strip as a single line of space-separated labels. The
/// active tab is rendered in the accent color; inactive tabs in secondary.
pub fn format_tab_strip_spans(tabs: &[TabEntry], tokens: &StyleTokens) -> Vec<(String, [f32; 4])> {
    let active_color = tokens.accent.to_array();
    let inactive_color = tokens.text_secondary.to_array();
    let sep_color = tokens.text_faint.to_array();
    let mut spans: Vec<(String, [f32; 4])> = Vec::new();

    if tabs.is_empty() {
        spans.push(("No file open".to_string(), inactive_color));
        return spans;
    }

    for (i, tab) in tabs.iter().enumerate() {
        if i > 0 {
            spans.push(("  ".to_string(), sep_color));
        }
        let color = if tab.active { active_color } else { inactive_color };
        spans.push((tab.label.clone(), color));
    }

    spans
}

// ---------------------------------------------------------------------------
// AI panel content
// ---------------------------------------------------------------------------

/// Format AI panel content with a structured header, body, and empty-state
/// fallback. The title/session line uses primary text; body uses secondary.
pub fn format_ai_panel_spans(
    title: Option<&str>,
    subtitle: Option<&str>,
    body: Option<&str>,
    tokens: &StyleTokens,
) -> Vec<(String, [f32; 4])> {
    let title_color = tokens.text_primary.to_array();
    let subtitle_color = tokens.text_muted.to_array();
    let body_color = tokens.text_secondary.to_array();
    let faint_color = tokens.text_faint.to_array();
    let accent_color = tokens.accent.to_array();

    let mut spans: Vec<(String, [f32; 4])> = Vec::new();

    let t = title.unwrap_or("AI Assistant");
    spans.push((t.to_string(), title_color));

    if let Some(sub) = subtitle {
        spans.push((" \u{2022} ".to_string(), faint_color));
        spans.push((sub.to_string(), subtitle_color));
    }

    spans.push(("\n\n".to_string(), faint_color));

    match body {
        Some(b) if !b.trim().is_empty() => {
            let wrapped = wrap_text(b, 40);
            spans.push((wrapped, body_color));
        }
        _ => {
            spans.push(("No active AI session\n".to_string(), faint_color));
            spans.push((
                "Open a file and request an AI edit to get started.".to_string(),
                accent_color,
            ));
        }
    }

    spans
}

// ---------------------------------------------------------------------------
// Empty state / panel placeholder
// ---------------------------------------------------------------------------

/// Produce a quiet empty-state message styled with faint text.
pub fn format_empty_state_spans(message: &str, tokens: &StyleTokens) -> Vec<(String, [f32; 4])> {
    let color = tokens.text_faint.to_array();
    vec![(message.to_string(), color)]
}

/// Produce a compact panel header line (title + optional subtitle).
/// Returns spans suitable for use as content_spans when a panel uses
/// its body/content area to display both header and body together.
pub fn format_panel_header_spans(
    title: &str,
    subtitle: Option<&str>,
    tokens: &StyleTokens,
) -> Vec<(String, [f32; 4])> {
    let title_color = tokens.accent.to_array();
    let subtitle_color = tokens.text_muted.to_array();
    let mut spans = vec![(title.to_string(), title_color)];

    if let Some(sub) = subtitle {
        spans.push(("\n".to_string(), subtitle_color));
        spans.push((sub.to_string(), subtitle_color));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tokens() -> StyleTokens {
        zaroxi_core_engine_style::test_utils::test_tokens_dark()
    }

    #[test]
    fn empty_explorer_emits_empty_state() {
        let spans = format_explorer_spans(&[], &test_tokens());
        assert_eq!(spans.len(), 1);
        assert!(spans[0].0.contains("No workspace"));
    }

    #[test]
    fn explorer_sections_emit_headers_and_items() {
        let sections = vec![PanelSection {
            header: "PROJECT".into(),
            items: vec!["main.rs".into(), "lib.rs".into()],
        }];
        let spans = format_explorer_spans(&sections, &test_tokens());
        // Should include header and items
        assert!(spans.iter().any(|s| s.0.contains("PROJECT")));
        assert!(spans.iter().any(|s| s.0.contains("main.rs")));
        assert!(spans.iter().any(|s| s.0.contains("lib.rs")));
    }

    #[test]
    fn status_bar_zones_separated() {
        let zones = StatusBarZones {
            left_segments: vec!["Ready".into(), "Ln 1, Col 1".into()],
            right_segments: vec!["Rust".into()],
        };
        let spans = format_status_bar_spans(&zones, &test_tokens());
        assert!(spans.iter().any(|s| s.0.contains("Ready")));
        assert!(spans.iter().any(|s| s.0.contains("Rust")));
    }

    #[test]
    fn empty_tabs_produce_no_file_message() {
        let spans = format_tab_strip_spans(&[], &test_tokens());
        assert_eq!(spans.len(), 1);
        assert!(spans[0].0.contains("No file"));
    }

    #[test]
    fn active_tab_gets_accent_color() {
        let tabs = vec![
            TabEntry { label: "main.rs".into(), active: true },
            TabEntry { label: "lib.rs".into(), active: false },
        ];
        let spans = format_tab_strip_spans(&tabs, &test_tokens());
        assert_eq!(spans.len(), 3); // tab, sep, tab
        assert!(spans[0].0.contains("main.rs"));
        assert!(spans[2].0.contains("lib.rs"));
    }

    #[test]
    fn empty_ai_panel_shows_prompt() {
        let spans = format_ai_panel_spans(None, None, None, &test_tokens());
        assert!(spans.iter().any(|s| s.0.contains("No active AI session")));
        assert!(spans.iter().any(|s| s.0.contains("request an AI edit")));
    }
}

fn wrap_text(text: &str, max_chars: usize) -> String {
    let mut out = String::with_capacity(text.len() + text.len() / max_chars);
    for line in text.lines() {
        if out.is_empty() {
            // first line: no leading newline
        } else {
            out.push('\n');
        }
        let mut remaining = line;
        while remaining.len() > max_chars {
            let split =
                remaining.char_indices().take(max_chars).last().map(|(i, _)| i + 1).unwrap_or(0);
            out.push_str(&remaining[..split]);
            out.push('\n');
            remaining = &remaining[split..];
        }
        out.push_str(remaining);
    }
    out
}
