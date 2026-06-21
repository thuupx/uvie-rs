//! Tone key handling and tone carrier placement logic.

use crate::engine::UltraFastViEngine;
use crate::syllable::{F_HORN, F_LITERAL, F_TONE_SET, Syl};
use crate::tables::{nucleus_tone_target, onset_is_gi, onset_is_qu};
use crate::validation::SyllableValidator;

/// Tone key processing and carrier placement.
pub(crate) trait ToneHandler {
    fn handle_tone_key(&mut self, b: u8, caps: bool);
    fn tone_carrier_idx(&self) -> Option<usize>;
    fn reapply_tone_after_nucleus_change(&mut self);
}

impl ToneHandler for UltraFastViEngine {
    #[inline]
    fn handle_tone_key(&mut self, b: u8, caps: bool) {
        let tone_val = self.mode.tone[b as usize];

        let n = self.buf.len();
        if n == 0 {
            self.buf.push(Syl::literal(b, caps));
            return;
        }

        // Special 'r' heuristic: after certain onset consonants, 'r' is part of
        // the onset (e.g. "tr", "br") - not a tone key. Only apply this when the
        // previous character is actually inside the onset; if it is part of the
        // nucleus or coda, 'r' must remain a tone key.
        if b == b'r' && n > 0 {
            let prev = self.buf.get(n - 1).base;
            let (_, nucleus_start, _, _) = self.partition_syllable();
            let prev_idx = n - 1;
            if prev_idx < nucleus_start
                && matches!(prev, b't' | b'p' | b'f' | b'c' | b'b' | b'd' | b'g' | b'k')
            {
                self.buf.push(Syl::literal(b'r', caps));
                return;
            }
        }

        // If the word is already invalid Vietnamese (passthrough mode), treat
        // tone keys as plain consonants.
        if !self.is_valid_vietnamese() {
            self.buf.push(Syl::consonant(b, caps));
            return;
        }

        let carrier = self.tone_carrier_idx();

        if carrier.is_none() {
            let (_, ns, ne, _) = self.partition_syllable();
            if ne <= ns {
                let has_modified_consonant = (0..self.buf.len()).any(|i| {
                    let s = self.buf.get(i);
                    s.base == b'd' && s.flags & F_HORN != 0
                });
                if has_modified_consonant {
                    if self.raw_len > 0 {
                        self.raw_len -= 1;
                    }
                    return;
                }
            }
            self.buf.push(Syl::literal(b, caps));
            return;
        }

        let carrier_idx = carrier.unwrap();

        let existing = self.buf.get(carrier_idx);
        let already_has_tone = existing.flags & F_TONE_SET != 0;

        if already_has_tone && existing.tone == tone_val && existing.flags & F_LITERAL == 0 {
            // Double-same-tone-key: cancel tone.
            let reverted = {
                let s = self.buf.get(carrier_idx);
                let mut new_s = *s;
                new_s.flags &= !(F_TONE_SET);
                new_s.tone = 0;
                new_s.recompute_out();
                new_s
            };
            self.buf.set(carrier_idx, reverted);

            if self.raw_len > 0 {
                self.raw_len -= 1;
            }
            self.buf.push(Syl::consonant(b, caps));
            return;
        }

        // Override: last tone key wins.
        {
            let s = self.buf.get_mut(carrier_idx);
            s.tone = tone_val;
            s.flags |= F_TONE_SET;
            s.recompute_out();
        }
    }

    #[inline]
    fn tone_carrier_idx(&self) -> Option<usize> {
        let n = self.buf.len();

        let (_onset_end, nucleus_start, nucleus_end, _coda_start) = self.partition_syllable();

        if nucleus_start >= nucleus_end {
            return None;
        }

        let nucleus_len = nucleus_end - nucleus_start;

        let mut nuc: [char; 3] = ['\0'; 3];
        let take = nucleus_len.min(3);
        for i in 0..take {
            nuc[i] = self.buf.get(nucleus_start + i).base_no_tone();
        }
        let nuc_slice = &nuc[..take];

        let onset_raw = self.onset_raw_slice();
        let (eff_nucleus_start, eff_nuc_slice, tone_offset) = if onset_is_qu(onset_raw)
            && nucleus_start < n
            && self.buf.get(nucleus_start).base == b'u'
        {
            let eff_start = nucleus_start + 1;
            if eff_start < nucleus_end {
                let eff_len = (nucleus_end - eff_start).min(3);
                let mut enuc = ['\0'; 3];
                for i in 0..eff_len {
                    enuc[i] = self.buf.get(eff_start + i).base_no_tone();
                }
                (eff_start, enuc, 0usize)
            } else {
                (
                    nucleus_start,
                    {
                        let mut a = ['\0'; 3];
                        a[..take].copy_from_slice(nuc_slice);
                        a
                    },
                    0,
                )
            }
        } else if onset_is_gi(onset_raw)
            && nucleus_start < n
            && self.buf.get(nucleus_start).base == b'i'
        {
            let eff_start = nucleus_start + 1;
            if eff_start < nucleus_end {
                let eff_len = (nucleus_end - eff_start).min(3);
                let mut enuc = ['\0'; 3];
                for i in 0..eff_len {
                    enuc[i] = self.buf.get(eff_start + i).base_no_tone();
                }
                (eff_start, enuc, 0usize)
            } else {
                (
                    nucleus_start,
                    {
                        let mut a = ['\0'; 3];
                        a[..take].copy_from_slice(nuc_slice);
                        a
                    },
                    0,
                )
            }
        } else {
            (
                nucleus_start,
                {
                    let mut a = ['\0'; 3];
                    a[..take].copy_from_slice(nuc_slice);
                    a
                },
                0,
            )
        };

        let eff_len = (nucleus_end - eff_nucleus_start).min(3);
        let eff_slice = &eff_nuc_slice[..eff_len];

        if let Some(target_in_nucleus) = nucleus_tone_target(eff_slice) {
            return Some(eff_nucleus_start + target_in_nucleus + tone_offset);
        }

        if eff_nucleus_start < nucleus_end {
            Some(nucleus_end - 1)
        } else if nucleus_start < nucleus_end {
            Some(nucleus_end - 1)
        } else {
            None
        }
    }

    #[inline]
    fn reapply_tone_after_nucleus_change(&mut self) {
        let (_, nucleus_start, nucleus_end, _) = self.partition_syllable();
        let mut tone_val: Option<u8> = None;
        let mut old_carrier: Option<usize> = None;
        for i in nucleus_start..nucleus_end {
            let s = self.buf.get(i);
            if s.flags & F_TONE_SET != 0 {
                tone_val = Some(s.tone);
                old_carrier = Some(i);
                break;
            }
        }

        let Some(tv) = tone_val else {
            return;
        };
        let new_carrier = self.tone_carrier_idx();

        if new_carrier == old_carrier {
            return;
        }

        if let Some(oc) = old_carrier {
            let s = self.buf.get_mut(oc);
            s.flags &= !F_TONE_SET;
            s.tone = 0;
            s.recompute_out();
        }

        if let Some(new_carrier) = self.tone_carrier_idx() {
            let s = self.buf.get_mut(new_carrier);
            s.tone = tv;
            s.flags |= F_TONE_SET;
            s.recompute_out();
        }
    }
}
