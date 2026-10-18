//! Typed errors. Key chords and actions arrive from user configuration
//! (JSON, TOML, Lua), so parsing is total — a malformed binding is an error
//! a user can read, never a panic.

/// Why a chord or keymap operation failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ActionsError {
    /// A chord string was empty.
    #[error("empty key chord")]
    EmptyChord,
    /// A chord contained an unrecognized key name or modifier.
    #[error("unknown key or modifier in chord `{0}`")]
    UnknownKey(String),
    /// A function key outside `F1`–`F24`.
    #[error("function key out of range: `{0}` (expected F1–F24)")]
    FunctionOutOfRange(String),
    /// Two bindings in the same mode claim the same chord.
    #[error("chord `{chord}` is bound to both `{first}` and `{second}` in mode `{mode}`")]
    Conflict {
        /// The duplicated chord, as written.
        chord: String,
        /// The incumbent action.
        first: String,
        /// The new action.
        second: String,
        /// The mode they clash in.
        mode: String,
    },
}
