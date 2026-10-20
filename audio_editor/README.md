# Silence Cutter

A minimal, local-first audio editor for very long spoken-word recordings (2–4 hour community discussions, panels, lectures). Drop in an MP3 or WAV file, review auto-detected silences on a zoomable waveform, accept/reject each candidate with a click, make manual cuts, and export a tightened WAV or MP3 — with no cloud upload, no accounts, and no Audacity learning curve.

**Status: implementation in progress (P0-T1 scaffold done; P1-T1 backend pipeline under review).** All product/technical documentation is complete and approved (Oct 2026); see the [progress tracker](#progress-tracker) below. The app is two fully separate deployables (D20): a static TypeScript frontend and a local Rust backend that owns all decoding, analysis, and export. Audio never leaves your machine in v1.

- License: [GPL-3.0](LICENSE)
- Audience: open-source; source-build only for v1 (release binaries post-v1)

## Document map

| Document | What it is |
|---|---|
| [idea.md](idea.md) | The original raw idea (input to the workflow) |
| [todo.md](todo.md) | The Idea → Documentation workflow that produced this docs set |
| [docs/background.md](docs/background.md) | Understanding of the idea: gaps, risks, prior art, assumptions (Step 1) |
| [docs/prd.md](docs/prd.md) | Product requirements: goals/non-goals, personas, FR/NFR tables, decision log D1–D22 (Step 2) |
| [docs/architecture.md](docs/architecture.md) | System design: stack, components, algorithms, API contract, D23–D34 (Step 3) |
| [docs/plan.md](docs/plan.md) | Implementation plan: 4 phases, 11 tasks with acceptance criteria, measurements appendix (Step 4) |
| [docs/coding-guide.md](docs/coding-guide.md) | Binding rules for implementers (humans and AI): process, style, tests, definition of done (Step 6) |
| [docs/ui-guide.md](docs/ui-guide.md) | Visual spec: palette, typography, layout, components, a11y, copy tone (Step 6) |
| [docs/dependency-licenses.md](docs/dependency-licenses.md) | Dependency license audit: permissive deps in-tree + MPL-2.0/LGPL obligations (plan P0-T1) |

If a document and this README disagree, the documents win — then fix the README.

## Progress tracker

Mirrors the task statuses in [`docs/plan.md`](docs/plan.md) — **always update both in the same PR** when a task changes state. Update a row only when the task's acceptance criteria are verified (see coding-guide "Definition of done").

| Phase | Task | Status |
|---|---|---|
| 0 · Setup | P0-T1 — Repository scaffold, health check, CI | done |
| 1 · Vertical slice | P1-T1 — Backend: upload + prepare pipeline | under review |
| 1 · Vertical slice | P1-T2 — Frontend: import flow + waveform + ruler + zoom/pan | not started |
| 1 · Vertical slice | P1-T3 — 🔒 Spike gate: measured real-file validation | not started |
| 1 · Vertical slice | P1-T4 — Detection end-to-end | not started |
| 1 · Vertical slice | P1-T5 — Edit model: cuts, manual selection, undo, status bar | not started |
| 1 · Vertical slice | P1-T6 — Playback: transport + both clocks + cut-skip | not started |
| 2 · Export | P2-T1 — Export job: WAV/MP3 assembly + download UI | not started |
| 3 · Hardening | P3-T1 — Edge cases, errors, lifecycle hardening | not started |
| 3 · Hardening | P3-T2 — Accessibility, shortcuts, UI-guide conformance | not started |
| 4 · Release | P4-T1 — Playwright e2e + CI completion | not started |
| 4 · Release | P4-T2 — Real-file acceptance, README, docs sync | not started |

Status values: `not started` / `in progress` / `under review` / `blocked` / `done` (`under review` = code awaits the user's review pass; mirrors `docs/plan.md`).

## Getting started

> Commands below were verified in P0-T1 (scaffold). MP3 decode/encode is not wired until P1/P2, so the C toolchain prerequisite is not needed yet; P4-T2 does a final clean-machine re-verification (NFR-9).

### Prerequisites

- **Rust** (stable toolchain) and a **C toolchain** (needed to build the MP3 encoder, libmp3lame)
- **Node.js** ≥ 20 (for the frontend build) and npm
- Linux, macOS, or Windows (desktop); Chromium- or Firefox-based browser

### Run (development — two commands, two terminals)

```bash
# Terminal 1 — backend (API on http://127.0.0.1:7817)
cd backend && cargo run

# Terminal 2 — frontend (UI on http://localhost:5173)
cd frontend && npm install && npm run dev
```

Open `http://localhost:5173`. Configuration env vars are listed in [`docs/coding-guide.md` §11](docs/coding-guide.md).

### Build (production-style local use)

```bash
cd backend && cargo build --release
cd frontend && npm install && npm run build   # dist/ + VITE_API_BASE baked in
# serve frontend/dist from any static server, e.g.:
cd frontend && npm run preview               # http://localhost:4173
```

## Contributing

Read [`docs/coding-guide.md`](docs/coding-guide.md) first — it is binding. Work task-by-task from [`docs/plan.md`](docs/plan.md), branch + PR per task, Conventional Commits (`feat(P1-T2): …`), CI must be green. When anything is ambiguous, ask instead of guessing.

## Privacy

v1 runs entirely on your machine: the API binds to `127.0.0.1`, audio is only ever sent from your browser to that local backend, and there is no telemetry. See [`docs/prd.md`](docs/prd.md) NFR-3.
