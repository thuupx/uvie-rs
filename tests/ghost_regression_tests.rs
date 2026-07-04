// ===========================================================================
// Ghost-character regression tests.
// These cover the root-cause fix: backspace rebuilds the composing state from
// a lossless keystroke log instead of replaying the lossy `self.raw` buffer,
// and `commit_diff` clears `diff_committed` so V-C-V auto-committed text does
// not leak into the next word.
// ===========================================================================

#[cfg(test)]
mod ghost_regression_tests {
    use uvie::UltraFastViEngine;
    use uvie::diff::Diffable;

    fn type_diff(e: &mut UltraFastViEngine, s: &str) -> String {
        let mut screen = String::new();
        for ch in s.chars() {
            let (bs, suffix) = e.feed_diff(ch);
            let sc: Vec<char> = screen.chars().collect();
            screen = sc[..sc.len().saturating_sub(bs)].iter().collect::<String>();
            screen.push_str(suffix);
        }
        screen
    }

    fn backspace(e: &mut UltraFastViEngine, screen: &mut String) {
        let (bs, suffix) = e.backspace_diff();
        let sc: Vec<char> = screen.chars().collect();
        *screen = sc[..sc.len().saturating_sub(bs)].iter().collect::<String>();
        screen.push_str(suffix);
    }

    #[test]
    fn backspace_after_w_cancel_keeps_literal() {
        let mut e = UltraFastViEngine::new();
        let screen = type_diff(&mut e, "owwa");
        assert_eq!(screen, "owa");

        let mut screen = screen;
        backspace(&mut e, &mut screen);
        assert_eq!(
            screen, "ow",
            "backspace after double-w cancel must not reapply horn"
        );
        assert_eq!(e.current_composing_diff(), "ow");
    }

    // Double-tone cancel + backspace semantics (keystroke-order, consistent
    // with "tooi" → "tô" → "to"): "ass" = a + s(sắc) + s(cancel) → visible "as".
    // Backspace undoes the LAST keystroke (the cancel), so the tone is restored
    // → "á". This is consistent with undoing one keystroke everywhere else
    // (e.g. "tôi" + backspace undoes 'i' → "tô", not removing the whole "i").
    #[test]
    fn backspace_after_tone_cancel() {
        let mut e = UltraFastViEngine::new();
        let screen = type_diff(&mut e, "ass");
        assert_eq!(screen, "as");

        let mut screen = screen;
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "á");
        assert_eq!(e.current_composing_diff(), "á");
    }

    #[test]
    fn commit_clears_diff_committed() {
        let mut e = UltraFastViEngine::new();
        type_diff(&mut e, "neeboo"); // V-C-V split → committed "nê"
        assert_eq!(e.committed_text_diff(), "nê");
        e.commit_diff();
        assert_eq!(
            e.committed_text_diff(),
            "",
            "commit must clear diff_committed"
        );
    }

    #[test]
    fn backspace_sequence_to_empty_is_consistent() {
        let mut e = UltraFastViEngine::new();
        let mut screen = type_diff(&mut e, "tooi");
        assert_eq!(screen, "tôi");
        for _ in 0..4 {
            backspace(&mut e, &mut screen);
        }
        assert_eq!(screen, "");
        assert!(!e.is_composing_diff());
    }

    #[test]
    fn backspace_across_vcv_split_boundary() {
        let mut e = UltraFastViEngine::new();
        let mut screen = type_diff(&mut e, "neebo"); // "nêbo", committed "nê"
        assert_eq!(screen, "nêbo");
        // bs1: "nêb", bs2: "nê", bs3: pop committed → "n"
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "nêb");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "nê");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "n");
        // composing is gone once we cross into committed territory
        assert_eq!(e.current_composing_diff(), "");
    }

    #[test]
    fn backspace_after_vcv_then_retype_composes() {
        let mut e = UltraFastViEngine::new();
        let mut screen = type_diff(&mut e, "neebo"); // "nêbo"
        backspace(&mut e, &mut screen); // "nêb"
        assert_eq!(screen, "nêb");
        // Retype 'a' → should compose with 'b', not be raw.
        let (bs, suffix) = e.feed_diff('a');
        let sc: Vec<char> = screen.chars().collect();
        screen = sc[..sc.len().saturating_sub(bs)].iter().collect::<String>();
        screen.push_str(suffix);
        assert!(
            screen.ends_with("ba")
                || screen.ends_with("bá")
                || screen.ends_with("bà")
                || screen.ends_with("bả")
                || screen.ends_with("bã")
                || screen.ends_with("bạ"),
            "retype after backspace must compose, got: {screen}"
        );
        assert_eq!(e.raw_len(), e.raw_chars_len());
    }

    #[test]
    fn backspace_after_mid_nucleus_tone() {
        // Backspace after mid-nucleus tone should rebuild correctly.
        // "thajat" → "thật", backspace → "thậ" → "thạ" → "tha" → "th" → "t" → ""
        let mut e = UltraFastViEngine::new();
        let mut screen = type_diff(&mut e, "thajat");
        assert_eq!(screen, "thật");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "thậ");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "thạ");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "tha");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "th");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "t");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "");
    }

    #[test]
    fn backspace_after_mid_nucleus_oo() {
        // "tojot" → "tột", backspace → "tộ" → "tọ" → "to" → "t" → ""
        let mut e = UltraFastViEngine::new();
        let mut screen = type_diff(&mut e, "tojot");
        assert_eq!(screen, "tột");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "tộ");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "tọ");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "to");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "t");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "");
    }

    #[test]
    fn backspace_after_mid_nucleus_ie() {
        // "ieje" → "iệ", backspace → "iej" → "ie" → "i" → ""
        let mut e = UltraFastViEngine::new();
        let mut screen = type_diff(&mut e, "ieje");
        assert_eq!(screen, "iệ");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "iej");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "ie");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "i");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "");
    }

    #[test]
    fn backspace_after_z_consonant() {
        // "azure" → "azure", backspace should work normally
        let mut e = UltraFastViEngine::new();
        let mut screen = type_diff(&mut e, "azure");
        assert_eq!(screen, "azure");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "azur");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "azu");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "az");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "a");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "");
    }

    #[test]
    fn backspace_mid_nucleus_then_retype() {
        // Backspace mid-nucleus tone then retype should compose again.
        // "thajat" → "thật", backspace to "thạ", retype 't' → "thạt"
        let mut e = UltraFastViEngine::new();
        let mut screen = type_diff(&mut e, "thajat");
        assert_eq!(screen, "thật");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "thậ");
        backspace(&mut e, &mut screen);
        assert_eq!(screen, "thạ");
        // Retype 't' → should reapply coda
        let (bs, suffix) = e.feed_diff('t');
        let sc: Vec<char> = screen.chars().collect();
        screen = sc[..sc.len().saturating_sub(bs)].iter().collect::<String>();
        screen.push_str(suffix);
        assert_eq!(screen, "thạt");
    }
}
