//! tui-actions — the input layer every terminal front-end shares (the
//! consolidation target for kestrel-tui, suture-tui, and aileron's hand-rolled
//! keymaps; ADR-0004 §2.4).
//!
//! | Area | Entry points |
//! |---|---|
//! | keys | [`Key`], [`KeyEvent`] — the crate's own event type (no crossterm dep; apps convert at the edge) |
//! | chords | [`KeyChord`] — `"ctrl+s"`, `"shift+alt+k"`, `"g g"` sequences |
//! | actions | [`Action`] — an app-defined vocabulary with stable string identity |
//! | modes | [`ModeStack`] — modal context stack (normal → insert → command → popup) |
//! | registry | [`Keymap`] — bindings, conflict + unbound-key detection, dispatch |
//! | help | [`Keymap::help_rows`] — help panel rows derived from the *live* keymap |
//!
//! ```
//! use tui_actions::{Action, Key, KeyChord, KeyEvent, Keymap, Mode};
//!
//! let mut map = Keymap::new(Mode::Normal);
//! map.bind("ctrl+s", Action::new("file.save"), "Save the document").unwrap();
//!
//! let evt = KeyEvent::ctrl(Key::Char('s'));
//! let hits = map.resolve(&evt);
//! assert_eq!(hits, vec![Action::new("file.save")]);
//!
//! // Help is derived, never hand-maintained:
//! let rows = map.help_rows();
//! assert_eq!(rows[0].1, "file.save");
//! # let _ = (KeyChord::parse("ctrl+s").unwrap(), Mode::Normal);
//! ```
//!
//! # Layer
//!
//! L2 — domain. Zero estate-internal deps and zero terminal deps by design:
//! the value is that three apps can share one table-tested model, so it must
//! not drag `crossterm`, `ratatui`, or anything else in. Toolkit lowering
//! happens in the consuming composition crates.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod action;
mod chord;
mod error;
mod keymap;

pub use action::Action;
pub use chord::KeyChord;
pub use error::ActionsError;
pub use keymap::{HelpRow, Keymap, Mode, ModeStack};

/// A physical/logical key. Terminals deliver characters and a handful of
/// named keys; everything else (modifiers, chords, sequences) is built from
/// these by [`KeyEvent`] and [`KeyChord`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    /// A printable character (case-sensitive as delivered).
    Char(char),
    /// A function key, `F1`–`F24` (`n` is 1-based; out-of-range values are
    /// normalized by the parser).
    Function(u8),
    /// Return/Enter.
    Enter,
    /// Escape.
    Esc,
    /// Tab.
    Tab,
    /// Shift+Tab.
    BackTab,
    /// Backspace.
    Backspace,
    /// Delete.
    Delete,
    /// Insert.
    Insert,
    /// Space (distinct from `Char(' ')` when a terminal reports it so).
    Space,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Home.
    Home,
    /// End.
    End,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
}

/// Modifier flags on an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers {
    /// Shift held.
    pub shift: bool,
    /// Control (or the terminal's ctrl encoding) held.
    pub ctrl: bool,
    /// Alt/Option/Meta held.
    pub alt: bool,
}

impl Modifiers {
    /// No modifiers.
    pub const NONE: Modifiers = Modifiers {
        shift: false,
        ctrl: false,
        alt: false,
    };
    /// Control only.
    pub const CTRL: Modifiers = Modifiers {
        shift: false,
        ctrl: true,
        alt: false,
    };
    /// Alt only.
    pub const ALT: Modifiers = Modifiers {
        shift: false,
        ctrl: false,
        alt: true,
    };
    /// Shift only.
    pub const SHIFT: Modifiers = Modifiers {
        shift: true,
        ctrl: false,
        alt: false,
    };

    /// True when no modifier is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        !self.shift && !self.ctrl && !self.alt
    }
}

/// A key event as this crate models it: apps convert their terminal's event
/// type into this at the edge, which is what keeps the crate terminal-free
/// and headless-testable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    /// The key.
    pub key: Key,
    /// Modifiers held.
    pub modifiers: Modifiers,
}

impl KeyEvent {
    /// A plain key press.
    #[must_use]
    pub const fn plain(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::NONE,
        }
    }

    /// A key press with modifiers.
    #[must_use]
    pub const fn with(key: Key, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }

    /// `ctrl+key`.
    #[must_use]
    pub const fn ctrl(key: Key) -> Self {
        Self::with(key, Modifiers::CTRL)
    }

    /// `alt+key`.
    #[must_use]
    pub const fn alt(key: Key) -> Self {
        Self::with(key, Modifiers::ALT)
    }

    /// `shift+key` — printable characters arrive already shifted, so this is
    /// for named keys and terminals that report shift separately.
    #[must_use]
    pub const fn shift(key: Key) -> Self {
        Self::with(key, Modifiers::SHIFT)
    }
}

impl From<Key> for KeyEvent {
    fn from(key: Key) -> Self {
        Self::plain(key)
    }
}

/// The crate's version, for callers that report it.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
