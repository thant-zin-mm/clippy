# plan.md — Implementation Plan

_Step 4 of the Idea → Documentation workflow (see `../todo.md`). Status: awaiting review._
_Build strategy: **vertical slice first**, with an **early measured spike gate** on a real long file; **coarse tasks** (~one focused session each); **CI + Playwright committed for v1**._

**How to use this plan:** work phases in order; within a phase, tasks in order. A task is only `done` when its acceptance criteria are verified and testing notes executed; then update its `status` **and** the progress table in `README.md` (Ground Rule: README stays in sync). If reality forces a design change, stop, propose the document update to the user (per `coding-guide.md`), don't silently deviate.

**Status legend:** `not started` / `in progress` / `under review` / `blocked` / `done`
_(The coder workflow uses `under review` for code awaiting the user's review pass; it is an interim value between `in progress` and `done`.)_

---

## Phase 0 — Setup & CI

**Goal:** a repo both halves of the system exist in, with green automation from day one, so every later task lands on running code.
**Depends on:** none.
**Exit criteria:** two documented commands start backend + frontend; opening the UI shows a live "backend connected" badge; CI workflow runs fmt/clippy/tsc/eslint/tests green on push.

### P0-T1 — Repository scaffold, health check, CI
**Status:** `under review`

**Do:**
- Create structure per `architecture.md` §11: `backend/` (Rust crate: axum, tokio, serde, tower-http CORS; `GET /api/health` returning `{ok, version}`; env config `SILENCE_CUTTER_ADDR` default `127.0.0.1:7817`, `SILENCE_CUTTER_ORIGINS`; graceful shutdown; startup temp sweep stub) and `frontend/` (Vite + TS, ESLint + Prettier, base HTML/CSS, api client with `VITE_API_BASE` default `http://127.0.0.1:7817`, health-check badge).
- Add `LICENSE` (GPL-3.0), root `.gitignore`, `scripts/` fixture generator stub (synthesis of short WAVs with known silences; MP3 via lame once available — stub OK here).
- GitHub Actions workflow: Rust job (fmt `--check`, clippy `-D warnings`, `cargo test`) and Node job (`tsc --noEmit`, eslint, `vitest run` — configure `passWithNoTests` or add a placeholder test so Phase 0 CI is green before suites land) on push/PR, with caches.
- Record dependency license audit checklist in this task's PR notes (arch §7: MPL-2.0 symphonia obligations, LGPL lame linking note).

**Refs:** FR-20, FR-23 (partial), NFR-6/7 (baseline), NFR-9; arch §6 (`/api/health`), §9 (dev env), §11 (structure).
**Acceptance criteria:**
- `cargo run` in `backend/` binds `127.0.0.1:7817`; `curl /api/health` → `{ok:true,...}`.
- `npm run dev` in `frontend/` serves UI showing backend-connected state; wrong `VITE_API_BASE` shows unreachable message (FR-21 hint path).
- Port conflict prints a clear error (FR-20); CORS blocks an unknown origin but allows Vite's (D34).
- CI green on a clean branch.
**Testing/verification:** curl health; manual origin test (curl with `Origin: http://evil.test` → no CORS headers); both CI jobs pass.

---

## Phase 1 — Vertical slice (import → waveform → detect → edit → play)

**Goal:** the full review workflow works end-to-end on real audio, proven early by a measured spike; every remaining phase then hardens rather than discovers.
**Depends on:** Phase 0.
**Exit criteria:** P1-T1…P1-T6 all `done`, **including the P1-T3 measured gate**; a real long file can be imported, auto-detected, reviewed by clicking, manually cut, and heard with cuts applied (export comes in Phase 2).

### P1-T1 — Backend: upload + prepare pipeline
**Status:** `not started`

**Do:**
- `POST /api/sources`: streamed multipart → temp file (`silence-cutter-*`), size guard (default 4 GB), returns `{source_id}`; `DELETE /api/sources/{id}` removes file + caches.
- `POST /api/sources/{id}/prepare` → job; job manager (serial per source, panic isolation, `DELETE` cancel best-effort) emitting SSE `progress`/`done`/`error` on `GET /api/jobs/{id}/events`, plus `GET /api/jobs/{id}` status recovery endpoint (SSE-drop reconciliation, arch §6).
- Source lifecycle: `DELETE` removes file + caches; **idle TTL sweep (60 min)** for sources whose tabs vanished without `DELETE`; startup sweep; shutdown cleanup (temp-file leak edge case, PRD §10).
- Prepare pass (streaming, never full PCM): symphonia decode (MP3/WAV) → peak pyramid (i16, L0=512 samples, ×4 levels) + per-10 ms `peak_db`/`rms_db` frame arrays + meta (duration, sample rate, channels, format) + noise-floor 10th-percentile + suggested threshold (+12 dB, clamped [−80,−10]).
- `GET /api/sources/{id}/peaks?start_sample&end_sample&columns≤4096` → binary i16 min/max pairs (pyramid slice + downsample).
- Typed errors with hints (corrupt/unsupported → `{code,message,hint}`).

**Refs:** FR-01 (server half), FR-02, FR-07 (frame data half), FR-11 (suggestion), FR-20/21/22; NFR-1/2/4; arch §3.1, §4.3–4.4, §5.1, §6.
**Acceptance criteria:**
- Fixture MP3 + WAV upload→prepare→done payload matches ground truth (duration ±50 ms, sample rate/channels exact); corrupt file → friendly error, server alive.
- Peaks endpoint returns exactly `2×columns` i16 values; matches brute-force min/max on random buffers (unit test).
- SSE emits monotonic progress then a typed `done`; temp file exists during session, gone after `DELETE`.
- Startup sweep removes leftover `silence-cutter-*` files from a simulated crash.
**Testing/verification:** unit tests (peaks vs brute force, frames math, noise-floor percentile on synthetic distributions); integration test upload→prepare→peaks→delete on checked-in tiny fixtures; manual curl SSE.

### P1-T2 — Frontend: import flow + waveform + ruler + zoom/pan
**Status:** `not started`

**Do:**
- Import UI: drag-drop + file picker → XHR upload with byte progress → prepare job via SSE with determinate progress → meta stored in store; `pagehide` fires `sendBeacon(DELETE /api/sources/{id})` as best-effort cleanup (arch §6).
- Waveform renderer (DPR-aware canvas): fetch peaks for visible window with `AbortController` cache keyed `(start,end,columns)`; draw waveform; adaptive original-time ruler (1 s…1 h nice steps); wheel/keyboard zoom toward cursor, drag-pan; playhead line placeholder.
- Loading/progress/error states for the whole import (FR-22, FR-21).

**Refs:** FR-01–FR-04, FR-22; NFR-1 (≤3 min import w/ progress); arch §3.2 (`waveform/`, `api/`), §5.1.
**Acceptance criteria:**
- Real multi-hour MP3 imports showing progress throughout; waveform + correct hh:mm:ss ruler; zoom from full view to ≥50× smooth; **zoom redraw < 200 ms (FR-02)** and no UI freeze >500 ms while panning/zooming.
- Peaks requests abort/cancel correctly on rapid zoom (no stale draw, no console errors).
- Unsupported/corrupt file → toast with hint, UI usable afterward.
**Testing/verification:** vitest for peaks cache keys + ruler tick selection; manual scripted checks on long file; Chromium + Firefox smoke.

### P1-T3 — 🔒 Spike gate: measured real-file validation
**Status:** `not started`

**Do:**
- Using the user's real 2–4 h recording (or equivalent generated file), measure and record in `docs/plan.md` §Appendix:
  1. import+prepare wall time (gate: **≤ 3 min**, NFR-1);
  2. peak backend RSS and browser task-manager memory during/after import (gate: **backend < 4 GB, tab < 1 GB**, NFR-2);
  3. detect on defaults returns a sane candidate count (manual eyeball of 10 candidates vs waveform);
  4. WAV/MP3 export wall time (gate: ≤2×/≤3× duration, NFR-1) — *if Phase 2 not yet built, run a CLI-only export path stub or defer this measurement to P4-T2 and note it*;
  5. memory after 5 minutes of interactive zoom/review (no leak trend).
- Fix anything that misses a gate **within the spike's scope**; if a fix requires re-scoping the architecture — stop and escalate to the user (Ground Rule 4; mitigates background R12).

**Refs:** NFR-1/2, success metric §8; arch §8; background R1/R12.
**Acceptance criteria:** all measured numbers recorded in the Appendix with ✅/❌ against gates; no ❌ left unescalated; follow-up fixes (if any) tracked as sub-notes on this task.
**Testing/verification:** measurements themselves + `/proc`/`htop`/browser task manager notes; include machine specs line.

### P1-T4 — Detection end-to-end (backend + panel + overlay)
**Status:** `not started`

**Do:**
- Backend: `analysis.rs` pure functions — run-length threshold scan per §4.3 over cached frames → candidates; `POST /api/sources/{id}/detect` (sync).
- Frontend: detection panel (threshold dB, min duration ms, prefilled `suggested_threshold_db` + "Use suggestion", **Run detection** button); FR-12 confirmation dialog warning review state resets; candidates as distinct overlay regions on waveform; click to select **and auto-pan/zoom to fit the candidate with padding (US5)**; inspector showing start/end/duration.
- Persist candidates/cut/kept status in store (no server persistence — D27).

**Refs:** FR-07–FR-12; arch §3.1 (`analysis`), §3.2 (`ui/detectPanel`, `waveform`), §4.3, §5.2.
**Acceptance criteria:**
- Synthetic fixture with known silences → candidates match exact spans (±10 ms) in unit tests (all-silent, no-silence, one-sample, dead-channel cases per arch §10).
- UI: suggestion prefill works; re-run only on button press with the warning dialog; selecting a candidate zooms to fit it with padding (US5); ≥500 candidates render interactively (perf check on dense synthetic detection); defaults (−40 dB/1.0 s) produce usable results on the real file.
- Inspector audition: selecting a candidate and using the audition action sets the playhead ~2 s before the candidate start and plays (US5 "Space auditions from just before the candidate").
**Testing/verification:** cargo unit tests for detection; vitest for panel state transitions (dialog gating); manual dense-candidate zoom test; manual audition check.

### P1-T5 — Edit model: cuts, manual selection, undo, status bar
**Status:** `not started`

**Do:**
- `state/cutModel`: merged removed spans, keep-margin bake-in on accept (centered, A5/D31), overlap/adjacency merge, `resultDuration`, both-clock mapping (§4.5); `state/undo`: 100-snapshot stack over `{candidates, cuts}`.
- Inspector **Cut this / Keep this** actions wired to store; accepted → cut entry (correct baked span), rejected → `kept` styling (FR-10).
- Manual selection: drag-select on canvas (click = seek, drag = selection), **Cut selection** / **Discard selection** buttons; Delete key cuts selection (FR-13).
- Status bar: candidates found/cut/kept + live "−hh:mm:ss to remove" + result duration (FR-19).
- Undo/redo buttons + Ctrl/Cmd+Z / Ctrl+Shift+Z covering every mutation (FR-16).

**Refs:** FR-09, FR-10, FR-13, FR-14, FR-16, FR-19; arch §3.2 (`cutModel`, `undo`), §4.1, §4.5, §5.2.
**Acceptance criteria:**
- vitest coverage: merge semantics (overlap, ≤1 ms adjacency, ordering), keep-margin clamping at file edges, mapping round-trips, undo/redo over a scripted mutation sequence incl. accept-after-redo branching.
- **Re-run undo:** starting a detection re-run pushes a snapshot first; undo restores the pre-re-run candidates and their cut/kept statuses (US7 explicitly lists re-run as undoable).
- UI: accepting a candidate updates cut overlay + counters + result duration instantly; manual cut merges with an overlapping detected cut without double-counting; 20 random mutations then 20 undos restores exact prior state.
**Testing/verification:** vitest suites; manual scripted pass with keyboard only.

### P1-T6 — Playback: transport + both clocks + cut-skip
**Status:** `not started`

**Do:**
- `audio/playback`: object URL from the imported `File`; click-to-seek, double-click play, Space toggle (ignored in inputs), playhead animation; rAF loop performing cut-skip per §4.5 (binary search over merged spans; seek to span end; guard re-entrancy while seeking).
- Keep source-time anchor across edits (pause/play positions stable when a cut is toggled; document exact rule in code comments); status bar shows `source / result` clocks live (FR-06).
- Seek latency: seeking repositions the `<audio>` directly (gate: <300 ms perceptual).

**Refs:** FR-05, FR-06, FR-15; D6/D7/D24; arch §3.2 (`audio/playback`), §4.5, §5.3.
**Acceptance criteria:**
- Playhead starting from arbitrary clicks (start/middle/end) works; during playback, cut regions are skipped with ≤~50 ms audible overrun (rAF granularity) and play never stalls.
- Toggling a cut while paused preserves the source position; result clock updates accordingly.
- Space works globally except inside text inputs; double-click plays from click point (US2).
- vitest: cut-skip decision function over synthetic times/spans (entry mid-span, adjacent spans, at-file-end).
**Testing/verification:** manual listen-through over a cut-dense region of the real file; vitest for pure mapping/skip functions; Chromium + Firefox.

**Phase 1 exit criteria:** all six tasks `done`; P1-T3 gates ✅ (or escalated & resolved); a demo pass on the real file: import → detect → review-by-clicking → manual cut → play edited timeline.

---

## Phase 2 — Export

**Goal:** deliver the artifact — the tightened WAV/MP3 file.
**Depends on:** Phase 1 (cut model + real files flowing).
**Exit criteria:** P2-T1 `done`; exported file plays correctly in external players.

### P2-T1 — Export job: WAV/MP3 assembly + download UI
**Status:** `not started`

**Do:**
- Backend: `POST /api/sources/{id}/exports {format, filename_hint, cuts[]}` → job (validate: bounds, ordering, non-empty after merge; defensive re-merge per §4.5); streaming export: re-decode source → copy kept segments → **5 ms linear fades** at every kept-segment edge bordering a removed span → WAV (hound, source rate/channels, 16-bit or 24-bit per A2) or MP3 (lame-sys, 192 kbps CBR per A1/D33, resample to 48 kHz only if source rate unsupported); write to temp output; SSE `done {export_id, filename}`; `GET /api/exports/{id}/download` streamed with safe `Content-Disposition`; cancel via job delete (best-effort).
- Frontend: export dialog (format choice, live result duration from `cutModel`, expected time-removed), progress via SSE, auto/manual download on done, error toasts (disk full, etc.).
- Ensure original file untouched semantics hold (server only has its uploaded copy) and session restart loses nothing already exported.

**Refs:** FR-17, FR-18, FR-22; NFR-1 (export budgets), NFR-5 (±50 ms length, no clicks); A1/A2/A5; arch §3.1 (`audio::export`), §5.4, §6, D33.
**Acceptance criteria:**
- Round-trip test: fixture with known cuts → export → decode output → assert (a) duration = `resultDuration` ±50 ms, (b) audio inside removed spans absent (compare energy), (c) 4–5 ms fade ramps present at joins (unit-test the fade math + integration energy check), (d) MP3 is 192 kbps CBR.
- Manual listen spot-check on real file: 10 random joins, no clicks, no clipped words at cut edges (success metric §8).
- Progress events reach 100%; disk-full/cancel paths produce friendly errors; export of a >50%-cut file still valid.
**Testing/verification:** cargo integration export tests on fixtures (WAV asserted sample-exactly vs expected concat; MP3 decoded back via symphonia for duration/energy); vitest for dialog state; manual real-file listen.

**Phase 2 exit criteria:** demo on real file: full loop import → review → export → play result in an external player.

---

## Phase 3 — Hardening & polish

**Goal:** make it safe, complete, and presentable per the guides.
**Depends on:** Phase 2.
**Exit criteria:** P3-T1, P3-T2 `done`; no known crash/edge-case bugs from the §10 list; UI passes the `ui-guide.md` checklist.

### P3-T1 — Edge cases, errors, lifecycle hardening
**Status:** `not started`

**Do:**
- Work the `prd.md` §10 edge-case list end-to-end (silence at start/end, all-silent/noisy files, overlapping accepted+manual cuts, >50%-cut, tiny/empty/non-audio files, dead channel, seek-during-analysis, <min-duration audio, tab reload mid-session, port in use, replaced source cleanup).
- Lock in FR-21 error catalogue (every failure mode → `{code, message, hint}` + UI toast/banner); backend unreachable mid-session banner; upload failure retry path; temp lifecycle audit (delete on replace, sweep at startup, shutdown cleanup).
- Cancellation (`DELETE /api/jobs/{id}`) if not already cheap (S).

**Refs:** FR-20/21, NFR-4, `prd.md` §10 (all rows); arch §7, §3.1 (`state`, `jobs`).
**Acceptance criteria:** every §10 row has a passing manual check recorded in the Appendix; error catalogue table in this task's notes maps each FR-21 case → code/hint + test.
**Testing/verification:** integration tests for each error path reachable without UI; manual checklist for the rest.

### P3-T2 — Accessibility, shortcuts, UI-guide conformance
**Status:** `not started`

**Do:**
- Full keyboard reachability of all controls (tab order, visible focus, Space/arrows/Ctrl+Z per §3.2 `shortcuts`), ARIA labels/roles on toolbar, panels, canvas alternative (e.g. `role="img"` + text summary, live region for status changes), contrast ≥ 4.5:1 (NFR-7).
- Visual pass against `ui-guide.md` (colors, type, spacing, component states, empty/loading/error/empty-candidate states), copy pass against its tone rules; dark-mode only if ui-guide/PRD changed scope (FR-25 is C — out).
- Responsive-enough desktop widths (≥1280 px primary, degrade to 1024 px).

**Refs:** NFR-7, FR-24 (next/prev candidate buttons if cheap — S), FR-25 (explicitly out); `ui-guide.md`; arch §3.2.
**Acceptance criteria:** keyboard-only walkthrough of the entire happy path (import→detect→review→export) without mouse; axe/Lighthouse-style spot check on main states shows no critical issues; ui-guide checklist signed off in Appendix.
**Testing/verification:** manual keyboard pass; browser contrast tooling; screenshot review vs ui-guide.

**Phase 3 exit criteria:** hardening checklist + ui-guide checklist green; Chromium **and** Firefox walkthroughs pass.

---

## Phase 4 — Release readiness

**Goal:** prove v1 end-to-end and publish the front door.
**Depends on:** Phase 3.
**Exit criteria:** P4-T1, P4-T2 `done`; success metric §8 recorded and passing; README live.

### P4-T1 — Playwright e2e + CI completion
**Status:** `not started`

**Do:**
- Playwright (Chromium) smoke: launch backend + preview build → import fixture WAV → run detection (custom thresholds that guarantee candidates) → cut one candidate + manual selection cut → export WAV → assert download size/duration via decode; run headless in CI.
- Wire e2e job into GitHub Actions (separate job, artifact screenshots on failure); ensure full CI (fmt/clippy/tsc/eslint/vitest/cargo/integration/e2e) is green on main.
- Cover the FR-21 "backend unreachable" banner in e2e (start UI with bad `VITE_API_BASE`).

**Refs:** testing strategy arch §10; FR-23; D (CI + Playwright in v1).
**Acceptance criteria:** e2e passes locally and in CI ≥2 consecutive runs (flakes triaged, not ignored); CI badge in README.
**Testing/verification:** the workflow itself; re-run stability check.

### P4-T2 — Real-file acceptance, README, docs sync
**Status:** `not started`

**Do:**
- Execute the **success-metric protocol** (PRD §8) on the user's real 2–4 h file: end-to-end timing (gate ≤ 15 min), 10-join spot-check, duration verification; fill the `plan.md` Appendix (incl. any deferred P1-T3 items like export timing).
- **Detection calibration check (PRD §8 quality metric):** with the user, label ≥ 20 silences in a real file (all ≥ 3 s); run detection at defaults + tuned thresholds; record recall on silences ≥ 3 s (target ≥ 90%) and a rough false-positive burden; note tuning advice in README. Treat as calibration finding per §8 (not a release blocker unless recall is grossly off — escalate in that case).
- Write `README.md` (Step 7 will formalize structure; this task drafts content): project summary, **two-command setup** (D22) for dev + build instructions, supported formats/limits, dependency prereqs (Rust + C toolchain, D21), GPL-3.0 license, screenshots/GIF, link to `docs/`.
- Docs sync pass: reconcile `prd.md`/`architecture.md`/`plan.md` with any deviations discovered during implementation (Ground Rule 5); confirm every FR has a passing evidence reference (test or recorded manual check).

**Refs:** G6, FR-23, success metrics §8; Ground Rules 5–6.
**Acceptance criteria:** Appendix filled with ✅ against success metric; README instructions followed verbatim on a clean environment (or clean container) result in a running app; FR coverage table complete (each FR-01…FR-23 → test/check reference); all documents mutually consistent.
**Testing/verification:** clean-machine (fresh clone) walkthrough; final full-CI run.

**Phase 4 exit criteria:** all tasks `done` → v1.0 tag ready; completion summary per `todo.md` §Completion.

---

## Appendix — measurements & checklists (living section)

> Filled in as tasks complete; referenced by P1-T3, P3-T1, P3-T2, P4-T2.

**Machine specs:** _TBD (P1-T3)_

| Measurement | Gate | Result | Task |
|---|---|---|---|
| Import+prepare, real file (min:sec) | ≤ 3:00 | _TBD_ | P1-T3 |
| Backend peak RSS | < 4 GB | _TBD_ | P1-T3 |
| Browser tab memory | < 1 GB | _TBD_ | P1-T3 |
| Detect on real file | sane candidates, eyeball pass | _TBD_ | P1-T3 |
| Export WAV / MP3 (× duration) | ≤ 2× / ≤ 3× | _TBD_ | P1-T3 or P4-T2 |
| Memory after 5 min review | no leak trend | _TBD_ | P1-T3 |
| Real-file E2E review time | ≤ 15 min | _TBD_ | P4-T2 |
| Recall on labeled silences ≥ 3 s | ≥ 90% (calibration) | _TBD_ | P4-T2 |
| 10-join spot-check | no clicks/clipping | _TBD_ | P4-T2 |
| Export length accuracy | ± 50 ms | _TBD_ | P2-T1 |

| Checklist | Status | Task |
|---|---|---|
| PRD §10 edge cases (14 rows) | _TBD_ | P3-T1 |
| FR-21 error catalogue | _TBD_ | P3-T1 |
| Keyboard-only happy path | _TBD_ | P3-T2 |
| ui-guide conformance | _TBD_ | P3-T2 |
| FR coverage table (FR-01…23 → evidence) | _TBD_ | P4-T2 |

---
_End of Step 4. Please review `docs/plan.md` — phase ordering, the 11 tasks, their gates — and approve, or request changes, before Step 5 (adversarial review of all documents)._
