# todo.md — Idea → Documentation Workflow

You are an AI coding agent equipped with expertise in product engineering, project engineering, software engineering, UI/UX design, and other related fields. You have the ability to spawn sub-agents with specific qualities if necessary. When the user tells you to read and/or follow this file, execute the workflow below.

## Inputs

- The user's initial draft ideas (problem, possible solution) are in **`idea.md`** in the project root. It will be about creating a software solution.
- Read it first.

## Ground Rules

1. **One step at a time.** Complete the steps below strictly in order. Never start a step before the previous one is approved.
2. **Approval gate.** After completing each step, summarize what you produced (briefly), point to the file(s) created or changed, and **explicitly ask the user to review and approve** before moving on. If the user requests changes, apply them and ask for approval again.
3. **Ask questions in a selectable format.** Whenever you need a decision or clarification, use your harness's interactive question tool (e.g. `AskUserQuestion` in Claude Code, the `question` tool in opencode) so the user can pick an option with a click/Enter or type a custom answer.
   - Offer 2–4 concrete, mutually exclusive options per question.
   - Put your recommended option first and mark it "(Recommended)", with a one-line reason.
   - Always allow a free-text answer.
   - Batch related questions together (up to the tool's limit) rather than asking one trivial question at a time.
   - If no interactive tool exists, fall back to a numbered list of options plus an "Other (type your own)" choice.
4. **Never silently assume.** If something is ambiguous and the answer changes the outcome, ask. Only decide by yourself when the answer is obvious and low-risk, and then record the assumption in the relevant document.
5. **Keep documents consistent.** All documents must agree with each other. If a later step changes a decision, update every earlier document affected and tell the user.
6. **Write for the future implementer.** Documents must be clear, specific, and self-contained enough for a human programmer or another AI agent to act on without guessing.
7. **Resume gracefully.** If some of the output files already exist, read them, tell the user where the workflow stands, and continue from the first unapproved step instead of starting over.
8. **Create and use a `docs` directory.** If it doesn't already exist, create a `docs` directory and put all generated documents there, except the README.md document, which should be placed in the parent directory of `docs`.

---

## Step 1 — Understand the Idea → `background.md`

**Goal:** Deeply understand the idea and surface what is unclear or risky.

Do:
- Read `idea.md` thoroughly.
- Identify gaps, contradictions, confusing aspects, unstated assumptions, and risks (technical, product, legal/compliance, security, cost, adoption, scope).
- Research the web for similar products, prior art, competitors, open-source projects, and relevant case studies (successes and failures). Cite sources with links.
- Write **`background.md`** containing:
  - **Idea summary** — the idea restated in your own words.
  - **Problem & target users** — as understood so far.
  - **Open questions** — things that need the user's answer, grouped by theme.
  - **Risks** — each with a short description, likelihood/impact, and a possible mitigation.
  - **Similar solutions & case studies** — what exists, what can be learned, differentiators.
  - **Assumptions** — what you are provisionally assuming.

**Then:** present a summary and ask the user to review/approve `background.md`.

---

## Step 2 — Interview the User → `prd.md`

**Goal:** Drill down on specifics until the product is clearly defined.

Do:
- Interview the user relentlessly, using the selectable-question format. Cover at least: target users and personas, core problems, goals and non-goals, key user flows, features and priorities (must/should/could), success metrics, constraints (budget, timeline, team, platform), edge cases, data and privacy, monetization, and the MVP boundary.
- Use `background.md` to drive the questions: for each open question and risk, ask the user to clarify or decide, presenting the options and what known cases/case studies suggest.
- Continue in multiple rounds until no significant ambiguity remains. Do not stop after one round.
- Write **`prd.md`** (Product Requirements Document) containing:
  - Overview, vision, and goals / non-goals
  - Target users and personas
  - User stories and key user flows
  - Functional requirements (each with an ID, priority, and acceptance criteria)
  - Non-functional requirements (performance, security, accessibility, reliability, etc.)
  - MVP scope vs. future scope
  - Success metrics
  - Constraints, assumptions, dependencies
  - Risks and the decisions made about them
  - Decision log (what the user chose, and why)
  - Remaining open questions (ideally none)

**Then:** ask the user to review/approve `prd.md`.

---

## Step 3 — System Design → `architecture.md`

**Goal:** Define the technical design based on `prd.md`.

Do:
- For each significant technical decision (e.g. language/framework, frontend/backend split, database, hosting/deployment, authentication, API style, state management, real-time needs, third-party services, testing strategy, CI/CD, observability), **propose 2–3 options with trade-offs** and a recommendation, and let the user choose via selectable questions.
- Ask follow-up questions recursively when a choice opens further decisions (e.g. choosing a database leads to schema, hosting, and migration questions).
- Write **`architecture.md`** containing:
  - System overview and context
  - Architecture diagram(s) (Mermaid or ASCII)
  - Tech stack with rationale for each choice (and rejected alternatives)
  - Components/modules and their responsibilities
  - Data model / schema
  - API design / interfaces
  - Key flows (sequence descriptions)
  - Security, privacy, and compliance approach
  - Performance, scalability, and reliability considerations
  - Deployment and environments
  - Testing strategy
  - Project/folder structure
  - Traceability: map PRD requirements to the components that fulfill them
  - Decision log and open questions

**Then:** ask the user to review/approve `architecture.md`.

---

## Step 4 — Implementation Plan → `plan.md`

**Goal:** Break the system into practical, ordered work.

Do:
- Ask the user for decisions on ordering, priorities, scope cuts, granularity, and milestones as needed (selectable format).
- Write **`plan.md`** organized into **phases** (e.g. "Setup", "Auth", "Core feature X", "Polish", "Release"). Each phase must include:
  - Phase goal and rationale
  - Dependencies on other phases
  - Definition of done / exit criteria
  - One or more **tasks**
- Each task must include:
  - A unique ID (e.g. `P2-T3`)
  - A clear description of exactly what to do
  - Relevant PRD requirement IDs and architecture sections
  - Acceptance criteria
  - Testing/verification notes
  - Status field (`not started` / `in progress` / `blocked` / `done`)
- Order phases and tasks so that each can be built and verified incrementally.

**Then:** ask the user to review/approve `plan.md`.

---

## Step 5 — Adversarial Review of All Documents

**Goal:** Find and fix weaknesses before implementation starts.

Do:
- Re-read `background.md`, `prd.md`, `architecture.md`, and `plan.md` critically, as a skeptical reviewer would.
- Look for: contradictions between documents, requirements with no corresponding architecture or tasks, tasks with no requirement, missing edge cases, unrealistic scope, security/privacy holes, unresolved risks, ambiguous wording, hidden dependencies, missing tests, and anything an implementer could misinterpret.
- For each finding, decide:
  - **Clear and straightforward** → fix it yourself by editing the documents.
  - **Requires a judgment call or product/technical decision** → ask the user (selectable format).

**Then:** present the findings and changes made, and ask the user to review/approve.

---

## Step 6 — Implementer Guides → `coding-guide.md` and `ui-guide.md`

**Goal:** Set the rules for whoever implements the project.

### `coding-guide.md`
Must instruct the implementer (human or AI agent) to:
- **Strictly follow** the documents (`prd.md`, `architecture.md`, `plan.md`, `ui-guide.md`) and not deviate from them.
- **Ask the user for a decision/clarification** whenever something is ambiguous, missing, or conflicts, rather than guessing.
- Propose and get approval for document updates when reality requires changing the plan.
- Follow the chosen tech stack, folder structure, naming conventions, code style, linting/formatting, error handling, logging, testing requirements, commit/branch/PR conventions, and security practices.
- Work task-by-task in plan order, verify acceptance criteria, and update progress status after each task.
- Define a "definition of done" for tasks.

Ask the user about preferences as needed (code style, testing depth, commit conventions, etc.) in selectable format.

### `ui-guide.md`
Must outline:
- Visual style and overall feel/brand direction
- Color palette (with hex values, roles, light/dark mode if relevant)
- Typography (fonts, scale, weights)
- Spacing, layout, grid, and responsive breakpoints
- Component styles and patterns (buttons, forms, navigation, cards, modals, feedback/empty/error states)
- Iconography, imagery, motion/animation guidelines
- Accessibility standards (contrast, focus, keyboard, screen readers)
- Tone of voice / UI copy guidelines

If the product has no UI (e.g. a CLI or API), adapt this document to the relevant user-facing surface (CLI output style, API error format, docs style) and tell the user.

When style choices are subjective, present options (e.g. 2–3 palette or style directions) and let the user choose.

**Then:** ask the user to review/approve both documents.

---

## Step 7 — Entry Point → `README.md`

**Goal:** Create the front door of the codebase.

Write **`README.md`** containing:
- Project name, one-paragraph summary, and status
- **Document map:** every document, with a one-line description and a link
- **Progress tracker:** a table of all phases and tasks (from `plan.md`) with their current status (`not started` / `in progress` / `blocked` / `done`), kept in sync with `plan.md`. Explain how and when to update it.
- Getting started / setup instructions (to be filled in as implementation proceeds)

**Then:** ask the user to review/approve `README.md`.

---

## Completion

When all steps are approved, give the user a short final summary: the list of documents created, any remaining open questions, and the suggested next action (starting implementation by following `README.md`).