# background.md — Understanding the Idea

_Step 1 of the Idea → Documentation workflow (see `../todo.md`). Status: reviewed & approved._

> **Supersession note (Step 5):** the Open questions below were answered during the Step 2 interview and beyond. Authoritative decisions now live in `prd.md` §12 (D1–D22, incl. deployment → separate local Rust backend + static frontend, D20–D22) and `architecture.md` §13 (D23–D34). Where this document's speculation (e.g. "web app with client-side processing") conflicts with those, the PRD/architecture win.

## Idea summary

A focused, "Audacity-lite" audio editor for very long spoken-word recordings (2–4 hour community discussions). It has two parts:

- **Part A — Manual editing surface:** import an audio file, render it as a scrollable/zoomable waveform with timestamps, let the user click anywhere (or double-click / press Space) to play/pause from that point, drag-select arbitrary ranges, cut them out, and export the edited audio.
- **Part B — Silence auto-detection:** an "Auto-detect silences" button analyzes the track for periods below a user-configurable loudness threshold (dB) and minimum duration (seconds), draws each candidate on the waveform as a reviewable overlay, and lets the user accept (cut) or reject each suggestion with one click.

The primary pain point is editorial, not creative: long pauses between question and answer make discussion recordings tedious to listen to, and removing them by hand in a general-purpose DAW is slow.

Deployment is explicitly open: **prefer the web (runs in the client's browser)**, falling back to a locally-run app if browser memory/CPU limits make that infeasible. This is one of the biggest open technical questions (see Open questions → Deployment).

## Problem & target users

**Problem.** Recordings of community discussions (panels, Q&A, meetups, interviews) contain many long silences — people thinking, waiting, passing a microphone. These stretches waste listener time and inflate file size. Existing tools either (a) require manual scrubbing and cutting, (b) auto-remove silence without review, which risks cutting quiet-but-real speech, or (c) are heavyweight general-purpose editors with a steep learning curve.

**Target users (as understood so far):**

- Primary: the person who records/produces community discussions and must publish or share a tightened version (a community organizer, moderator, archivist, or podcaster).
- Secondary: other people with long spoken-word audio — lecture capture, oral histories, meeting minutes, language recordings.
- Non-users (explicitly not the target): music producers, multitrack mixing engineers, sound designers.

**Usage shape.** One file at a time, opened in a single session, edited, exported. Sessions are infrequent but long (tens of minutes of review). There is no indication yet of collaboration, accounts, or a library of files.

## Open questions

### A. Deployment & performance (highest priority)

1. **Web or local app?** The idea raises this directly. Browser memory limits make full-decode approaches fail on 2–4 h files (see Risks R1). Decision: pure browser, local desktop app (Electron/Tauri), or hybrid (browser UI + local helper)?
2. **Target file sizes.** What are typical inputs: 40 MB MP3 (2 h @ 128 kbps) or 1 GB WAV? This determines whether streaming/chunked decoding is required.
3. **Accepted load/export time.** Is a 30–60 s "analyzing…" wait acceptable on import? On export?

### B. Input/output formats

4. Which input formats must be supported (MP3, M4A/AAC, WAV, FLAC, OGG, WEBM)?
5. Which output format(s) (MP3, WAV, FLAC, M4A)? Is a lossy re-encode of an already-lossy MP3 acceptable?
6. Sample rate/channel handling: keep source as-is, or normalize (e.g. always export 44.1 kHz stereo)?

### C. Editing model (Part A)

7. **Cut semantics:** does "cut" delete the range (audio closes up), or is it a non-destructive list of excluded regions applied at export? (The two differ in undo complexity and in what playback does before export.)
8. **Playback preview:** while editing, does play/pause traverse the *edited* timeline (cuts audible immediately) or the *original* timeline (cuts visible but not applied)?
9. **Undo/redo:** required? Depth? (Strongly recommended — see Risks R7.)
10. **Time display after cuts:** do timestamps on the waveform show original time, exported time, or both?
11. Scope of Part A beyond the idea: zoom, copy/paste, insert silence, fade handles, trim-to-selection, multiple files? (Guard against Audacity-scope creep.)

### D. Silence detection workflow (Part B)

12. **Defaults:** what default threshold (dB) and minimum silence duration (s) fit the user's recordings? Prior art: Audacity defaults −20 dB / 0.5 s (Truncate Silence), −30 dB / 1.5 s (Silence Finder); Notevibes ≈ −40 dBFS / 0.3 s; Candid Tools ≈ noise floor + 12 dB, 10 ms granularity.
13. **Keep-margin:** when a silence is cut, keep a short natural pause (e.g. 0.3–0.5 s) so speech doesn't run together? Prior art strongly suggests yes.
14. **Review ergonomics:** with possibly hundreds of candidates over 4 hours — accept/reject one by one, batch accept-all-above-N-seconds, keyboard next/prev navigation? How is a candidate auditioned (play around it) before deciding?
15. **Re-run behavior:** if the user changes thresholds, are suggestions regenerated and are previous accept/reject decisions preserved?
16. **Manual candidates:** can the user select a range and add it to the same cut list (one shared cut list for manual + detected)?

### E. Product & scope

17. **Personal tool or distributable product?** Affects polish, accessibility, browser support matrix, docs, licensing.
18. **Browser support:** Chromium-only OK, or Firefox/Safari too? (Firefox lacks browser-side AAC encoding; Safari differs on codec support.)
19. **Persistence:** is saving a project/session (edits + decisions surviving a tab close) needed in v1, or is export the only output?
20. **Privacy expectation:** files must never leave the machine? (Naturally satisfied by a pure-browser app; constrains any server-side option.)
21. **Success criteria:** what makes this "done"? E.g. "a 3-hour recording is tightened in under 15 minutes of review."

## Risks

| ID | Risk | Likelihood | Impact | Possible mitigation |
|----|------|-----------|--------|---------------------|
| R1 | **Browser memory exhaustion on 2–4 h files.** Decoded PCM is huge: ~176 KB/s per channel at 44.1 kHz float32 → a 4 h stereo file ≈ 5 GB in RAM; `decodeAudioData` also historically fails to release memory (Chromium bug 40400583). | High | High — app crashes or fails to load | Never hold full PCM: decode in chunks (WebCodecs `AudioDecoder`), or render peaks incrementally and keep the encoded file for playback/export; downsampled peaks (~1 kB/s); or move to a local app with more headroom; measure on real 4 h files early (spike task). |
| R2 | **Long decode/analyze/export times.** Full-file analysis + re-encode of 3 h of audio is CPU-bound; browser export (e.g. MP3 via JS encoder) can take minutes. | High | Medium — perceived as broken if silent | Run in a Web Worker with progress UI; stream results; consider WAV/FLAC output (fast, no encoder loop) or native encoder via WebCodecs (AAC/Opus/FLAC). |
| R3 | **Audible clicks at cut points.** Hard splices mid-waveform produce clicks. | Medium | Medium — output sounds amateur | Apply short fades (≈4 ms, as Candid Tools does) at every join; snap optional to zero-crossings. |
| R4 | **Silence detection false positives/negatives.** Threshold-based detection cannot distinguish room noise from quiet speech; noisy rooms have no clean gap between noise floor and speech (Audacity manual warns about exactly this). | High | High — user loses trust, cuts real words | Review-first UI (candidate overlay, accept/reject), keep-margin, tunable threshold with live re-render of candidates, presets, audition before accept. Never auto-apply. |
| R5 | **Generation loss / codec issues on export.** Re-encoding MP3→MP3 loses quality; VBR MP3 sources can desync waveform vs. playback timestamps (documented wavesurfer.js issue). | Medium | Medium | Prefer WAV/FLAC output; if MP3, single encode at decent bitrate; document VBR caveat; align waveform and playback on the same decoded timeline. |
| R6 | **Waveform/UI performance with many candidates.** Hundreds–thousands of suggestion overlays as DOM regions cause jank (wavesurfer.js docs warn against thousands of region elements). | Medium | Medium | Single canvas overlay for candidates; virtualize the candidate review list. |
| R7 | **No undo → destructive mistakes.** Losing 30 minutes of review decisions to one misclick. | Medium | High | Non-destructive cut list + full undo/redo of accept/reject/cut actions; "original file untouched until export." |
| R8 | **Playhead/time mapping complexity after cuts.** Dual timelines (original vs. edited) confuse users if unexplained. | Medium | Medium | Show both clocks explicitly ("source time / result time"), or keep a single source timeline until export (decision needed). |
| R9 | **Scope creep toward Audacity.** Effects, EQ, multitrack, recording... | Medium | High — project never finishes | Hard non-goal list in the PRD; every feature request tested against "does it help cut silences?" |
| R10 | **Browser compatibility gaps** (AAC encode missing in Firefox; Safari codec quirks; mobile memory limits). | Medium | Low–Medium | Explicit browser matrix; degrade gracefully (offer WAV where MP3 encode unavailable); desktop-first. |
| R11 | **Detection accuracy on the user's actual recordings** (reverb, background hum, overlapping speakers during "silence"). | Medium | High | Early calibration against 1–2 real user files; threshold presets; possibly relative (noise-floor-tracking) threshold instead of absolute dB. |
| R12 | **Cost/effort if the chosen platform fails.** Backtracking from "web" to "local app" late rewrites the audio I/O layer. | Medium | High | Resolve deployment question **first** (Step 2); build a thin platform spike (load → detect → export on a real 4 h file) before committing to full build. |

## Similar solutions & case studies

### Direct prior art (same problem)

- **Audacity — Silence Finder / Truncate Silence / Label Sounds** ([Silence Finder params](https://manual.audacityteam.org/man/silence_finder_setting_parameters.html), [Truncate Silence](https://manual.audacityteam.org/man/truncate_silence.html), [Label Sounds](https://manual.audacityteam.org/man/label_sounds.html)) — the reference implementation of threshold-based silence detection. Key learnings: detection is a *two-knob* problem (threshold dB + minimum duration); threshold must sit above noise floor and below quiet speech, which is not always possible; a single absolute threshold produces false positives from clicks/pops; Label Sounds offers peak/average/RMS measurement modes — RMS is more robust than peak for noisy material. Truncate Silence offers "truncate to fixed length" vs "compress by percentage" — i.e. *keep a natural pause* is a first-class option.
- **Auphonic Editor** ([docs](https://auphonic.com/help/web/auphoniceditor.html)) — commercial SaaS that auto-marks cuts (silence, filler words, coughs, music, user cuts) directly on the waveform, color-coded, with per-cut **deactivate** and a filter by category. Validates the review-first interaction model in Part B: detection proposes, waveform displays, user toggles each. Also shows the cost of the alternative (server-side re-processing required to apply changes).
- **Candid Tools — Remove Silence** ([site](https://candidtools.com/remove-silence)) — closest to our exact UX, entirely in-browser: threshold + minimum-length + **"pause to keep"**, shaded candidate preview on the waveform, live count of silences/time saved before applying, 10 ms detection granularity, 4 ms fades at joins, noise-floor-relative default (background + 12 dB). Learnings: (a) preview-before-apply is essential, (b) keep-margin should be a control, (c) honest limits section ("detection works on level only") builds trust.
- **Notevibes — Silence Remover** ([site](https://notevibes.com/silence-remover)) — browser-based, three modes (remove all / auto-trim edges / split at silence), ≈ −40 dBFS for ≥ 0.3 s, up to 200 MB files. Shows a viable simple product, but no per-candidate review — exactly the gap our Part B fills.
- **AudioGrabber** ([GitHub](https://github.com/mantukin/AudioGrabber)) — browser extension: waveform editor + silence detection using wavesurfer.js + Web Audio API; shows the stack pattern but limited to capture/trim use case.

### Adjacent tools (workflow, not feature, competition)

- **audio-editor.app / Puter** ([site](https://audio-editor.app/?editor)) — browser editor with waveform + transcript editing, "shorten pauses", autosave to `.aeproject`, offline/PWA. Validates browser feasibility for moderate files and the value of session persistence; heavier and account/cloud-oriented (Puter drive).
- **AudioMass** ([about](https://audiomass.co/about.html)) — open-source, dependency-free browser editor (<100 KB), region cut/copy/paste, undo stack, offline use. Proof that core Part A is very achievable in plain JS; no auto silence detection.
- **Audio Cutter Pro** ([GitHub](https://github.com/ArPaN-DS/Audio_Cutter_Pro)) — Flask + wavesurfer.js + pydub/ffmpeg, server-side processing incl. silence detection. Represents the "local server" middle path: keeps heavy decode/encode out of the browser at the cost of an install.
- **Plaud Web — Audio Trimming** ([docs](https://support.plaud.ai/hc/en-us/articles/51039129977497-Audio-Trimming)) — commercial hardware companion; explicitly **does not** offer browser-side auto silence removal (mobile app only), a hint that competitors treat browser-side detection on long files as hard.
- **wavesurfer.js** ([large-files docs](https://wavesurfer.xyz/docs/performance), [loading docs](https://wavesurfer.xyz/docs/loading-audio/)) — the de-facto waveform library; critical guidance: it "doesn't aim to cut, add effects, or process your audio"; large files need MediaElement playback + pre-computed peaks, canvas cap 8000 px, keep region-element counts low. Directly shapes our architecture (Part A ≠ wavesurfer alone).
- **bbc/audiowaveform** — CLI reference for offline peaks generation (useful if a server/CLI step is ever introduced).

### Case studies / technical evidence

- **Chromium issue 40400583** — [`decodeAudioData` memory not released, ~2 GB after 10 files](https://issues.chromium.org/issues/40400583/resources): confirms full-decode pipelines are memory-hostile.
- **W3C public-audio mailing list (2013)** — [selective/partial decoding request](https://lists.w3.org/Archives/Public/public-audio/2013JulSep/0069.html): developers report `decodeAudioData` "runs out of memory and dies" on hour-long recordings; the API still only decodes whole files. Still the state of the art in browsers → chunked decoding must be done via WebCodecs or a local app.
- **Audacity manual (Silence Finder parameter lessons)** — false positives from ticks/pops, misses from loud clicks: absolute thresholding alone is imperfect → hence a human review step in Part B.

### Differentiators (our wedge)

1. **Long-form first:** designed for 2–4 h files, not 5-minute clips (most browser tools cap at ~100–200 MB).
2. **Review-oriented detection:** every suggestion is inspectable and individually acceptable/rejectable on the waveform (Auphonic does this but is paid SaaS and server-processed; Candid/Notevibes auto-apply; Audacity has no interactive review).
3. **Minimal surface:** only what's needed to tighten a discussion — no effects, no multitrack, no recording.
4. **Local/private by construction** if pure-browser.

## Assumptions

_Recorded per Ground Rule 4 — these are provisional and will be confirmed or corrected in Step 2._

1. Single-user, single-file editing; no accounts, collaboration, or file library.
2. The original file is never modified; all edits are applied to a copy at export time.
3. Cuts are full removal of the selected range (audio closes up), not muting to silence.
4. Desktop browser is the primary target; mobile is best-effort or out of scope.
5. Audio is mono or stereo only; no multichannel.
6. Detection is level-based (dB threshold + minimum duration) — no ML/VAD model in v1.
7. The tool is for personal/internal use first (polish and distribution questions deferred).
8. English-language UI is sufficient.
9. No monetization, analytics, or telemetry in v1.
10. Export to a single file (not chaptered/split output) is sufficient for v1.

---
_End of Step 1. Please review `background.md` — in particular the **Open questions** (A–E) and **Assumptions** — and approve, or tell me what to change, before I move to Step 2 (interview → `prd.md`)._
