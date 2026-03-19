# Ratagit Development Model

> Last updated: 2026-03-20

Ratagit uses a rolling milestone model instead of fixed phase plans.

## Principles

- Work in short, shippable slices.
- Keep one active milestone at a time.
- Track execution in `STATUS.md`.
- Keep milestone scope in standalone docs under `docs/milestones/`.
- Reprioritize quickly when implementation feedback changes priorities.

## Documentation Rules

- `MILESTONES.md`: index of milestones (active/completed/planned).
- `milestones/<milestone>.md`: single source of truth for one milestone.
- `STATUS.md`: day-to-day execution tracking, parity status, active risks.
- `DECISIONS.md`: architecture and technical ADRs.

## Workflow

1. Define/adjust milestone in `docs/milestones/`.
2. Mark it active in `MILESTONES.md`.
3. Execute and track progress in `STATUS.md`.
4. When done, move milestone to completed and activate the next one.

## Boundary

The architecture contract stays unchanged:

`UI -> Message -> update -> App -> GitRepository`
