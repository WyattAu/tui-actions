# tui-actions

The terminal input layer the estate's TUIs share (ADR-0004 §2.4): the
consolidation target for kestrel-tui, suture-tui, and aileron's hand-rolled
keymaps.

```rust
use tui_actions::{Action, Key, KeyEvent, Keymap, Mode};

let mut map = Keymap::new(Mode::Normal);
map.bind("ctrl+s", Action::new("file.save"), "Save the document")?;
map.bind("g g", Action::new("view.top"), "Go to top")?;

assert_eq!(
    map.resolve(&KeyEvent::ctrl(Key::Char('s'))),
    vec![Action::new("file.save")]
);
// Help rows are derived from the live keymap, never hand-maintained:
let rows = map.help_rows();
# Ok::<(), tui_actions::ActionsError>(())
```

| Area | What |
|---|---|
| Chords | `"ctrl+s"`, `"shift+alt+k"`, `"f12"`, multi-step `"g g"`: parse + round-trip through display form |
| Actions | app-defined vocabulary with stable string identity |
| Modes | `ModeStack`: modal context (normal → insert → command → popup), top owns input |
| Registry | `Keymap`: conflict detection (both actions named), `unbind`, single- and multi-step dispatch, unbound-key detection |
| Help | `help_rows` / `help_rows_in`: sorted rows derived from the bindings |

## Properties

- **Zero terminal dependencies**: no crossterm, no ratatui. Apps convert
  their event type into `KeyEvent` at the edge, which is what keeps this
  headless-testable; the value is that three apps share one table-tested
  model.
- **Total parsing**: chords come from user configuration; malformed input
  is a typed error, never a panic.
- **L2 domain**: pure logic, table-testable, shared by composition crates.

Licensed under MIT OR Apache-2.0.
