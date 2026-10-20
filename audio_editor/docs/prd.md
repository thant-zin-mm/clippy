# prd.md — Product Requirements Document

_Step 2 of the Idea → Documentation workflow (see `../todo.md`). Status: awaiting review._

## 1. Overview & vision

**Product name (working):** Silence Cutter — a minimal, open-source, local-first audio editor whose single job is removing long silences from very long spoken-word recordings (2–4 hour community discussions).

**Vision.** The user opens the app on their own machine, drops in a recording, reviews auto-detected silences on a waveform, accepts/rejects them with clicks, makes a few manual cuts if needed, and exports a tightened WAV or MP3 — without learning Audacity and without uploading private audio anywhere.

**Architecture shape (decided).** Two fully separate deployables (D20): a **browser frontend** (Chromium + Firefox, plain TS) and a **local Rust backend** that owns decoding, analysis, and export (exact crates in `architecture.md`). In v1 both run on the user's machine and the API is localhost-only, so audio never leaves the machine; the API boundary is designed so a cloud-hosted backend could be swapped in later (post-v1, with privacy review). No accounts, no cloud, no telemetry in v1.

## 2. Goals & non-goals

### Goals (v1)

- G1: Load an MP3 or WAV recording of 2–4 hours on a typical desktop without crashing or freezing the UI.
- G2: Render a zoomable, scrollable waveform with original-time timestamps and a play/pause control that starts from the clicked position (Space toggles, double-click plays).
- G3: Detect silences by absolute dB threshold + minimum duration; display every candidate on the waveform; user accepts/rejects candidates individually by clicking.
- G4: Manual select-and-cut on the waveform; all cuts (manual + accepted detections) live in one non-destructive cut list; playback follows the edited timeline.
- G5: Export the edited audio as WAV or MP3 (original untouched); short fades at joins to avoid clicks.
- G6: Ships as an open-source (GPL-3.0) app with **fully separate deployment** (frontend static app + backend API server): README documents exactly two commands to build and run each part.

### Non-goals (v1)

- NG1: No music-production features: EQ, compression, reverb, effects, multitrack, recording, pitch/time.
- NG2: No copy/paste, insert-silence, fade handles, or volume adjustment.
- NG3: No automatic one-click "remove all silences" — every cut must be user-approved.
- NG4: No cloud sync, accounts, collaboration, sharing links, or telemetry.
- NG5: No mobile support; no Safari support (best-effort only, not tested).
- NG6: No project persistence (no save/resume after closing the app — decided; see §14).
- NG7: No formats beyond MP3 and WAV input; no Opus/FLAC/OGG/M4A in v1.
- NG8: No ML/VAD silence detection — level-based detection only.

## 3. Target users & personas

**P1 — The Discussion Producer (primary).**
A community organizer/volunteer who records panels and Q&A sessions (often 2–4 h), and must publish a listenable version. Not a developer. For v1 (D21, source-build-only) she either follows the README's step-by-step prerequisites (Rust + C toolchain) with a helper's one-time setup, or a release binary arrives post-v1; the product itself must demand nothing more technical than running two commands. Comfortable with waveform players (YouTube-style), impatient with professional DAWs.

**P2 — The Archivist / Researcher (secondary).**
Maintains recordings of meetings, oral histories, lectures. Wants quiet gaps removed for review/transcription efficiency. More tolerant of technical setup; values accuracy over speed — will zoom in to verify no words were clipped.

**P3 — The Open-Source Contributor (enabler, not end-user).**
Someone who finds the project on GitHub, runs it, and improves it. Needs a clean codebase, docs, and tests — supported by `coding-guide.md` and the plan, not by product features.

## 4. User stories & key flows

### US1 — Import
_As P1, I drop a 3-hour MP3 into the app so I can begin editing._

- Acceptance: drag-drop or file picker; file is streamed to the local backend; waveform appears with progress feedback; import completes ≤ ~3 min for a 3 h 128 kbps MP3 (NFR-1); errors (corrupt/unsupported) are shown clearly, app stays usable.

### US2 — Manual review & playback
_As P1, I click a point on the waveform and press Space to hear the audio from there._

- Acceptance: click sets the playhead; Space = play/pause; double-click = play from there; timeline shows original time (hh:mm:ss); current playhead time and "result time" (edited timeline) are both visible; playback skips accepted cuts (US5).

### US3 — Auto-detect
_As P1, I press "Auto-detect silences" so I don't have to hunt for pauses manually._

- Acceptance: with thresholds set (defaults −40 dB, 1.0 s), detection runs (fast — brief busy indicator only); each candidate is drawn as a distinct overlay region on the waveform; a counter shows "N candidates found"; clicking a candidate selects it.

### US4 — Tune detection
_As P2, I adjust the dB threshold and minimum duration and re-run detection until candidates match my judgment._

- Acceptance: threshold (dB) and min duration (s) are editable; a "Measure noise floor" control shows the suggestion computed at import (~noise floor + 12 dB, offered pre-filled, not auto-applied); re-run happens only on explicit button press; a warning is shown that re-run clears current review state ("Re-run replaces all candidates and resets accept/reject decisions. Continue?" — undo can still restore the prior review state, US7).

### US5 — Review candidates
_As P1, I click each candidate on the waveform and decide keep-cut or keep-audio._

- Acceptance: selecting a candidate shows its start/end/duration and the surrounding context (zoom-to-fit the candidate with padding); two buttons: **Cut this** (accept → becomes an entry in the cut list) and **Keep this** (reject → candidate removed or marked kept); already-cut ranges are visually distinct from unreviewed candidates; keyboard Space auditions from just before the candidate.

### US6 — Manual cut
_As P1, I drag-select a range the detector missed and cut it._

- Acceptance: drag on waveform creates a selection; "Cut selection" adds it to the same cut list; overlapping cuts are prevented or merged; selection can also be discarded.

### US7 — Undo
_As P1, I undo a wrong acceptance without re-analyzing._

- Acceptance: Ctrl/Cmd+Z (and a visible undo button) reverses the most recent state change (cut, un-cut, reject, selection-delete, re-run); Ctrl+Shift+Z / redo button reapplies; undo depth covers at least the session's actions or memory-based cap with clear behavior (architecture decides mechanism; requirement is: no action is unrecoverable within a session).

### US8 — Export
_As P1, I export the tightened audio as MP3 and listen to the result._

- Acceptance: choose WAV or MP3 (fixed 192 kbps CBR for MP3); progress shown; exported file skips every cut-range, applies the keep-margin rules, joins with ~5 ms fades; original file on disk is never modified; export completes ≤ ~2× audio duration on a typical desktop (NFR-1); the result plays in any common player with no clicks at joins.

### US9 — Failure & limits
_As P1, I get a clear message if something can't be done._

- Acceptance: unsupported format, corrupt file, disk-full on export, backend not running — each produces a plain-language error with a suggested next step; the app never silently drops audio.

### Flow summary (happy path)

```
drop file → import/analyze waveform → set thresholds (or accept defaults)
→ auto-detect → click through candidates (Cut/Keep) → manual cut (optional)
→ export WAV/MP3 → done (close app; nothing retained)
```

## 5. Functional requirements

Priority: **M** = must (v1), **S** = should, **C** = could (explicitly not committed for v1).

| ID | Requirement | Pri | Acceptance criteria |
|----|-------------|-----|---------------------|
| FR-01 | Import MP3 or WAV via drag-drop or file picker; stream to local backend | M | Works for files ≥ 1 GB without loading whole file into browser memory; clear progress; corrupt file → friendly error (US1) |
| FR-02 | Waveform rendering from decoded audio (peaks computed backend-side) | M | Full-file overview at import; redraw < 200 ms on zoom change; visually faithful for speech (peaks min/max per pixel column) |
| FR-03 | Zoom & pan: mouse-wheel/scroll zoom toward cursor; drag-pan; keyboard shortcuts (e.g. +/- zoom, arrows pan) | M | Zoom range from full-file to ≥ 50× magnification; smooth at both extremes (US2) |
| FR-04 | Original-time ruler with hh:mm:ss labels | M | Labels accurate to ±10 ms; readable at all zooms (fewer labels when zoomed out) |
| FR-05 | Playhead; click-to-seek; Space = play/pause; double-click = play from click point | M | Seek latency < 300 ms after click; Space toggles globally except while typing in an input (US2) |
| FR-06 | Transport display: current result-time and total result-duration (edited timeline) visible at all times | M | Updates at least 10×/s during playback (≤ ~100 ms stale; may track display refresh); both clocks (original + result) shown (US2) |
| FR-07 | Auto-detect silences: absolute threshold in dB (default −40) + minimum duration (default 1.0 s) | M | Candidates = maximal spans below threshold lasting ≥ min duration; measured over the loudest channel per window (architecture fixes window size, target ≤ 10 ms granularity) (US3) |
| FR-08 | Candidate overlay on waveform with distinct styling; click to select | M | Up to 500+ candidates render and remain interactive; selection shows start/end/duration (US5) |
| FR-09 | Accept candidate ("Cut this") → entry in cut list with keep-margin applied at playback/export | M | Applied cut = silence span shortened to the keep-pause (default 0.4 s), centered on the original span (US5, US8) |
| FR-10 | Reject candidate ("Keep this") → not cut; visually marked as reviewed | M | Rejected candidates no longer look actionable; excluded from counts shown to user (US5) |
| FR-11 | Threshold + min-duration inputs and noise-floor-based threshold suggestion | M | Suggestion (= measured background level at import + ~12 dB) is computed during import; a "Measure noise floor" control displays it and lets the user apply it as the threshold (not auto-applied) (US4) |
| FR-12 | Manual re-run only, with confirmation dialog warning that review state resets | M | No auto re-run on setting change; dialog is explicit (US4) |
| FR-13 | Manual selection (drag) + "Cut selection" + "Discard selection" | M | Selection snaps to reasonable sample boundaries (no sub-ms ambiguity); adds to same cut list as FR-09; overlapping/adjacent cuts merge cleanly (US6) |
| FR-14 | Cut list model: all cuts (detected-accepted + manual) held non-destructively; toggle a cut back off is possible | M | Removing a cut restores audio everywhere (playback, result duration, export) (US6) |
| FR-15 | Playback follows edited timeline (skips cut ranges) | M | When playhead crosses a cut during playback it jumps to the other side; pause/seek math accounts for cuts (US2) |
| FR-16 | Undo/redo covering all state-changing actions | M | Ctrl/Cmd+Z, Ctrl+Shift+Z, visible buttons; no unrecoverable mistakes in session (US7) |
| FR-17 | Export WAV (16-bit or 24-bit PCM, source sample rate/channels) and MP3 (192 kbps CBR) | M | Only uncut audio; ~5 ms fade in/out at each join; progress UI; source file untouched (US8) |
| FR-18 | Export honors keep-margin semantics of every accepted cut | M | No accepted cut is removed *entirely* — at least the keep-pause remains (default 0.4 s) (US8, A5, D9) |
| FR-19 | Status bar: candidates found / cut / kept; total time to be removed ("−23:41") | M | Numbers update after every accept/reject/cut/undo; result duration consistent with export (US5) |
| FR-20 | Backend lifecycle: start command serves the API on localhost (configurable port); graceful shutdown; port-conflict handling; CORS restricted to the configured UI origin. Frontend runs separately (dev server or static hosting). | M | README documents both commands (backend + frontend); if port busy, clear message; UI on any origin reaches the API only when its origin is allowed (G6) |
| FR-21 | Plain-language error states for: unsupported/corrupt file, decode failure, export failure, backend unreachable | M | Each documented error has a recovery hint (US9) |
| FR-22 | Progress indicators for import, analysis, re-run, export (determinate where possible) | M | No indefinite spinner without explanation; UI remains responsive (cancel where feasible: S) |
| FR-23 | README + LICENSE (GPL-3.0) + release build instructions | M | A new user reaches a running app from a clean machine following README only (G6) |
| FR-24 | Candidate jump navigation (next/previous candidate buttons or keys) | S | Even though review is click-driven, minimal next/prev aids locating the next unreviewed candidate on long files |
| FR-25 | Dark-mode UI | C | Not committed; defer to post-v1 |
| FR-26 | Project save/load | C | Explicitly out (NG6) |

## 6. Non-functional requirements

| ID | Requirement |
|----|-------------|
| NFR-1 | **Performance targets (typical desktop, ~4-core CPU, 16 GB RAM):** import+waveform of 3 h 128 kbps MP3 ≤ 3 min; re-run detection ≤ 10 s; export WAV ≤ 2× audio duration, MP3 ≤ 3×; UI never frozen > 500 ms (work in backend/worker threads). _(Import budget relaxed from ≤ 60 s per user decision, 2026-10-18.)_ |
| NFR-2 | **Memory:** browser tab < 1 GB RAM for a 4 h stereo file (audio data stays backend-side); backend peak < 4 GB for same file. |
| NFR-3 | **Privacy:** in v1 the API runs on localhost and audio bytes go only to that backend (frontend is a separate static app; API base URL configurable, default `http://127.0.0.1:<port>`); no telemetry; no third-party CDN assets. A future cloud mode would require explicit user opt-in (see §7) and a privacy review. |
| NFR-4 | **Reliability:** corrupt inputs never crash the backend process; backend restarts cleanly; no silent data loss (non-destructive model guarantees source intact). |
| NFR-5 | **Accuracy:** detection granularity ≤ 10 ms; export length matches the UI-computed result duration ± 50 ms; no audible clicks at joins (spot-check + automated test). |
| NFR-6 | **Compatibility:** Chromium and Firefox (latest two versions) on Linux, macOS, Windows (desktop only). |
| NFR-7 | **Accessibility baseline:** all actions keyboard-reachable; visible focus; contrast ≥ 4.5:1 for text; ARIA labels on controls (full WCAG audit out of scope for v1). |
| NFR-8 | **Localization:** English only in v1. |
| NFR-9 | **Licensing/compliance:** GPL-3.0; dependency licenses compatible (verify in Step 3); MP3 encoding uses a patent-safe/GPL-compatible encoder path. |

## 7. MVP scope vs future

**MVP (v1) = FR-01…FR-23** with the flows in §4. Concretely:

- In: MP3/WAV → waveform → detect (dB + duration, absolute, noise-floor suggestion) → click-review (cut/keep) → manual cut → undo → export WAV/MP3 with keep-margin and fades.
- Out: every non-goal in §2, plus all S/C requirements.

**Post-v1 candidates (unordered, not commitments):** FR-24 refinement, batch accept/reject filters, Opus/FLAC/M4A input, project save/load, auto-suggest thresholds on import, zero-crossing snap, split-at-silence export, Safari/mobile, release binaries with auto-update, **cloud-deployed server mode (same API; requires storage/job abstractions + privacy review)**.

## 8. Success metrics

- **Primary (gate for v1):** a real 2–4 h community-discussion file (user-supplied) is tightened end-to-end (import → review → export) in **≤ 15 minutes** of user time; exported file passes a spot-check (no clipped words at 10 random joins; no audible clicks; duration matches expectation).
- **Quality:** on a human-labeled sample file, ≥ 90% of silences ≥ 3 s are detected; false positives on speech segments do not force the user to reject more than ~30% of candidates at default settings (calibration finding, not a hard SLA).
- **Engineering:** automated tests green; README-driven install works on a clean Linux and macOS machine; NFR-1/2/5 targets met on the reference machine.
- **Adoption (soft):** repository is publicly usable by a stranger given the documented prerequisites (toolchain) — no other hand-holding needed.

## 9. Constraints & assumptions

### Constraints

- C1. Local Rust backend + browser UI; audio stays local (decision from interview).
- C2. Desktop-only; Chromium + Firefox.
- C3. GPL-3.0 open source; no proprietary dependencies that forbid redistribution.
- C4. Solo/low-resource development; prefer boring, well-documented technology.
- C5. No persistence in v1 — closing the app discards the session.

### Assumptions (provisional; flagged for the implementer to validate)

- A1. MP3 export bitrate fixed at 192 kbps CBR in v1 (no bitrate UI). *(Low-risk default; change on request.)*
- A2. WAV export: source sample rate and channel count preserved; 16-bit PCM unless source is 24-bit.
- A3. Manual "Cut selection" adds to the same cut list as accepted detections (FR-14) — obvious from the non-destructive model.
- A4. Review UI includes counters/status even though queue navigation was deprioritized (FR-19, FR-24 as Should).
- A5. Keep-margin is centered: keep ≈ 0.2 s before + 0.2 s after detected silence by default (total 0.4 s), truncated at audio edges.
- A6. Detection operates on per-10 ms frames of the max-absolute sample per channel (peak-based, like Audacity "Peak level"); RMS mode is post-v1.
- A7. "Measure noise floor" = e.g. 10th-percentile frame RMS over the file, +12 dB offset (Candid Tools precedent); exact formula fixed in architecture.
- A8. Analysis/exports are cancellable where feasible; not strictly required in v1 (marked S in FR-22).
- A9. The user's machine has enough free disk space for the exported file; the app does not manage disk quotas.
- A10. No telemetry, analytics, or crash reporting ever (consistent with NFR-3) unless the user explicitly opts into filing an issue with logs attached.

## 10. Edge cases (must behave sanely)

- Silence at the very start or end of the file (keep-margin truncation; do not produce zero-length joins).
- All-silence file or extremely noisy file (no gap between noise floor and speech) — detection may return 0 or everything; UI must still be usable; noise-floor suggestion must not claim certainty.
- Candidate overlapping a manual cut, or two candidates overlapping after manual edits — merge/deduplicate rules in architecture; no double-counting in the "time removed" metric.
- Cuts covering > 50% of the file (weird but legal) — playback/export remain correct.
- Zero-length or 1-sample files, empty file, non-audio file with audio extension — reject with clear errors.
- Stereo file with one dead channel — detection must not crash (max-channel rule, A6).
- User seeks/clicks during playback and during analysis — playback state machine must stay consistent.
- Very short audio (< min duration) — waveform + controls still render.
- Browser tab reload mid-session — session lost by design (NG6); backend must not leak the uploaded temp file (cleanup on disconnect/startup).
- Port already in use / backend already running — friendly message (FR-20).

## 11. Decisions made about risks

| Risk (from background.md) | Decision |
|---|---|
| R1 Browser memory on 2–4 h files | **Mitigated by architecture:** decoding/analysis/export entirely in Rust backend; browser receives only peaks + streams playback preview — risk largely eliminated for the tab. Backend memory (NFR-2) is the remaining constraint. |
| R2 Long CPU times | Backend threads + progress UI (FR-22); NFR-1 budgets; cancel = S. |
| R3 Clicks at joins | Required ~5 ms fades at every join (FR-17); tested. |
| R4 Detection false positives | Review-first design; nothing auto-applied (NG3); noise-floor suggestion (FR-11); defaults from prior art; calibration against real user files during implementation. |
| R5 Codec/encode quality | MP3 export 192 kbps CBR only (A1); WAV lossless path always available; single re-encode. |
| R6 Many-candidate UI perf | Backend-computed peaks; overlay rendering budgeted for 500+ candidates (FR-08); virtualize only if measurements demand (architecture). |
| R7 No-undo mistakes | Full undo/redo required (FR-16). |
| R8 Dual timelines | Both clocks always visible (FR-06); original time is the ruler of record. |
| R9 Scope creep | Non-goals §2 are binding; any new feature requires a PRD update + approval. |
| R10 Browser gaps | Firefox + Chromium explicitly in scope; Safari/NG5 best-effort; avoid Chromium-only APIs without fallback. |
| R11 Accuracy on real recordings | Success metric §8 requires calibration on a real user file before calling v1 done. |
| R12 Platform risk | Platform chosen early (Rust backend + browser UI); Step 3 must keep decode/export in a swappable core with automated tests against fixture files. |

## 12. Decision log (user-confirmed)

| # | Decision | Choice | Why (short) |
|---|----------|--------|-------------|
| D1 | Deployment | **Browser UI + local backend** | Heavy decode/export kept out of the browser; local privacy preserved |
| D2 | Input formats | **MP3 + WAV only** | Matches user's actual files; keeps codec matrix small |
| D3 | Output formats | **WAV and MP3** | Lossless archive + small shareable file |
| D4 | Audience | **Open-source / public product** | README, a11y baseline, tests required |
| D5 | Edit model | **Non-destructive cut list** | Safe mistakes, cheap undo, drives playback/export |
| D6 | Playback | **Edited timeline** (skips cuts) | User hears the result while reviewing |
| D7 | Timestamps | **Both clocks; original primary** | Review against source recording; result duration visible |
| D8 | Part A extras | **Zoom & pan + undo/redo in v1**; paste/trim-clip deferred | Zoom is essential on 4 h files; undo prevents lost work |
| D9 | Keep-margin | **Keep a fixed pause** (default 0.4 s) | Proven by Audacity/Candid; avoids run-together speech |
| D10 | Threshold model | **Absolute dB + auto-suggest** from measured noise floor | User's idea ("set the loudness threshold") + safety net |
| D11 | Review flow | **Waveform clicking only** (queue nav is S, not committed) | User chose minimal UI |
| D12 | Re-run policy | **Manual re-run only**, confirmation warns review state resets | Protects review effort from accidents |
| D13 | Backend language | **Rust core** | Performance, single static binary potential, memory safety |
| D14 | File access | **Upload from picker/drag-drop** | No folder-permission setup; works cross-OS |
| D15 | Persistence | **None in v1** | Scope cut; sessions are short-lived by design |
| D16 | Success metric | **Real-file end-to-end ≤ 15 min** | Grounds v1 in the actual use case |
| D17 | Browsers | **Chromium + Firefox** | Covers the user's environment with modest QA cost |
| D18 | License | **GPL-3.0** | User's choice; ensures derivatives stay open |
| D19 | Detection defaults | **−40 dB, 1.0 s min, keep 0.4 s** | Candid/Notevibes-informed; adjustable per file |
| D20 | Deployment topology | **Fully separate:** frontend static app + backend API server; API boundary designed for possible future cloud deployment | User decision (2026-10-18): wants modular path to a cloud-hosted server later |
| D21 | Distribution | **Source build only for v1** (Rust + C toolchain for lame); release binaries post-v1 | Keeps CI/pre-release work out of v1 |
| D22 | Local startup | **Two documented commands only** (backend + frontend), no convenience launcher | Purest separation; README carries the setup burden |

## 13. Remaining open questions

None blocking. The following are deliberately deferred to **Step 3 (architecture)** where they are technical, not product, decisions:

- Rust crates for decode (symphonia vs ffmpeg CLI), MP3 encode, HTTP framework, waveform peak format/transport.
- Frontend stack (framework vs plain TS), state management, undo mechanism representation.
- Candidate merge/overlap semantics details; exact noise-floor formula; cancellation model; temp-file lifecycle.
- Test fixture strategy and CI.

## 14. Traceability hooks

- Product requirement IDs (FR-*, NFR-*, US*) are the contract for `architecture.md` (§Traceability maps FR → components) and `plan.md` (tasks reference FR IDs).
- Decision IDs (D1–D22) must be honored; changing one requires updating this PRD **and** dependent documents (Ground Rule 5). Architecture-level decisions D23–D34 live in `architecture.md` §13 with the same rule.

---
_End of Step 2. Please review `docs/prd.md` — especially §2 non-goals, §5 FR table, and §12 decision log — and approve, or request changes, before I move to Step 3 (system design → `architecture.md`)._
