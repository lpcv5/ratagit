# Ratagit Development Model

> Last updated: 2026-03-20

Ratagit uses a milestone workflow managed in `.track/` via the `project-tracker` skill.

## Principles

- Work in short, shippable slices.
- Keep one active milestone at a time.
- Keep planning/execution state in `.track/status.yaml`.
- Keep milestone docs in `.track/milestones/`.
- Keep progress history append-only in `.track/history.jsonl`.
- Reprioritize quickly when implementation feedback changes priorities.

## Documentation Rules

- `.track/status.yaml`: goals, states, and active milestone mapping.
- `.track/milestones/<milestone>.md`: single source of truth for milestone scope and checklist.
- `.track/history.jsonl`: append-only execution log.
- `DECISIONS.md`: architecture and technical ADRs.

## Workflow

1. Use the `project-tracker` skill to initialize and recover tracking context in `.track/`.
2. Use the skill to create goals and milestone documents.
3. Confirm milestone scope and acceptance checklist before activation.
4. Keep exactly one active goal at a time.
5. Execute acceptance items incrementally and record progress in history.
6. Run delivery check through the skill and mark goal done only after all items pass.

## Mandatory Rule

- Do not use `docs/MILESTONES.md`, `docs/STATUS.md`, or `docs/milestones/*` for tracking anymore.
- All project tracking must go through `.track/` and the `project-tracker` skill.

## Boundary

The architecture contract stays unchanged:

`UI -> Message -> update -> App -> GitRepository`
