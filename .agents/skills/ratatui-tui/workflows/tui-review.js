// Dynamic-workflow template for reviewing ratatui TUI codebases.
// Treat as a template: tune `DIMENSIONS`, severity threshold, and prompts
// to the codebase before running. Invoke via the Workflow tool with
// args: { path: "src/" } (defaults to "src/").
//
// Pattern: fan-out one reviewer per TUI dimension, then adversarially
// verify each finding as soon as its review completes (pipeline, no
// barrier). Only verified findings are reported.

export const meta = {
  name: 'ratatui-tui-review',
  description: 'Review ratatui TUI code across 5 dimensions, adversarially verify findings',
  whenToUse: 'After substantial ratatui changes or before shipping a TUI release',
  phases: [
    { title: 'Review', detail: 'one reviewer per TUI dimension' },
    { title: 'Verify', detail: 'adversarial refutation of each finding' },
  ],
}

const target = (args && args.path) || 'src/'

const FINDINGS = {
  type: 'object',
  required: ['findings'],
  properties: {
    findings: {
      type: 'array',
      items: {
        type: 'object',
        required: ['file', 'line', 'title', 'detail', 'severity'],
        properties: {
          file: { type: 'string' },
          line: { type: 'number' },
          title: { type: 'string' },
          detail: { type: 'string' },
          severity: { type: 'string', enum: ['critical', 'major', 'minor'] },
        },
      },
    },
  },
}

const VERDICT = {
  type: 'object',
  required: ['isReal', 'reason'],
  properties: {
    isReal: { type: 'boolean' },
    reason: { type: 'string' },
  },
}

const DIMENSIONS = [
  {
    key: 'architecture',
    prompt: `TEA (Elm Architecture) compliance: single App/Model owning all state,
all mutations routed through update() via a Message/Action enum, view/render
functions take &self and perform no mutation or business logic, no rendering
from update. Flag state scattered across globals or rendered widgets.`,
  },
  {
    key: 'terminal-safety',
    prompt: `Terminal restoration and panics: uses ratatui::run() or init()/restore()
(which install a terminal-restoring panic hook) rather than hand-rolled raw-mode
setup without a hook; color_eyre::install() runs BEFORE terminal init; no
unwrap()/expect()/panic! outside tests; raw mode restored on every exit path
including errors propagated with ?.`,
  },
  {
    key: 'styling',
    prompt: `Styling rules: Stylize trait helpers (.bold(), .cyan(), .dim()) over
verbose Style::default().fg(...) chains; no hardcoded Color::White/Color::Black
(breaks light/dark terminals); consistent palette usage; text wrapped before
rendering into constrained areas.`,
  },
  {
    key: 'events',
    prompt: `Event handling: async apps use crossterm EventStream + tokio::select!
(never blocking event::read() inside async); sync apps poll with timeout when
animating; animations (e.g. tui-shimmer) driven by a tick event with phase
stored in the model, not wall-clock reads inside render; key handling covers
both press and repeat where it matters; quit always reachable.`,
  },
  {
    key: 'rendering-perf',
    prompt: `Render performance: no heavy allocation or I/O inside the draw closure;
layout-cache feature not accidentally disabled via default-features = false;
image protocols queried once at startup (Picker::from_query_stdio) with
StatefulImage reuse, never re-encoding per frame; widgets rebuilt cheaply or
cached when expensive.`,
  },
]

phase('Review')
const results = await pipeline(
  DIMENSIONS,
  (d) =>
    agent(
      `Review the ratatui TUI code under ${target} for ${d.key} issues.
${d.prompt}
Read the relevant source files. Report only concrete issues with exact
file:line locations — no speculation about code you did not read.`,
      { label: `review:${d.key}`, phase: 'Review', schema: FINDINGS },
    ),
  (review, d) =>
    review
      ? parallel(
          review.findings.map((f) => () =>
            agent(
              `Adversarially verify this ${d.key} finding in a ratatui codebase.
Finding: "${f.title}" at ${f.file}:${f.line} — ${f.detail}
Read ${f.file} and try to REFUTE it: is the code actually fine, is the
pattern intentional, or does the issue not exist at that location?
Default to isReal=false if you cannot confirm it from the code.`,
              { label: `verify:${d.key}:${f.file}`, phase: 'Verify', schema: VERDICT },
            ).then((v) => ({ ...f, dimension: d.key, verdict: v })),
          ),
        )
      : [],
)

const verified = results.flat().filter(Boolean)
const confirmed = verified.filter((f) => f.verdict && f.verdict.isReal)
const refuted = verified.length - confirmed.length
log(`${confirmed.length} confirmed findings (${refuted} refuted)`)

return {
  target,
  confirmed,
  summary: {
    critical: confirmed.filter((f) => f.severity === 'critical').length,
    major: confirmed.filter((f) => f.severity === 'major').length,
    minor: confirmed.filter((f) => f.severity === 'minor').length,
  },
}
