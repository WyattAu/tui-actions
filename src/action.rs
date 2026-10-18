//! Actions: the app-defined vocabulary a keymap maps onto.

use core::fmt;

/// An application action — a stable string identity (`"file.save"`,
/// `"pane.split.right"`) plus optional display name and help text.
///
/// Apps own the vocabulary; this crate owns the *mechanics* (dispatch,
/// conflicts, help derivation), which is the part three apps were
/// reimplementing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Action {
    id: String,
}

impl Action {
    /// A new action from its dotted identifier.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    /// The action identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// True when two actions are the same action.
    #[must_use]
    pub fn is(&self, other: &Action) -> bool {
        self.id == other.id
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.id)
    }
}

impl From<&str> for Action {
    fn from(id: &str) -> Self {
        Self::new(id)
    }
}
