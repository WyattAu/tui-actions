//! The registry is table-testable by design — that is why it exists
//! (three apps were reimplementing this and none had the checks).
//! Test targets lift the deny-level lints at file scope.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use tui_actions::{
    Action, ActionsError, Key, KeyChord, KeyEvent, Keymap, Mode, ModeStack,
};

fn sample() -> Keymap {
    let mut map = Keymap::new(Mode::Normal);
    map.bind("ctrl+s", Action::new("file.save"), "Save the document")
        .unwrap();
    map.bind("q", Action::new("app.quit"), "Quit").unwrap();
    map.bind("g g", Action::new("view.top"), "Go to top").unwrap();
    map.bind_in(&Mode::Insert, "esc", Action::new("edit.finish"), "Leave insert")
        .unwrap();
    map
}

// -- chord parsing -----------------------------------------------------------------

#[test]
fn chords_parse_the_conventions_tuis_speak() {
    let c = KeyChord::parse("ctrl+shift+alt+k").unwrap();
    assert_eq!(c.steps().len(), 1);
    let step = c.steps().first().unwrap();
    assert_eq!(step.key, Key::Char('k'));
    assert!(step.modifiers.ctrl && step.modifiers.shift && step.modifiers.alt);

    let seq = KeyChord::parse("g g").unwrap();
    assert_eq!(seq.steps().len(), 2, "spaces separate steps, not just modifiers");

    let f = KeyChord::parse("f12").unwrap();
    assert_eq!(f.steps().first().unwrap().key, Key::Function(12));

    let named = KeyChord::parse("ctrl+pageup").unwrap();
    assert_eq!(named.steps().first().unwrap().key, Key::PageUp);
}

#[test]
fn chord_display_round_trips() {
    for src in ["ctrl+s", "q", "g g", "f5", "alt+enter", "esc", "shift+tab"] {
        let chord = KeyChord::parse(src).unwrap();
        let rendered = chord.to_string();
        let again = KeyChord::parse(&rendered).unwrap();
        assert_eq!(chord, again, "{src} must round-trip through its display form");
    }
}

#[test]
fn malformed_chords_are_errors_not_panics() {
    assert_eq!(KeyChord::parse(""), Err(ActionsError::EmptyChord));
    assert_eq!(KeyChord::parse("   "), Err(ActionsError::EmptyChord));
    assert!(matches!(
        KeyChord::parse("f0"),
        Err(ActionsError::FunctionOutOfRange(_))
    ));
    assert!(matches!(
        KeyChord::parse("f99"),
        Err(ActionsError::FunctionOutOfRange(_))
    ));
    // An unrecognized modifier head leaves the whole string as the key,
    // which then fails to parse.
    let unknown = KeyChord::parse("hyper+x");
    assert!(
        matches!(unknown, Err(ActionsError::UnknownKey(_))),
        "got {unknown:?}"
    );
    // "ctrl+" with an empty key part is an error, not a panic.
    assert!(KeyChord::parse("ctrl+").is_err());
}

// -- conflicts and unbinding ----------------------------------------------------------

#[test]
fn duplicate_bindings_are_rejected_with_both_actions() {
    let mut map = sample();
    let err = map
        .bind("ctrl+s", Action::new("file.save_as"), "Save as")
        .unwrap_err();
    match err {
        ActionsError::Conflict { chord, first, second, mode } => {
            assert_eq!(chord, "ctrl+s");
            assert_eq!(first, "file.save");
            assert_eq!(second, "file.save_as");
            assert_eq!(mode, "normal");
        }
        other => panic!("expected a conflict, got {other:?}"),
    }
}

#[test]
fn the_same_chord_may_exist_in_two_modes() {
    let mut map = sample();
    map.bind_in(&Mode::Insert, "ctrl+s", Action::new("insert.save"), "Save")
        .unwrap(); // legal: different mode, different meaning
    assert_eq!(map.resolve(&KeyEvent::ctrl(Key::Char('s'))).len(), 1);
}

#[test]
fn unbinding_removes_the_row() {
    let mut map = sample();
    assert!(map.unbind("q"));
    assert!(!map.unbind("q"), "unbinding twice is a no-op, not a panic");
    assert!(map.resolve(&KeyEvent::plain(Key::Char('q'))).is_empty());
}

// -- dispatch ---------------------------------------------------------------------------

#[test]
fn single_step_chords_resolve() {
    let map = sample();
    let hits = map.resolve(&KeyEvent::ctrl(Key::Char('s')));
    assert_eq!(hits, vec![Action::new("file.save")]);
    assert!(map.resolve(&KeyEvent::plain(Key::Char('z'))).is_empty());
    // Shift must not silently match the unmodified binding.
    assert!(map.resolve(&KeyEvent::shift(Key::Char('s'))).is_empty());
}

#[test]
fn multi_step_chords_complete_and_report_pending() {
    let map = sample();
    let g = KeyEvent::plain(Key::Char('g'));
    assert!(map.is_prefix(&Mode::Normal, &g), "'g' starts the 'g g' sequence");
    assert!(map.complete(&Mode::Normal, &[g, g]).contains(&Action::new("view.top")));
    assert!(map.complete(&Mode::Normal, &[g]).is_empty(), "one 'g' is not the action");
    assert!(map.is_unbound(&Mode::Normal, &[g, g, g]));
}

#[test]
fn unbound_keys_are_detectable() {
    let map = sample();
    assert!(map.is_unbound(&Mode::Normal, &[KeyEvent::plain(Key::Char('z'))]));
    assert!(!map.is_unbound(&Mode::Normal, &[KeyEvent::plain(Key::Char('q'))]));
}

// -- modes ------------------------------------------------------------------------------

#[test]
fn mode_stack_owns_input_by_stack_order() {
    let mut stack = ModeStack::with_base(Mode::Normal);
    assert_eq!(stack.active(), Some(&Mode::Normal));
    stack.push(Mode::Popup);
    assert_eq!(stack.active(), Some(&Mode::Popup));
    assert!(stack.contains(&Mode::Normal), "the base is still in the stack");
    assert_eq!(stack.pop(), Some(Mode::Popup));
    assert_eq!(stack.active(), Some(&Mode::Normal));
    assert_eq!(stack.len(), 1);
    assert!(!stack.is_empty());
}

// -- help derivation ---------------------------------------------------------------------

#[test]
fn help_rows_are_derived_from_the_live_keymap() {
    let rows = sample().help_rows();
    let keys: Vec<&str> = rows.iter().map(|r| r.0.as_str()).collect();
    assert_eq!(keys, vec!["ctrl+s", "g g", "q"], "sorted by keys");
    assert_eq!(rows.first().unwrap().1, "file.save");
    assert_eq!(rows.first().unwrap().2, "Save the document");

    // Insert-mode rows come from the same source, not a hand-written panel.
    let insert_rows = sample().help_rows_in(&Mode::Insert);
    assert_eq!(insert_rows.len(), 1);
    assert_eq!(insert_rows.first().unwrap().1, "edit.finish");
}
