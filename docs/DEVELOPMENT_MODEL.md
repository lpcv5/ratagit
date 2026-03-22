# Ratagit Development Model

> Last updated: 2026-03-22

Ratagit uses a milestone-based workflow with incremental delivery.

## Principles

- Work in short, shippable slices.
- Keep one active milestone at a time.
- Reprioritize quickly when implementation feedback changes priorities.

## Documentation Rules

- Milestone scope and acceptance checklist should be maintained in project docs/PR descriptions.
- Progress records should be maintained in commit history and PR discussion.
- `DECISIONS.md`: architecture and technical ADRs.

## Workflow

1. Confirm milestone scope and acceptance checklist before activation.
2. Keep exactly one active milestone at a time.
3. Execute acceptance items incrementally.
4. Run delivery checks (`cargo check`, `cargo test`, `cargo clippy`) before marking milestone complete.

## Mandatory Rule

- Keep milestone tracking lightweight and explicit in the repository's active docs/PR context.

## Boundary

The runtime architecture contract is:

`UI -> Action -> Dispatcher -> Stores -> Effect Runtime -> AppStateSnapshot -> UI`
