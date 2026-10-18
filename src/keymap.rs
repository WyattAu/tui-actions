//! The registry: modes, bindings, conflict detection, dispatch, help.

use crate::action::Action;
use crate::chord::KeyChord;
use crate::error::ActionsError;
use crate::KeyEvent;
use core::fmt;

/// An input mode. Apps define their own beyond the conventional five
/// (`Custom`) — the conventional names exist because every TUIs' help panel
/// and status bar already speak them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Mode {
    /// Navigation / commands (vim's normal mode; aileron's default).
    Normal,
    /// Text entry.
    Insert,
    /// A `:`-style command line.
    Command,
    /// An open popup/modal owns input.
    Popup,
    /// Incremental search.
    Search,
    /// Anything else.
    Custom(String),
}

impl Mode {
    /// The mode's display name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Mode::Normal => "normal",
            Mode::Insert => "insert",
            Mode::Command => "command",
            Mode::Popup => "popup",
            Mode::Search => "search",
            Mode::Custom(s) => s,
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A help-panel row: `(keys, action, doc)` — derived from the live keymap,
/// never hand-maintained (the failure mode of every hand-rolled help panel).
pub type HelpRow = (String, String, String);

/// The modal context stack. The *top* of the stack owns input; pushing a
/// popup and popping it back is the whole model.
#[derive(Debug, Clone, Default)]
pub struct ModeStack {
    stack: Vec<Mode>,
}

impl ModeStack {
    /// An empty stack.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A stack with a base mode.
    #[must_use]
    pub fn with_base(mode: Mode) -> Self {
        Self { stack: vec![mode] }
    }

    /// Push a mode (now owning input).
    pub fn push(&mut self, mode: Mode) {
        self.stack.push(mode);
    }

    /// Pop the top mode; `None` when the stack is empty.
    pub fn pop(&mut self) -> Option<Mode> {
        self.stack.pop()
    }

    /// The mode that currently owns input (top of stack; `None` if empty).
    #[must_use]
    pub fn active(&self) -> Option<&Mode> {
        self.stack.last()
    }

    /// How many contexts are stacked.
    #[must_use]
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// True when no context is active.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// True when `mode` is anywhere in the stack.
    #[must_use]
    pub fn contains(&self, mode: &Mode) -> bool {
        self.stack.contains(mode)
    }
}

/// One binding.
#[derive(Debug, Clone)]
struct Binding {
    chord: KeyChord,
    action: Action,
    doc: String,
}

/// A keymap: per-mode chord → action bindings with the two checks every
/// hand-rolled keymap forgets — conflicts and unbound keys.
#[derive(Debug, Clone)]
pub struct Keymap {
    default_mode: Mode,
    modes: Vec<(Mode, Vec<Binding>)>,
}

impl Keymap {
    /// An empty keymap whose default mode is `default`.
    #[must_use]
    pub fn new(default: Mode) -> Self {
        Self {
            default_mode: default,
            modes: Vec::new(),
        }
    }

    /// The default mode.
    #[must_use]
    pub fn default_mode(&self) -> &Mode {
        &self.default_mode
    }

    /// Bind a chord in the default mode.
    ///
    /// # Errors
    /// [`ActionsError`] on malformed chords and on conflicts (a chord already
    /// bound in this mode — the check that keeps vim muscle memory honest).
    pub fn bind(
        &mut self,
        chord: &str,
        action: Action,
        doc: &str,
    ) -> Result<(), ActionsError> {
        let mode = self.default_mode.clone();
        self.bind_in(&mode, chord, action, doc)
    }

    /// Bind a chord in an explicit mode.
    ///
    /// # Errors
    /// As [`Keymap::bind`], with the conflict reported for `mode`.
    pub fn bind_in(
        &mut self,
        mode: &Mode,
        chord: &str,
        action: Action,
        doc: &str,
    ) -> Result<(), ActionsError> {
        let parsed = KeyChord::parse(chord)?;
        let bucket = match self.modes.iter_mut().find(|(m, _)| m == mode) {
            Some((_, b)) => b,
            None => {
                self.modes.push((mode.clone(), Vec::new()));
                match self.modes.iter_mut().find(|(m, _)| m == mode) {
                    Some((_, b)) => b,
                    None => return Err(ActionsError::EmptyChord), // unreachable: just pushed
                }
            }
        };
        if let Some(existing) = bucket.iter().find(|b| b.chord == parsed) {
            return Err(ActionsError::Conflict {
                chord: chord.to_owned(),
                first: existing.action.to_string(),
                second: action.to_string(),
                mode: mode.to_string(),
            });
        }
        bucket.push(Binding {
            chord: parsed,
            action,
            doc: doc.to_owned(),
        });
        Ok(())
    }

    /// Remove a binding by chord in the default mode.
    pub fn unbind(&mut self, chord: &str) -> bool {
        let Ok(parsed) = KeyChord::parse(chord) else {
            return false;
        };
        let Some((_, bucket)) = self.modes.iter_mut().find(|(m, _)| *m == self.default_mode) else {
            return false;
        };
        let before = bucket.len();
        bucket.retain(|b| b.chord != parsed);
        bucket.len() != before
    }

    /// Actions bound to `event` in the default mode (multi-step chords are
    /// not matched here — see [`Keymap::complete`]).
    #[must_use]
    pub fn resolve(&self, event: &KeyEvent) -> Vec<Action> {
        self.resolve_in(&self.default_mode, event)
    }

    /// Actions bound to `event` in `mode`.
    #[must_use]
    pub fn resolve_in(&self, mode: &Mode, event: &KeyEvent) -> Vec<Action> {
        self.bindings(mode)
            .iter()
            .filter(|b| b.chord.steps().len() == 1 && b.chord.matches_prefix(&[*event], 0))
            .map(|b| b.action.clone())
            .collect()
    }

    /// True when any binding in `mode` starts with this event — the app loop
    /// uses it to decide "enter pending-chord state" vs "unbound".
    #[must_use]
    pub fn is_prefix(&self, mode: &Mode, event: &KeyEvent) -> bool {
        self.bindings(mode)
            .iter()
            .any(|b| b.chord.matches_prefix(&[*event], 0) && b.chord.steps().len() > 1)
    }

    /// The actions a *complete* sequence of events resolves to in `mode`.
    #[must_use]
    pub fn complete(&self, mode: &Mode, events: &[KeyEvent]) -> Vec<Action> {
        let consumed = events.len();
        self.bindings(mode)
            .iter()
            .filter(|b| {
                b.chord.steps().len() == consumed
                    && events.iter().enumerate().all(|(i, e)| {
                        b.chord
                            .matches_prefix(&[*e], i)
                    })
            })
            .map(|b| b.action.clone())
            .collect()
    }

    /// True when no binding in `mode` matches the given events — the
    /// "show help / beep" decision.
    #[must_use]
    pub fn is_unbound(&self, mode: &Mode, events: &[KeyEvent]) -> bool {
        let consumed = events.len();
        !self.bindings(mode).iter().any(|b| {
            b.chord.steps().len() == consumed
                && events
                    .iter()
                    .enumerate()
                    .all(|(i, e)| b.chord.matches_prefix(&[*e], i))
        })
    }

    /// Help rows for the default mode, sorted by keys then action.
    #[must_use]
    pub fn help_rows(&self) -> Vec<HelpRow> {
        self.help_rows_in(&self.default_mode)
    }

    /// Help rows for `mode`, sorted by keys then action.
    #[must_use]
    pub fn help_rows_in(&self, mode: &Mode) -> Vec<HelpRow> {
        let mut rows: Vec<HelpRow> = self
            .bindings(mode)
            .iter()
            .map(|b| (b.chord.to_string(), b.action.to_string(), b.doc.clone()))
            .collect();
        rows.sort();
        rows
    }

    /// Every mode that has at least one binding.
    pub fn modes(&self) -> impl Iterator<Item = &Mode> {
        self.modes.iter().map(|(m, _)| m)
    }

    fn bindings(&self, mode: &Mode) -> &[Binding] {
        self.modes
            .iter()
            .find(|(m, _)| m == mode)
            .map(|(_, b)| b.as_slice())
            .unwrap_or(&[])
    }
}
