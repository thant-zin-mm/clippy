# ui-guide.md — Interface Guide

_Step 6 of the Idea → Documentation workflow (see `../todo.md`). Status: awaiting review._
_Binding visual/UX spec for **Silence Cutter** v1 (dark tool aesthetic, compact density, minimal motion — interview decisions, Step 6). Implementers: match this guide; deviations need a doc update (coding-guide §1.4)._

## 1. Brand & feel

A **focused dark studio tool** — the feel of a lightweight rack unit: quiet chrome, dense controls, the waveform is the hero. Long sessions (30–60 min of review) must stay comfortable: high contrast where it matters (waveform, timecodes, actions), muted everything else. No personality flourishes, no illustrations, no gradients except functional state fills.

Working product name in UI: **Silence Cutter** (header, 13 px, weight 600, `--text-primary`).

## 2. Color palette (CSS custom properties)

Dark-only; no theme toggle (FR-25 out of scope).

| Token | Hex | Role |
|---|---|---|
| `--bg` | `#0F1115` | App background |
| `--surface` | `#171A21` | Panels, sidebar, toolbar, status bar |
| `--surface-2` | `#1E222B` | Hover fills, input backgrounds on surfaces, modal body |
| `--border` | `#2A2F3A` | 1 px dividers, input/button borders |
| `--text-primary` | `#E6E9EF` | Primary text (≈ 15:1 on `--bg`) |
| `--text-secondary` | `#9AA3B2` | Labels, secondary text (≈ 8:1 on `--bg`) |
| `--text-faint` | `#6B7383` | Disabled text, ruler minor ticks (≥ 4.5:1 for non-essential only) |
| `--accent` | `#5B9CFF` | Primary actions, focus ring, playhead (≈ 6:1 on `--bg`) |
| `--accent-strong` | `#7DB0FF` | Hover of primary controls |
| `--accent-ink` | `#0B1020` | Text on `--accent` fills |
| `--success` | `#3FB950` | Export done, "kept" confirmations |
| `--warning` | `#D29922` | Candidate overlays, re-run warning, noise-floor hints |
| `--danger` | `#F85149` | Cut overlays, destructive buttons, errors |
| `--wave` | `#6E9BFF` | Waveform bars (rest) |
| `--wave-played` | `#A9C4FF` | Waveform bars before the playhead |

Functional washes (RGBA over `--bg`):

| Token | Value | Role |
|---|---|---|
| `--candidate-fill` | `rgba(210,153,34,0.20)` | Pending detected-silence overlay |
| `--candidate-border` | `#D29922` (60% alpha) | Its border |
| `--kept-fill` | `rgba(154,163,178,0.10)` | Rejected/kept-candidate overlay (muted) |
| `--cut-fill` | `rgba(248,81,73,0.22)` | Cut (removed) region overlay |
| `--cut-border` | `#F85149` (70% alpha) | Its border |
| `--selection-fill` | `rgba(91,156,255,0.16)` | Drag selection |
| `--overlay-scrim` | `rgba(0,0,0,0.55)` | Modal scrim |

Rules: never use pure black `#000` as a surface; never use color alone to encode state — cut regions also get a diagonal hatch pattern (CSS repeating-linear-gradient or canvas pattern), kept candidates get reduced opacity + a check glyph in the inspector.

## 3. Typography

```
--font-sans:  system-ui, -apple-system, "Segoe UI", Roboto, Ubuntu, sans-serif;
--font-mono:  ui-monospace, "SF Mono", "Cascadia Mono", Menlo, Consolas, monospace;
```

| Role | Font | Size / weight / line-height |
|---|---|---|
| App header, panel titles | sans | 13 px / 600 / 1.3 |
| Controls, buttons, inputs | sans | 13 px / 500 / 1.3 |
| Body & help text | sans | 12 px / 400 / 1.5 |
| Micro labels, badges | sans | 11 px / 500 / 1.2 |
| **All timecodes, dB values, counts, durations** | **mono, `font-variant-numeric: tabular-nums`** | 12 px / 400 / 1.3 |
| Ruler tick labels | mono | 11 px / 400, `--text-secondary` |

No webfont files (system stacks only). No italics except quoted dialog copy. Uppercase only for tiny badges (e.g. `WAV`, `MP3`).

## 4. Spacing, layout, grid, breakpoints

- **Spacing scale (4 px base):** `--sp-1: 4px`, `--sp-2: 8px`, `--sp-3: 12px`, `--sp-4: 16px`, `--sp-6: 24px`. Control inner padding uses `--sp-2`; panel padding `--sp-3`; gaps between panels `--sp-1` with 1 px borders instead of big gutters (compact density).
- **Radii:** controls `4 px`, panels/modals `6 px`. **Borders:** always 1 px `--border`. No drop shadows except modal: `0 8px 32px rgba(0,0,0,0.45)`.

**Layout (desktop, primary ≥ 1280 px):**

```
┌────────────────────────────────────────────────────────────┐
│ Header/toolbar 40px: [logo+name] [Import] [Detect] [Export]│
│                            [Undo][Redo]        (right: ok) │
├──────────────┬─────────────────────────────────────────────┤
│ Sidebar      │                                             │
│ 280px        │   Waveform stage (fills remaining space)    │
│ (surface)    │   ├ ruler 20px                              │
│ - Detect     │   ├ canvas (flex height ~ 220–320px)        │
│   panel      │   └ candidate/selection strips              │
│ - Inspector  │                                             │
│              ├─────────────────────────────────────────────┤
│              │ Status bar 28px: counters + both clocks     │
└──────────────┴─────────────────────────────────────────────┘
```

- Sidebar: detection settings (top), inspector (below, appears on candidate/selection). Waveform stage is the visual center; never shrink canvas below 160 px height.
- **Breakpoints:** ≥ 1280 px full layout; 1024–1279 px sidebar 240 px, tighter paddings; < 1024 px unsupported (NFR-6 desktop-only) — render a "desktop app; narrow window not supported" banner rather than a broken layout.
- The app does not scroll as a page; panels scroll internally (`overflow: auto`) with styled thin scrollbars (`--surface-2` track, `--border` thumb).

## 5. Components

### Buttons

Height 28 px (compact), padding 0 10 px, radius 4 px, font per §3; icon+label gap `--sp-1`.

| Variant | Style |
|---|---|
| Primary | `--accent` fill, `--accent-ink` text; hover `--accent-strong`; active translateY(1 px) |
| Secondary | `--surface-2` fill, `--border`, `--text-primary`; hover border `--text-faint` |
| Ghost | transparent; hover `--surface-2` |
| Danger | transparent, 1 px `--danger` border, `--danger` text; hover fill `rgba(248,81,73,0.12)` |
| Disabled | 45% opacity, `cursor: not-allowed`, no hover |

Focus (all interactive elements): `outline: 2px solid var(--accent); outline-offset: 1px;` via `:focus-visible` only.

### Inputs

Number/text inputs (threshold dB, min duration, keep-pause): height 28 px, `--surface-2` bg, 1 px `--border`, mono font for numeric values, unit suffix inside the field ("dB", "s") in `--text-secondary`. Steppers on hover for numbers (1 dB / 0.1 s steps). Invalid input: border `--danger` + inline hint text below (11 px `--danger`).

No checkboxes/sliders/radio groups in v1 except format choice in the export dialog (segmented control: `WAV | MP3`, secondary style, selected = accent border + `--surface-2` fill).

### Panels (sidebar sections)

`--surface` bg, 1 px bottom `--border`, section title row 28 px (title left, optional chevron). Content padding `--sp-3`. Empty inspector: centered `--text-secondary` 12 px text "Select a candidate on the waveform or drag to select a range."

### Modals (confirmations, export)

Centered, min-width 360 px, `--surface` bg, 1 px `--border`, radius 6 px, scrim `--overlay-scrim`; title 13/600; body 12/400; footer right-aligned buttons: Secondary (cancel) then Primary/Danger. Escape and scrim click cancel. Focus trapped inside while open; focus returns to the opener.

- **Re-run confirm (FR-12):** title "Re-run detection?", body states review state resets: "Re-running replaces all candidates and resets accept/reject decisions. You can undo this." Buttons: Cancel / Re-run (primary).
- **Export dialog:** format segmented control; summary lines in mono: source duration, result duration, "−mm:ss removed"; primary "Export"; during export, button becomes "Exporting… 42%" with inline determinate progress bar; on done: success toast + download starts automatically.

### Toasts

Bottom-right, stack upward, max-width 360 px, `--surface-2` bg, 1 px `--border`, radius 4 px, padding `--sp-2` `--sp-3`; 4 s auto-dismiss (errors: persist until dismissed). Left color bar 3 px: info `--accent`, success `--success`, warning `--warning`, error `--danger`. Copy format per §9.

### Progress bars

4 px tall, radius 2 px, track `--surface-2`, fill `--accent`, width = pct. Indeterminate: animated stripe (only allowed looping animation). Used for: upload %, prepare %, export %.

### Status bar (28 px, mono for values)

Left: `candidates 42 · cut 7 · kept 3` (`--text-secondary`, values `--text-primary`). Right: both clocks — `src 1:23:45 · result 1:10:02 · −13:43` with `--cut-fill` background chip on the removed amount. During playback update ≥ 10×/s (FR-06).

## 6. Waveform stage (canvas) styling

- Canvas bg `--bg`; DPR-scaled; ruler band 20 px on top (`--surface`, bottom 1 px `--border`), tick marks `--text-faint`, labels `--text-secondary` mono (adaptive steps, arch waveform/ruler).
- Waveform: vertical bars or filled min/max columns in `--wave`; samples left of playhead `--wave-played`; center line 1 px `--border`.
- Overlays (bottom → top): kept candidates → pending candidates → cut regions (hatched) → selection → playhead.
- Playhead: 1.5 px line `--accent` + 6 px triangle handle in the ruler.
- Selection: `--selection-fill` + 1 px `--accent` edges; while dragging show mono duration badge near cursor (`--surface-2`, 1 px border, 11 px mono).
- Hover cursor: thin `--text-faint` vertical line at pointer x (no timestamp tooltip — ruler + status bar cover time).
- Canvas focus: `:focus-visible` outline per §5; canvas gets `role="img"` and an `aria-label` updated on state change (e.g. "Waveform of townhall.mp3, 2:47:03 long, 42 candidates, 7 cuts, playhead at 12:03") — plus `aria-live="polite"` on the status-bar counters, never on the canvas label (too chatty).

## 7. Iconography, imagery, motion

- **Icons:** small hand-picked inline SVG set, 16×16 viewBox, `stroke-width: 1.5`, `stroke: currentColor`, `fill: none` (except play triangle). Needed: import (tray+arrow), scissors/cut, play, pause, undo, redo, export/download, sliders (detect), check (keep), x (dismiss), warning triangle, logo mark (simple waveform glyph). No icon font, no emoji, no third-party icon package (coding-guide §4).
- **Imagery:** none. The waveform is the only picture; empty state uses the logo glyph at 32 px `--text-faint`.
- **Motion:** 120 ms `ease` on color/opacity for hover/active; toasts 150 ms fade+8 px rise; modal 120 ms fade; progress fills transition width 100 ms linear. The only looping animation is the indeterminate progress stripe (1 s linear). No springs, no parallax, no page transitions. `@media (prefers-reduced-motion: reduce)`: keep only progress stripe; make everything else instant.

## 8. Accessibility standards (NFR-7 checklist)

- Keyboard: every action reachable — `Space` play/pause, `Ctrl/Cmd+Z` undo, `Ctrl+Shift+Z` redo, `Delete` cut selection, `+`/`-` zoom, arrows pan/seek, `Tab` cycles controls in visual order, `Enter`/`Space` activate focused buttons (shortcuts map: arch §3.2).
- Focus always visible (§5); focus order header → sidebar → waveform → status bar; modals trap focus.
- Text contrast ≥ 4.5:1 (palette chosen to satisfy; verify any new color with a contrast checker before adding).
- All icon-only buttons carry `aria-label` + `title`; all inputs have `<label>` (visually hidden OK).
- Status updates announced politely (status bar live region); errors announced via toast `role="alert"` for errors.
- 28 px control height is the minimum target (desktop pointer); no hover-only functionality.

## 9. Tone of voice / UI copy

- Sentence case; terse; imperative; no exclamation marks; no "Oops!"; no marketing voice.
- **Error copy = what happened + what to do**, one to two sentences:
  - Unsupported file: "This file type isn't supported. Use an MP3 or WAV file."
  - Corrupt: "Couldn't read this audio file. It may be corrupt — try exporting a fresh copy."
  - Backend unreachable: "Can't reach the local backend at 127.0.0.1:7817. Start it with `cargo run` in `backend/` (see README), then retry."
  - Export failed: "Export failed while writing the MP3. Check free disk space and try again."
- Buttons are verbs: "Auto-detect", "Cut this", "Keep this", "Cut selection", "Export".
- Numbers always mono and tabular: durations `h:mm:ss` (ruler, clocks) or seconds with unit (`1.0 s`, `-40 dB`); never raw floats with >2 decimals in UI.
- Confirmations name the consequence (see re-run modal copy in §5).

## 10. Do / Don't

- DO keep the waveform visually dominant at all times.
- DO distinguish cut vs candidate vs kept by fill + border + pattern (not color alone).
- DO keep controls dense but aligned to the 4 px grid.
- DON'T add features, panels, or settings not in the PRD.
- DON'T use pure white surfaces, gradients, or decorative illustration.
- DON'T block the UI thread — all heavy work is backend jobs with progress (arch §8).
- DON'T write cute error messages; follow §9.

---
_End of Step 6. Please review `docs/coding-guide.md` and `docs/ui-guide.md` and approve both (or request changes) before Step 7 — the `README.md` entry point._
