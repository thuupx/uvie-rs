//! Post-commit word editing (`edit_newest_diff`) — LabanKey-style:
//! type a word, commit it with a space, arrow the caret back onto the word
//! end, then type a tone/modifier key to re-render the word in place.
//!
//! Core contract: editing a committed word with key `ch` must produce exactly
//! the same on-screen text as typing the extended raw word fresh.

use uvie::diff::Diffable;
use uvie::{InputMethod, UltraFastViEngine};

/// Screen model driven through the real engine: full on-screen text plus a
/// caret offset. The engine's (backspaces, suffix) diffs apply at the caret;
/// the commit space is posted natively by the host (after the caret).
struct Screen {
    text: String,
    caret: usize,
    engine: UltraFastViEngine,
}

impl Screen {
    fn new(modern: bool) -> Self {
        let mut engine = UltraFastViEngine::new();
        engine.set_input_method(InputMethod::Telex);
        engine.set_modern_orthography(modern);
        Self {
            text: String::new(),
            caret: 0,
            engine,
        }
    }

    /// Apply a (backspaces, suffix) instruction at the caret.
    fn apply(&mut self, bs: usize, out: &str) {
        let kept = self.caret.saturating_sub(bs);
        let head: String = self.text.chars().take(kept).collect();
        let tail: String = self.text.chars().skip(self.caret).collect();
        self.text = format!("{head}{out}{tail}");
        self.caret = kept + out.chars().count();
    }

    fn feed(&mut self, ch: char) {
        let (bs, out) = self.engine.feed_diff(ch);
        let out = out.to_string();
        self.apply(bs, &out);
    }

    /// Type `word` and commit it. The host posts the commit space natively,
    /// so the caret ends up 1 right of the committed word's end.
    fn commit_word(&mut self, word: &str) {
        for c in word.chars() {
            self.feed(c);
        }
        let _ = self.engine.commit_diff();
        self.text.push(' ');
        self.caret += 1;
    }

    /// Arrow-left: the caret steps back 1 char (the engine is not involved).
    fn arrow_left(&mut self) {
        self.caret = self.caret.saturating_sub(1);
    }

    /// Re-enter the newest committed word with `ch` appended to its raw
    /// keystrokes and apply the resulting diff at the caret.
    fn edit(&mut self, ch: char) -> Option<String> {
        let (bs, out) = self.engine.edit_newest_diff(ch)?;
        let out = out.to_string();
        self.apply(bs, &out);
        Some(self.text.clone())
    }

    /// Full engine reset (also clears the committed-word history).
    fn reset(&mut self) {
        self.engine.reset_diff();
        self.text.clear();
        self.caret = 0;
    }
}

/// Fresh-typing oracle: type `word` + edit key `ch` + commit space on a
/// clean engine, and return the full on-screen text.
fn type_fresh(word: &str, ch: char) -> String {
    let mut s = Screen::new(true);
    for ch in format!("{word}{ch}").chars() {
        s.feed(ch);
    }
    let _ = s.engine.commit_diff();
    s.text.push(' ');
    s.text
}

#[test]
fn edit_tone_key_matches_fresh_typing() {
    let cases: &[(&str, char)] = &[
        ("don", 's'),
        ("don", 'f'),
        ("don", 'j'),
        ("don", 'r'),
        ("don", 'x'),
        ("vieetj", 's'),
        ("vieetj", 'f'),
        ("ddoong", 's'),
        ("thuocs", 'f'),
        ("nguonf", 's'),
        ("choas", 'f'),
        ("phoos", 'x'),
        ("dduwowcj", 'f'),
        ("neebo", 's'),
    ];
    for &(word, ch) in cases {
        let fresh = type_fresh(word, ch);
        let mut s = Screen::new(true);
        s.commit_word(word);
        s.arrow_left(); // caret onto the committed word's end
        let Some(edited) = s.edit(ch) else {
            panic!("edit of {word:?} with {ch:?} was not handled");
        };
        assert_eq!(
            edited, fresh,
            "editing {word:?} with {ch:?} must equal fresh typing"
        );
    }
}

#[test]
fn edit_english_override_word() {
    // "good" is in the English override dictionary: the commit shows "good".
    let mut s = Screen::new(true);
    s.commit_word("good");
    assert!(s.text.starts_with("good"), "override must show the raw English word");

    s.arrow_left();
    let edited = s.edit('s').expect("edit handled");
    // Fresh typing of "goods": the override fires at "good", then 's' extends.
    let fresh = type_fresh("good", 's');
    assert_eq!(edited, fresh, "edit must match fresh typing");
    assert!(edited.starts_with("goods"));
}

#[test]
fn edit_targets_newest_committed_word() {
    let mut s = Screen::new(true);
    s.commit_word("vie");
    s.commit_word("con");
    s.arrow_left();
    let edited = s.edit('s').expect("edit handled");
    // Editing targets "con" (the newest), not the first word; the earlier
    // text and the commit space are untouched.
    let fresh = type_fresh("con", 's');
    assert!(edited.ends_with(&fresh), "{edited:?} must end with {fresh:?}");
    assert!(edited.starts_with("vie"), "earlier word must be untouched");
}

#[test]
fn edit_without_committed_word_returns_none() {
    let mut s = Screen::new(true);
    assert!(s.edit('s').is_none(), "empty history must not edit");
    for ch in "don".chars() {
        s.feed(ch);
    }
    // Still composing — no committed word yet.
    assert!(s.edit('s').is_none(), "composing word must not edit");
}

#[test]
fn reset_clears_edit_history() {
    let mut s = Screen::new(true);
    s.commit_word("don");
    s.reset();
    assert!(s.edit('s').is_none(), "reset must clear edit history");
}

#[test]
fn backspace_after_edit_restores_composing_state() {
    let mut s = Screen::new(true);
    s.commit_word("don");
    s.arrow_left();
    let _ = s.edit('s').expect("edit handled");
    // The engine is now composing the edited word; backspace must walk it
    // back through the snapshot stack like normal composing.
    // The engine is now composing the edited word; backspace must walk it
    // back through the snapshot stack like normal composing.
    let (bs, out) = s.engine.backspace_diff();
    let out = out.to_string();
    s.apply(bs, &out);
    assert!(s.text.starts_with("don"), "backspace after edit must restore the pre-edit word, got {:?}", s.text);
}

#[test]
fn ring_eviction_keeps_newest_editable() {
    let mut s = Screen::new(true);
    // Commit more words than the ring capacity (8).
    for i in 0..12 {
        s.commit_word(&format!("wo{i}"));
    }
    s.arrow_left();
    let edited = s.edit('s').expect("edit handled");
    // The newest committed word is "wo11"; editing it must match fresh typing.
    let fresh = type_fresh("wo11", 's');
    assert!(edited.ends_with(&fresh), "{edited:?} must end with {fresh:?}");
}
