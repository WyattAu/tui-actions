//! Key chords: one or more steps, each with optional modifiers.
//!
//! Syntax follows the vim/helix convention the estate's TUIs already speak:
//! `"ctrl+s"`, `"shift+alt+k"`, `"g g"` (two steps), `":"`, `"f5"`.

use crate::error::ActionsError;
use crate::{Key, KeyEvent, Modifiers};
use core::fmt;

/// A single physical key plus modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Binding {
    /// The key.
    pub key: Key,
    /// Modifiers.
    pub modifiers: Modifiers,
}

impl fmt::Display for Binding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.ctrl {
            f.write_str("ctrl+")?;
        }
        if self.modifiers.alt {
            f.write_str("alt+")?;
        }
        if self.modifiers.shift {
            f.write_str("shift+")?;
        }
        match self.key {
            Key::Char(c) => write!(f, "{c}"),
            Key::Function(n) => write!(f, "f{n}"),
            Key::Enter => f.write_str("enter"),
            Key::Esc => f.write_str("esc"),
            Key::Tab => f.write_str("tab"),
            Key::BackTab => f.write_str("backtab"),
            Key::Backspace => f.write_str("backspace"),
            Key::Delete => f.write_str("delete"),
            Key::Insert => f.write_str("insert"),
            Key::Space => f.write_str("space"),
            Key::Up => f.write_str("up"),
            Key::Down => f.write_str("down"),
            Key::Left => f.write_str("left"),
            Key::Right => f.write_str("right"),
            Key::Home => f.write_str("home"),
            Key::End => f.write_str("end"),
            Key::PageUp => f.write_str("pageup"),
            Key::PageDown => f.write_str("pagedown"),
        }
    }
}

/// One or more steps of a chord. Space-separated steps are a sequence
/// (`"g g"`); modifiers bind with `+` within a step.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyChord {
    steps: Vec<Binding>,
}

impl KeyChord {
    /// Build from parsed steps.
    #[must_use]
    pub fn new(steps: Vec<Binding>) -> Self {
        Self { steps }
    }

    /// Parse a chord: `"ctrl+s"`, `"g g"`, `"shift+alt+k"`.
    ///
    /// # Errors
    /// [`ActionsError::EmptyChord`] for blank input;
    /// [`ActionsError::UnknownKey`] for unrecognized names;
    /// [`ActionsError::FunctionOutOfRange`] for `F0`/`F25`+.
    pub fn parse(src: &str) -> Result<Self, ActionsError> {
        let mut steps = Vec::new();
        for step in src.split_whitespace() {
            let mut modifiers = Modifiers::NONE;
            let mut key_part = step;
            // Peel `+`-joined modifier prefixes.
            while let Some((head, tail)) = key_part.split_once('+') {
                match head.to_ascii_lowercase().as_str() {
                    "ctrl" | "control" | "c" => modifiers.ctrl = true,
                    "alt" | "option" | "meta" | "a" => modifiers.alt = true,
                    "shift" | "s" => modifiers.shift = true,
                    _ => break,
                }
                key_part = tail;
            }
            steps.push(Binding {
                key: parse_key(key_part)?,
                modifiers,
            });
        }
        if steps.is_empty() {
            return Err(ActionsError::EmptyChord);
        }
        Ok(Self { steps })
    }

    /// The steps, in order.
    #[must_use]
    pub fn steps(&self) -> &[Binding] {
        &self.steps
    }

    /// The first step (what the dispatcher compares against each event).
    #[must_use]
    pub fn first(&self) -> Option<Binding> {
        self.steps.first().copied()
    }

    /// True when a fresh event matches this chord's current prefix at
    /// `consumed` steps.
    #[must_use]
    pub fn matches_prefix(&self, events: &[KeyEvent], consumed: usize) -> bool {
        let Some(step) = self.steps.get(consumed) else {
            return false;
        };
        events.last().is_some_and(|evt| {
            evt.key == step.key
                && evt.modifiers.ctrl == step.modifiers.ctrl
                && evt.modifiers.alt == step.modifiers.alt
                && evt.modifiers.shift == step.modifiers.shift
        })
    }

    /// True when the chord is complete for the given consumed prefix.
    #[must_use]
    pub fn is_complete(&self, consumed: usize) -> bool {
        consumed >= self.steps.len()
    }
}

impl fmt::Display for KeyChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let joined: Vec<String> = self.steps.iter().map(|b| b.to_string()).collect();
        f.write_str(&joined.join(" "))
    }
}

fn parse_key(raw: &str) -> Result<Key, ActionsError> {
    if raw.is_empty() {
        return Err(ActionsError::UnknownKey(raw.to_owned()));
    }
    let lower = raw.to_ascii_lowercase();
    let named = match lower.as_str() {
        "enter" | "return" | "cr" => Key::Enter,
        "esc" | "escape" => Key::Esc,
        "tab" => Key::Tab,
        "backtab" | "shtab" => Key::BackTab,
        "backspace" | "bs" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "insert" | "ins" => Key::Insert,
        "space" => Key::Space,
        "up" => Key::Up,
        "down" => Key::Down,
        "left" => Key::Left,
        "right" => Key::Right,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" | "pgup" => Key::PageUp,
        "pagedown" | "pgdn" => Key::PageDown,
        _ => {
            if let Some(rest) = lower.strip_prefix('f') {
                let n: u8 = rest
                    .parse()
                    .map_err(|_| ActionsError::FunctionOutOfRange(raw.to_owned()))?;
                if !(1..=24).contains(&n) {
                    return Err(ActionsError::FunctionOutOfRange(raw.to_owned()));
                }
                return Ok(Key::Function(n));
            }
            let mut chars = raw.chars();
            let c = chars
                .next()
                .ok_or_else(|| ActionsError::UnknownKey(raw.to_owned()))?;
            if chars.next().is_some() {
                return Err(ActionsError::UnknownKey(raw.to_owned()));
            }
            Key::Char(c)
        }
    };
    Ok(named)
}
