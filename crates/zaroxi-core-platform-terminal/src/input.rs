//! Terminal input encoding.
//!
//! Maps high-level, platform-neutral key events to the byte sequences a shell
//! or TUI running inside the PTY expects. Kept pure (no windowing types) so the
//! desktop layer can translate `winit` events into [`TerminalKey`] and this
//! crate owns the single source of truth for VT encoding — fully unit-testable.

/// Platform-neutral logical key, produced by the interface layer from its
/// native key events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalKey {
    /// A printable character (already resolved for shift/caps by the platform).
    Char(char),
    Enter,
    Backspace,
    Tab,
    Delete,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
}

/// Active keyboard modifiers for a key event.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl KeyModifiers {
    pub fn none() -> Self {
        Self::default()
    }
}

/// Encode a key event into the bytes to write to the PTY.
///
/// `app_cursor` selects between "normal" cursor-key mode (CSI, `ESC [ A`) and
/// "application" cursor-key mode (SS3, `ESC O A`) which is requested by many
/// full-screen programs (vim, less, htop). Returns `None` for keys that produce
/// no input (e.g. an unmodified control key that the caller handles locally).
pub fn encode_key(key: TerminalKey, mods: KeyModifiers, app_cursor: bool) -> Option<Vec<u8>> {
    match key {
        TerminalKey::Char(c) => Some(encode_char(c, mods)),
        TerminalKey::Enter => Some(b"\r".to_vec()),
        TerminalKey::Backspace => {
            // DEL (0x7f) is the conventional Backspace byte for Unix ttys.
            if mods.alt { Some(vec![0x1b, 0x7f]) } else { Some(vec![0x7f]) }
        }
        TerminalKey::Tab => {
            if mods.shift {
                Some(b"\x1b[Z".to_vec()) // back-tab (CBT)
            } else {
                Some(b"\t".to_vec())
            }
        }
        TerminalKey::Escape => Some(vec![0x1b]),
        TerminalKey::Delete => Some(b"\x1b[3~".to_vec()),
        TerminalKey::Insert => Some(b"\x1b[2~".to_vec()),
        TerminalKey::PageUp => Some(b"\x1b[5~".to_vec()),
        TerminalKey::PageDown => Some(b"\x1b[6~".to_vec()),
        TerminalKey::Up => Some(cursor_seq(b'A', app_cursor)),
        TerminalKey::Down => Some(cursor_seq(b'B', app_cursor)),
        TerminalKey::Right => Some(cursor_seq(b'C', app_cursor)),
        TerminalKey::Left => Some(cursor_seq(b'D', app_cursor)),
        TerminalKey::Home => Some(cursor_seq(b'H', app_cursor)),
        TerminalKey::End => Some(cursor_seq(b'F', app_cursor)),
    }
}

/// `ESC O <final>` in application-cursor mode, `ESC [ <final>` otherwise.
fn cursor_seq(final_byte: u8, app_cursor: bool) -> Vec<u8> {
    if app_cursor { vec![0x1b, b'O', final_byte] } else { vec![0x1b, b'[', final_byte] }
}

/// Encode a printable character honoring Ctrl/Alt modifiers.
fn encode_char(c: char, mods: KeyModifiers) -> Vec<u8> {
    if mods.ctrl
        && let Some(b) = control_byte(c)
    {
        let mut out = Vec::with_capacity(2);
        if mods.alt {
            out.push(0x1b);
        }
        out.push(b);
        return out;
    }
    let mut buf = [0u8; 4];
    let s = c.encode_utf8(&mut buf);
    if mods.alt {
        // Meta/Alt sends ESC then the character (the common xterm behavior).
        let mut out = Vec::with_capacity(s.len() + 1);
        out.push(0x1b);
        out.extend_from_slice(s.as_bytes());
        out
    } else {
        s.as_bytes().to_vec()
    }
}

/// Map a character to its ASCII control byte when Ctrl is held.
/// `Ctrl+A..Ctrl+Z` → 0x01..0x1a, plus the standard symbolic control codes.
fn control_byte(c: char) -> Option<u8> {
    let lc = c.to_ascii_lowercase();
    match lc {
        'a'..='z' => Some((lc as u8) - b'a' + 1),
        ' ' | '@' => Some(0x00), // Ctrl+Space / Ctrl+@ → NUL
        '[' => Some(0x1b),       // Ctrl+[ → ESC
        '\\' => Some(0x1c),
        ']' => Some(0x1d),
        '^' => Some(0x1e),
        '_' | '/' => Some(0x1f),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctrl() -> KeyModifiers {
        KeyModifiers { ctrl: true, ..Default::default() }
    }
    fn alt() -> KeyModifiers {
        KeyModifiers { alt: true, ..Default::default() }
    }

    #[test]
    fn plain_characters_are_utf8() {
        assert_eq!(encode_key(TerminalKey::Char('a'), KeyModifiers::none(), false).unwrap(), b"a");
        assert_eq!(
            encode_key(TerminalKey::Char('é'), KeyModifiers::none(), false).unwrap(),
            "é".as_bytes()
        );
    }

    #[test]
    fn enter_backspace_tab() {
        assert_eq!(encode_key(TerminalKey::Enter, KeyModifiers::none(), false).unwrap(), b"\r");
        assert_eq!(
            encode_key(TerminalKey::Backspace, KeyModifiers::none(), false).unwrap(),
            vec![0x7f]
        );
        assert_eq!(encode_key(TerminalKey::Tab, KeyModifiers::none(), false).unwrap(), b"\t");
    }

    #[test]
    fn control_combinations() {
        // Ctrl+C = ETX (0x03), Ctrl+D = EOT (0x04), Ctrl+L = FF (0x0c), Ctrl+Z = SUB (0x1a)
        assert_eq!(encode_key(TerminalKey::Char('c'), ctrl(), false).unwrap(), vec![0x03]);
        assert_eq!(encode_key(TerminalKey::Char('d'), ctrl(), false).unwrap(), vec![0x04]);
        assert_eq!(encode_key(TerminalKey::Char('l'), ctrl(), false).unwrap(), vec![0x0c]);
        assert_eq!(encode_key(TerminalKey::Char('z'), ctrl(), false).unwrap(), vec![0x1a]);
    }

    #[test]
    fn alt_prefixes_escape() {
        assert_eq!(encode_key(TerminalKey::Char('b'), alt(), false).unwrap(), vec![0x1b, b'b']);
    }

    #[test]
    fn arrows_switch_on_application_cursor_mode() {
        assert_eq!(encode_key(TerminalKey::Up, KeyModifiers::none(), false).unwrap(), b"\x1b[A");
        assert_eq!(encode_key(TerminalKey::Up, KeyModifiers::none(), true).unwrap(), b"\x1bOA");
        assert_eq!(encode_key(TerminalKey::Left, KeyModifiers::none(), false).unwrap(), b"\x1b[D");
    }

    #[test]
    fn navigation_keys() {
        assert_eq!(
            encode_key(TerminalKey::Delete, KeyModifiers::none(), false).unwrap(),
            b"\x1b[3~"
        );
        assert_eq!(encode_key(TerminalKey::Home, KeyModifiers::none(), false).unwrap(), b"\x1b[H");
        assert_eq!(encode_key(TerminalKey::End, KeyModifiers::none(), false).unwrap(), b"\x1b[F");
        assert_eq!(
            encode_key(TerminalKey::PageUp, KeyModifiers::none(), false).unwrap(),
            b"\x1b[5~"
        );
    }
}
