# coding-guide.md — Rules for the Implementer

_Step 6 of the Idea → Documentation workflow (see `../todo.md`). Status: awaiting review._
_This guide binds every human or AI agent implementing **Silence Cutter** v1._

## 1. Authority: read this first, deviate never

1. **These documents are the contract, in binding order:**
   1. `docs/prd.md` (product: requirements FR-*/NFR-*, decisions D1–D22, non-goals)
   2. `docs/architecture.md` (technical design: D23–D34, API contract, algorithms §4.3/§4.5, folder structure §11)
   3. `docs/plan.md` (what to build, in what order, with acceptance criteria)
   4. `docs/ui-guide.md` (how it must look and feel)
   5. This guide (how to write and ship the code)
2. **Strictly follow the documents.** Do not "improve" scope, swap libraries, rename APIs, add features, or skip acceptance criteria on your own initiative.
3. **When anything is ambiguous, missing, or conflicts — stop and ask the user** (selectable options, recommended first). Guessing is a defect. Never resolve a document conflict silently.
4. **When reality forces a change** (library bug, platform limit, wrong estimate): (a) stop, (b) propose the exact document edits (which file, which section, what text), (c) get approval, (d) apply the edits, (e) then code. Ground Rule 5: all documents must stay mutually consistent — if you change one, update every other affected one and say so.
5. **Non-goals are binding** (`prd.md` §2). "Just a little fade-in feature" is a scope violation. Ask; expect it to go into the post-v1 list.

## 2. Working the plan

- Work **task-by-task, in plan order** (P0-T1 → P1-T1 → … → P4-T2). Never start a task whose dependencies are not `done`.
- One short-lived branch per task: `task/P1-T2-waveform-import` (pattern: `task/<ID>-<slug>`).
- Before coding a task: re-read its **Refs / Acceptance criteria / Testing notes** in `plan.md` and the traced sections in `prd.md`/`architecture.md`.
- After coding: verify **every** acceptance criterion explicitly (a test, a command with recorded output, or a named manual check written into the PR description).
- Update `plan.md` task `status` (`in progress` → `done`) and the `README.md` progress tracker in the same PR that finishes the task.
- Fill `plan.md` Appendix measurements when the task asks for them (P1-T3, P3-T1, P3-T2, P4-T2).

## 3. Git workflow & commits

- **Branch + PR per task** (decision, Step 6 interview): open a PR even when working solo; self-review the diff against the task's acceptance criteria before merge; squash-merge to `main`.
- **CI must be green on every PR** (`fmt`, `clippy -D warnings`, `tsc --noEmit`, `eslint`, `cargo test`, `vitest`, later Playwright).
- **Commit messages: Conventional Commits** — `feat(P1-T2): add canvas waveform with zoom/pan`. Types: `feat`, `fix`, `test`, `docs`, `chore`, `refactor`, `perf`, `ci`. Scope = task ID. Imperative mood, ≤72-char subject; body explains *why* when non-obvious.
- Never commit: secrets, large binaries, generated `dist/`, editor junk, `.env` files. Fixtures come from `scripts/` (generated), except the tiny checked-in integration fixtures explicitly listed in the task.

## 4. Stack & dependencies

- Use **exactly** the stack in `architecture.md` §2 (Rust/axum/tokio/symphonia/hound/lame-sys/rayon; Vite/TS/no-framework; custom canvas). 
- **No new dependency** without a PR-note justification (why existing deps can't do it) and, if it changes the stack table, user approval of the doc edit.
- Keep the frontend dependency count near-zero; anything added must be small, maintained, and license-compatible with GPL-3.0 (NFR-9).
- Keep the backend dependency tree boring; prefer std/`tokio`/`axum`-ecosystem solutions.

## 5. Code style

### Rust (backend)

- `rustfmt` defaults; `clippy -D warnings` must pass (no `#[allow]` without a one-line reason).
- Naming: modules `snake_case`, types `UpperCamelCase`, functions `snake_case`; follow the module layout in `architecture.md` §11 — put code where the guide says, don't invent parallel structures.
- **No `unwrap()`/`expect()`/panic on user-controlled input or in handlers** — return typed errors (`thiserror`). `unwrap` is acceptable only in tests and provably-infallible init.
- Async discipline: no CPU work on async threads; audio passes go through `spawn_blocking`/rayon (arch §3.1).
- Errors: one `error.rs` enum per the arch; every API error maps to `{code, message, hint}` with stable `code` strings (`E_UNSUPPORTED_FORMAT`, `E_CORRUPT_FILE`, `E_EXPORT_FAILED`, …). List codes as they are introduced in the PRD Appendix checklist (P3-T1).
- Logging: `tracing` to stdout; `info` for lifecycle (source created, job done), `warn` for recoverable anomalies, `error` for failures. Never log audio contents or full paths outside temp; no request bodies in logs.

### TypeScript (frontend)

- `tsconfig` `strict: true`; ESLint + Prettier defaults as configured in `frontend/`; no `any` without a one-line justification; prefer `unknown` + narrowing at boundaries (API responses).
- Pure logic lives in dependency-free modules (`state/cutModel.ts`, `state/undo.ts`, playback decision functions) so vitest can test without DOM. DOM/canvas stays in `waveform/`, `ui/`, `audio/playback.ts`.
- All API shapes typed in `api/types.ts` mirroring `architecture.md` §6 — update types and docs together.
- Naming: files `kebab-case.ts`, types/classes `PascalCase`, functions/vars `camelCase`, constants `SCREAMING_SNAKE` only for true constants.
- CSS: plain CSS with custom properties named per `ui-guide.md` tokens (`--bg`, `--accent`, …); no CSS framework; no inline styles except dynamic positioning (canvas overlays).
- No user-facing raw exceptions: map failures to toasts per `ui-guide.md` error-copy rules (FR-21).

## 6. Testing requirements

- **Bar (interview decision): logic-covered, no coverage % gate.** Every task ships tests for its new decision-making logic: pure functions always; I/O paths via the integration suite; UI wiring via targeted vitest where logic exists, manual checks otherwise (recorded in the PR).
- Commands (must pass locally before PR): `cargo fmt --check && cargo clippy --D warnings && cargo test` in `backend/`; `npm run lint && npm run typecheck && npm test` in `frontend/`.
- Audio tests use **synthetic fixtures** generated by `scripts/` (known silences, known lengths). Never commit large audio; keep integration fixtures tiny (< 100 KB each).
- Assert on **behavior, not implementation** (candidate spans, durations, merge results, error codes).
- Flaky tests are bugs: fix or delete-with-rationale; never `skip` silently.
- The P1-T3 spike and P4-T2 acceptance are **manual measurement tasks** — record numbers in the Appendix, don't fake them with synthetic optimism.

## 7. Security & privacy habits (arch §7)

- Never accept filesystem paths from the client; never `Path::new(client_string)`.
- Never bind `0.0.0.0` by default; CORS is an explicit allowlist, never `*`.
- Upload sizes and JSON bodies are validated; filenames sanitized for `Content-Disposition` only.
- No telemetry, no third-party network calls, no CDN assets (NFR-3). A PR that adds one is rejected.
- No secrets exist in this project; if that ever changes, they come from env, never commits.

## 8. Performance guardrails

- **Never** load PCM into the browser (NFR-2 — hard rule). If a feature seems to need it, escalate.
- Frontend keeps JSON state small; peaks are binary and cached per window (arch §3.2).
- Respect the budgets in NFR-1; if a task's implementation threatens them, measure early (don't wait for P1-T3) and escalate with numbers.
- rAF loops must be cheap (binary search over merged spans — no linear scans per frame).

## 9. Definition of done (per task)

A task is **done** only when all of the following hold:

1. Every acceptance criterion in `plan.md` for that task is verified (test output or named manual check).
2. New/changed logic has tests per §6; full local command suite green.
3. Code follows §5 (format, lint, structure, error, logging rules).
4. No undocumented deviation from the binding documents (or the docs were updated via the §1.4 loop).
5. `plan.md` status updated; `README.md` tracker synced; Appendix rows filled if this task owns them.
6. PR opened (Conventional Commit, task ID in scope), self-reviewed, CI green, squash-merged.
7. No secrets/binaries committed; no scope creep from §1.5.

## 10. Definition of done (v1 release)

P4-T2 completes only when: success-metric protocol and Appendix are ✅ (`prd.md` §8), every FR-01…FR-23 has an evidence reference, all five documents are mutually consistent, README works verbatim on a clean machine with documented prerequisites, and full CI (incl. Playwright) is green on `main`.

## 11. Environment & configuration reference

| Variable | Default | Where |
|---|---|---|
| `SILENCE_CUTTER_ADDR` | `127.0.0.1:7817` | backend bind (loopback) |
| `SILENCE_CUTTER_ORIGINS` | Vite dev+preview localhost/127.0.0.1 on 5173/4173 | CORS allowlist (arch §6) |
| `SILENCE_CUTTER_TMP_TTL_SECS` | `3600` | idle source TTL (arch §3.1) |
| `SILENCE_CUTTER_MAX_UPLOAD_BYTES` | `4294967296` (4 GB) | upload guard |
| `VITE_API_BASE` | `http://127.0.0.1:7817` | frontend build/dev |

Never invent extra env flags; new configuration requires a doc update (§1.4).

## 12. Working with AI agents

An AI implementer following this repo must: read this guide + the task box in `plan.md` before any edit; keep tool-use diffs scoped to the task; ask (via the harness's question tool) at every ambiguity; update statuses and trackers; and treat the acceptance criteria as the only definition of done — not "the code looks finished."
