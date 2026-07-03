# UVie Bug Triage & Fix Plan

> Collected from user reports. Triaged against `uvie-rs` v2.0.7 (engine) and `UVieKey` v1.3.1 (macOS app).
> Each item includes: symptom → root cause → fix → repo → files → effort.

Status legend: `[todo]` `[in_progress]` `[done]` `[blocked]`

---

## Branch strategy

One branch per repo (no external contributors, no need to split further). Never push to `master`/`main` directly.

| Repo | Branch | Scope |
| --- | --- | --- |
| `uvie-rs` | `fix/bugfix-round` | Bug #4 (subset), #7, #11 |
| `UVieKey` | `fix/bugfix-round` | Bug #1, #3, #5, #6, #8, #9, #10 |

`uvie-rs` ships a new release tag, then `UVieKey` bumps `uvie-rs-version` and re-fetches the prebuilt lib.

### Deferred (not in this round)

- **Enhancement #1 (flicker)**: deferred. The proposed fix (extend `Shift+Left` selection to all compound apps) makes things worse — users see the text briefly highlighted before being replaced, which is more jarring than the current flicker. Revisit with a different approach (e.g. `CGEventKeyboardEventSetUnicodeString` batch, or coalescing) when there's time to A/B test.
- **Enhancement #2 (FST refactor)**: deferred. The current engine is already a Mealy machine; a formal FST rewrite is high-risk and would need a compiled flat-table to match current perf. Park as research, not a merge target.

---

## A. Engine bugs (`uvie-rs`)

### Bug #11 — Gõ tiếng Việt sau `/` và `\` không bỏ dấu đúng
- **Symptom**: `"/duowcs"` → `"/duowcs"` (literal) thay vì `"/được"`. Sau space thì gõ bình thường.
- **Reproduced**: yes (see `examples/test_slash.rs` trace below).
- **Root cause**: `DiffState::is_word_boundary` in <ref_file file="/Users/thupham/Documents/Workspace/uvie-rs/src/diff.rs" /> only whitelists `.,!?;:"'()[]{}\n\r\t` and whitespace. `/` and `\` are **not** word boundaries, so `feed_diff_core` falls through to `self.feed(ch)` which pushes them as literal consonants into `buf`. The leading `/` then sits in the onset slot, `is_legal_onset(['/', 'd'])` returns false, and `handle_telex_w` rejects every horn candidate because `is_valid_vietnamese()` fails — so `w` is appended as a literal.
- **Trace** (`/dduowcs`):
  ```
  ch='/' out_buf='/'   ← '/' pushed into buf as literal consonant
  ch='d' out_buf='/d'
  ch='d' out_buf='/đ'  ← dd→đ works (is_in_onset check passes for '/')
  ch='u' out_buf='/đu'
  ch='o' out_buf='/đuo'
  ch='w' out_buf='/đuow'  ← w literal: is_valid fails because onset=['/','d']
  ```
- **Fix**: treat any non-letter, non-digit ASCII char as a word boundary in `is_word_boundary`. Minimal safe change: add `/ \ - _ + = * & % # @ ~ ` | < >` to the matcher. Better: replace the whitelist with `!ch.is_alphanumeric() && !is_modifier_or_tone_key(ch)`. Keep `\t\n\r ` via `is_whitespace()`.
- **Files**: <ref_file file="/Users/thupham/Documents/Workspace/uvie-rs/src/diff.rs" /> (`is_word_boundary`).
- **Tests**: add `tests.rs` cases: `"/duowcs" → "/được"`, `"a\\bees" → "a\bê"`, `"http://abc" → "http://abc"` (passthrough), `"a-b" → "a-b"`.
- **Effort**: S (1-2h).

### Bug #4 — Nhiều case gõ rất lạ/sai nhưng không có case cụ thể
- **Symptom**: users report weird output but no concrete repro.
- **Root cause hypothesis**: combination of (a) bug #11 (any punctuation breaks the word), (b) `quick_telex`/`quick_start` not exposed in UI so users hit unexpected expansions, (c) `is_word_boundary` whitelist missing common punctuation (`-`, `_`, `/`, `\`, `@`, etc.).
- **Plan**: fix #11 first; add a debug log mode (see UVieKey bug #3) so users can send exact keystroke traces; collect traces and triage individually.
- **Effort**: M (depends on incoming logs).

### Bug #7 — Thêm quick_telex cases (h→nh, …)
- **Symptom**: user wants more quick-telex expansions beyond `cc→ch, gg→gi, kk→kh, nn→ng, qq→qu, pp→ph, tt→th`. Specifically `hh→nh`.
- **Root cause**: <ref_snippet file="/Users/thupham/Documents/Workspace/uvie-rs/src/engine.rs" lines="256-279" /> hardcodes 7 expansions; no `hh→nh`.
- **Fix**: add `'h' => Some([b'n', b'h'])` to the `expansion` match. Also consider `'b' => Some([b'n', b'h'])`? No — only `hh→nh` is requested. Keep the double-tap pattern (only expand when same letter is typed twice).
- **Open question**: should we also expose `quick_telex` and `quick_start` toggles in UVieKey Settings? Currently neither is wired to the UI — `applyEngineSettings()` in `EventTap.swift` only pushes `inputMethod`, `modernOrthography`, `relaxedCoda`. Need to add `uvie_engine_set_quick_telex` calls + Settings toggles.
- **Files**: <ref_file file="/Users/thupham/Documents/Workspace/uvie-rs/src/engine.rs" /> (`quick_telex` match), <ref_file file="/Users/thupham/Documents/Workspace/uvie-rs/src/tests.rs" /> (new test `quick_telex_hh`).
- **Effort**: S.

---

## B. macOS app bugs (`UVieKey`)

### Bug #1 — Sau login, app chạy nhưng không gõ được tiếng Việt
- **Symptom**: after macOS login, the app is running (menu bar icon visible) but typing produces no Vietnamese transforms.
- **Root cause**: <ref_snippet file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/App/UVieKeyApp.swift" lines="63-68" /> checks `AccessibilityChecker.isTrusted` once at launch. After a fresh login, `AXIsProcessTrustedWithOptions(nil)` can return `true` from a previous session but the `CGEventTap` creation silently fails because the accessibility cache hasn't been refreshed by the system yet. The `eventTap.start()` prints `"EventTap: Failed to create tap"` and returns without retry. There is no recovery path.
- **Fix**:
  1. In `EventTap.start()`, when `CGEvent.tapCreate` returns nil, retry with exponential backoff (e.g. 0.5s, 1s, 2s, 5s, 10s) for up to 60s, re-checking `AccessibilityChecker.isTrusted` each attempt.
  2. Listen for `NSWorkspace.didActivateApplicationNotification` (already observed by `InputMethodManager`) — on first app activation after launch, retry `eventTap.start()` if the tap is nil.
  3. Surface the failure in the menu bar (show a warning badge "!" overlay) so the user knows something is wrong.
- **Files**: <ref_file file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/Core/EventTap.swift" /> (`start()`), <ref_file file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/UI/MenuBarController.swift" /> (warning badge).
- **Effort**: M.

### Bug #6 — Sau onboarding, user phải mở lại app; icon không hiển thị ngay
- **Symptom**: after completing onboarding (granting Accessibility), the user must reopen the app for typing and the menu bar icon to work.
- **Root cause**: <ref_snippet file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/App/UVieKeyApp.swift" lines="81-85" /> calls `eventTap.start()` immediately after `onboardingCompleted`. But macOS does not activate the newly-granted accessibility trust until the process is restarted (known macOS behaviour, especially on 14+). `CGEvent.tapCreate` returns nil, the tap never starts, and the icon stays in its initial state because `inputMethodManager.$isVietnamese` may not have fired yet.
- **Fix**:
  1. After onboarding completes, show an explicit "Restart UVieKey" step in `ReadyStep` (<ref_file file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/UI/OnboardingView.swift" />) with a button that calls `NSApp.terminate(nil)` (LaunchAtLogin will relaunch).
  2. OR: poll `AccessibilityChecker.isTrusted` for up to 30s after onboarding, and only call `eventTap.start()` once trusted returns true; if `tapCreate` still fails, show a modal "Please restart UVieKey" dialog.
  3. Ensure the menu bar icon is drawn synchronously in `setupStatusItem()` (it already is) — verify `updateIcon()` runs on the main thread before the first runloop spin.
- **Files**: <ref_file file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/UI/OnboardingView.swift" />, <ref_file file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/App/UVieKeyApp.swift" />.
- **Effort**: M. Combine with Bug #1 fix (shared retry logic).

### Bug #8 — User đôi khi bị crash
- **Symptom**: occasional crashes, no repro.
- **Root cause hypothesis**:
  - The `EventTap` callback runs on a `DispatchQueue.global(qos: .userInteractive)` runloop, but `_engine` (a class) is touched from that callback AND from `applyEngineSettings()` on the main thread (via `UserDefaults.didChangeNotification`). The Rust `UvieEngine` is `Mutex`-protected, so FFI calls are safe, but `EngineBridge.feed()` allocates `[CChar](repeating: 0, count: 128)` per call — fine. The likely crash is the `Unmanaged<EventTap>.fromOpaque(refcon).takeUnretainedValue()` in the C callback if `self` is deallocated while the tap is still active (race between `stop()` and `deinit`).
  - Secondary candidate: `perfLogHandle` is a `FileHandle?` captured in a closure at module scope; concurrent `write(contentsOf:)` from the runloop thread without locking can crash.
- **Fix**:
  1. In `EventTap.stop()`, ensure `CGEvent.tapEnable(tap:enable:false)` and `CFRunLoopRemoveSource` run on the same runloop that hosts the tap (currently calls `CFRunLoopGetCurrent()` from the calling thread — wrong thread). Move `stop()` to post on the tap's runloop.
  2. Guard `perfLogHandle` writes with a serial queue or `os_unfair_lock`.
  3. Wrap the FFI `feed`/`backspace`/`commit` calls in `catch_unwind` (already done in `ffi.rs`) — verify no `unwrap` in the Swift wrapper.
- **Files**: <ref_file file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/Core/EventTap.swift" /> (`stop()`, `perfLog`).
- **Effort**: M. Needs crash logs to confirm — tie to Bug #3 (log collection).

### Bug #9 — macOS 15 (Sequoia) không mở được app / app không hoạt động
- **Symptom**: on macOS 15, the app won't launch or launches but does nothing.
- **Root cause hypothesis**:
  - `Package.swift` declares `platforms: [.macOS(.v13)]` but uses `swift-tools-version:5.9`. Sequoia ships Swift 6+. The `@_silgen_name` attribute and `UnsafeMutablePointer<CChar>?` bridging should still work, but the `CGEvent.tapCreate` API requires the app to be in `Privacy & Security → Accessibility` AND, on 15+, may require `Input Monitoring` as well. The app only requests Accessibility.
  - Secondary: `LSMinimumSystemVersion` is `13.0` (<ref_file file="/Users/thupham/Documents/Workspace/UVieKey/Info.plist" />) — fine, but the release CI might be building with an older SDK that produces binaries incompatible with the 15.x runtime.
- **Fix**:
  1. Also request `Input Monitoring` permission on macOS 15+ (`IOHIDRequestAccess(kIOHIDRequestTypeListenEvent)` from `IOKit.hid`), with a fallback to the existing Accessibility-only path on 13/14.
  2. Bump `swift-tools-version` to 5.10 and ensure CI builds with Xcode 16+ (the release workflow already uses `macos-latest`).
  3. Add a diagnostic in `OnboardingView` that detects macOS 15+ and shows both permission steps.
- **Files**: <ref_file file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/Utils/AccessibilityChecker.swift" />, <ref_file file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/UI/OnboardingView.swift" />.
- **Effort**: M. Needs a Sequoia test machine.

### Bug #10 — macOS 26 (Tahoe) đã cấp Accessibility nhưng vẫn báo chưa cấp, stuck
- **Symptom**: on macOS 26, even after granting Accessibility in System Settings, the app keeps saying "not granted" and is unusable.
- **Root cause**: <ref_snippet file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/Utils/AccessibilityChecker.swift" lines="5-7" /> uses `AXIsProcessTrustedWithOptions(nil)` which caches the result. On macOS 26 the TCC database refresh is async and the API can return `false` for several seconds after the user toggles the switch. The `pollForAccess` helper polls every 0.5s for 60s, but the `isTrusted` getter used by `applicationDidFinishLaunching` and `OnboardingView.onAppear` is a one-shot read.
- **Fix**:
  1. Replace the one-shot `isTrusted` checks in `applicationDidFinishLaunching` and `OnboardingView.onAppear` with `AccessibilityChecker.pollForAccess` that drives a `@Published` `isTrusted` state.
  2. Add a "Refresh" button in `PermissionStep` that re-checks immediately and also re-opens System Settings.
  3. Detect macOS 26 and show an extra hint: "Restart UVieKey after granting permission" (TCC requires process restart to pick up the grant on some builds).
  4. Consider switching to the `CGEvent.tapCreate` success/failure as the source of truth (if the tap creates, we're trusted; if not, keep polling) — this sidesteps the cached `AXIsProcessTrusted` lie.
- **Files**: <ref_file file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/Utils/AccessibilityChecker.swift" />, <ref_file file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/UI/OnboardingView.swift" />, <ref_file file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/App/UVieKeyApp.swift" />.
- **Effort**: M. Combine with Bug #1 + #6.

### Bug #3 — Thêm tính năng log + nút gửi log
- **Symptom**: user wants a "Send Logs" button in the UI to collect diagnostics.
- **Root cause**: no logging infrastructure. Only `/tmp/uviekey_perf.log` exists (perf only, written from the event tap thread).
- **Fix**:
  1. Introduce a `Logger` (`os.Logger` with subsystem `com.thuupx.UVieKey`) for structured logs: engine config changes, event tap lifecycle, AX injection, app switch resets, FFI errors.
  2. Add a rolling file logger at `~/Library/Logs/UVieKey/uviekey.log` (1MB rotate, keep 3).
  3. In Settings → Nâng cao, add a "Gửi log" button that collects the last 24h of `uviekey.log` + `uviekey_perf.log` + system profile (macOS version, app version, enabled settings) into a `.txt` and presents `NSSharingServicePicker` (Mail / AirDrop / Messages). No telemetry server needed — user sends manually.
  4. Add a debug keystroke trace toggle (off by default) that logs `feed(char, bs, out, composing, committed, raw)` to the file when enabled — this also powers Bug #4 collection.
- **Files**: new `Sources/UVieKey/Utils/Logger.swift`, <ref_file file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/UI/SettingsWindow.swift" /> (`AdvancedPane`), <ref_file file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/Core/EventTap.swift" /> (replace `perfLog` with `Logger`).
- **Effort**: M.

### Bug #5 — Bỏ tính năng copy history
- **Symptom**: user wants clipboard history removed (better apps exist).
- **Root cause**: `ClipboardManager` + `ClipboardPane` + popover clipboard section all reference it.
- **Fix**:
  1. Remove `Sources/UVieKey/Features/ClipboardManager.swift`.
  2. Remove `case .clipboard` from `SettingsTab` and `ClipboardPane` from <ref_file file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/UI/SettingsWindow.swift" />.
  3. Remove the `clipboardSection` from `MenuBarPopoverView` in <ref_file file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/UI/MenuBarController.swift" />.
  4. Remove `ClipboardManager.shared.startObserving()` call in `applicationDidFinishLaunching`.
  5. Remove related `DefaultsKey` entries (`clipboardHistoryEnabled`, `clipboardMaxEntries`, `clipboardAutoSplitEnabled`, `clipboardSplitDelimiter`, `clipboardSplitMinLength`) and their `register(defaults:)` lines.
  6. Leave the keys in `UserDefaults` for one release so existing users don't crash on read; just stop using them.
- **Files**: see above.
- **Effort**: S.

### Bug #2 — User muốn bỏ dấu kiểu truyền thống thay vì kiểu mới
- **Symptom**: user wants the option to place tones the traditional way (e.g. `hoas → hoà` instead of `hoá`).
- **Root cause**: this is **already supported** by the engine — `enable_modern_orthography` defaults to `false` in `UltraFastViEngine::new()`, and the macOS app defaults it to `true` in `applicationDidFinishLaunching`. The setting is exposed in Settings → Nâng cao → "Chính tả hiện đại" (<ref_snippet file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/UI/SettingsWindow.swift" lines="754-764" />).
- **Fix**: no code change. Just confirm the toggle works and update the onboarding/Settings description to make it clearer ("Bật để dùng quy tắc mới `hoas→hoá`; tắt để dùng quy tắc truyền thống `hoas→hoà`").
- **Effort**: XS (docs/label only).

---

## C. Enhancements

### Enhancement #1 — Bớt nháy nháy khi gõ nhanh
- **Symptom**: visible flicker when typing fast, especially in compound apps (Safari, Notes).
- **Root cause**: the current diff path posts synthetic backspaces + a new suffix as separate `CGEvent`s. In compound apps it also sends an empty-char sentinel (`U+202F`) before the backspaces — that is 1 extra visible char that briefly renders. For multi-char replacements (`bs > 1`) the sentinel plus N backspace pairs plus the suffix means `2 + 2N + len(suffix)` events per keystroke, all visible.
- **Fix options** (in order of impact):
  1. **Replace `applyCompoundBackspaces` sentinel with a single `Shift+Left` selection** for the `bs` count, then post the suffix once. Already done for Chromium; extend to all compound apps. Removes the visible sentinel char.
  2. **Coalesce keystrokes**: when two keystrokes arrive within the same runloop tick (< 4ms), feed both into the engine and post a single diff covering both. Requires a micro-queue in `EventTap.handle`.
  3. **Use `CGEventKeyboardEventSetUnicodeString` for the suffix** instead of posting multiple key events per character. One event can carry up to 20 UTF-16 code units.
  4. **Suppress the original keyDown's KeyUp earlier**: currently we return `nil` for keyUp which suppresses, but for keyDown we post synthetic events AFTER returning `nil` — there is a brief window where the cursor has moved backward (from backspaces) before the suffix arrives. Posting all synthetic events in a single `CGEvent.post` batch via `CGEventPostToPid` reduces this.
- **Files**: <ref_file file="/Users/thupham/Documents/Workspace/UVieKey/Sources/UVieKey/Core/EventTap.swift" /> (`applyCompoundBackspaces`, `postText`, `applyBackspaces`).
- **Effort**: L. High-risk; needs A/B testing across Safari/Notes/Chrome/Slack.

### Enhancement #2 — Refactor engine sang Finite State Transducer
- **Question**: can the engine be rewritten as an FST for cleaner state transitions?
- **Analysis**: the current design is already close to a Mealy machine — `Syl` is the state, `process_key` is the transition function, `out_buf` is the output. A formal FST would:
  - Replace the ad-hoc `SylBuf` + `flags` + `tone` with a typed state enum (`OnsetEmpty | OnsetConsonant(c) | NucleusStart | NucleusVowel(v) | CodaStarted | Committed`).
  - Encode the legal transitions in a table (onset/nucleus/coda whitelists become the transition relation).
  - Output is a transducer: `(state, input) → (state', output)`.
- **Trade-offs**:
  - **Pro**: easier to reason about, property-based testable, no `F_LITERAL`/`F_TONE_SET` bit flags, naturally expresses V-C-V split as a state reset.
  - **Con**: the current `bit-flip` model is what gives sub-microsecond latency (no allocation, no enum dispatch). A naive FST with `match` on a state enum will likely be 2-5x slower. To match current perf, the FST must be compiled to a flat `[u8; 256]` transition table keyed by `(state, input)` — essentially what `CLASSIFY_TELEX` already is, but extended to multi-step states.
  - **Risk**: high. The diff layer + V-C-V split + backspace replay depend on the current `key_log`/`raw_chars` model. An FST rewrite would need to preserve those.
- **Recommendation**: do NOT rewrite now. Instead, document the current state model as an FST (it already is one, just not named) and extract a `transition(state, input) → (state', output)` function as a refactor, keeping the `Syl` representation. This is a research spike, not a merge target.
- **Branch**: `chore/fst-refactor-spike` (experiment only).
- **Effort**: XL (research). Not scheduled for this round.

---

## Execution order

1. **uvie-rs `fix/bugfix-round`** — Bug #11, #7, #2 label. Self-contained, ship a new engine release.
2. **UVieKey `fix/bugfix-round`** — Bug #1, #3, #5, #6, #8, #9, #10. Bump `uvie-rs-version` and re-fetch prebuilt lib in `build.sh` after engine release.

Bug #4 is a follow-up bucket fed by logs from Bug #3. Enhancements #1 and #2 are deferred (see Branch strategy).

---

## Repro artefacts

`examples/test_slash.rs` (added to uvie-rs in step 1, kept as a debug example):

```rust
use uvie::{UltraFastViEngine, diff::Diffable};

fn type_seq_diff(e: &mut UltraFastViEngine, s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        let (bs, suffix) = e.feed_diff(ch);
        for _ in 0..bs { out.pop(); }
        out.push_str(suffix);
    }
    out
}

fn main() {
    for input in ["/duowcs", "/duowjc", "duowcs", "\\duowcs", "/dduowcs", " dduowcs"] {
        let mut e = UltraFastViEngine::new();
        println!("{:<14} => {}", input, type_seq_diff(&mut e, input));
    }
}
```

Current (broken) output:
```
/duowcs        => /duowcs
/duowjc        => /duowjc
duowcs         => dước
\duowcs        => \duowcs
/dduowcs       => /đuowcs    ← w literal
 dduowcs       =>  đước       ← works (space is a word boundary)
```

Expected after fix:
```
/duowcs        => /dước       (or /được if user typed dduowcs)
/dduowcs       => /đước
\duowcs        => \dước
```
