# M1 Core Workflow Hardening

> Status: Completed
> Last updated: 2026-03-20

## Goal

Make `add -> commit` complete and reliable for daily usage.

## Scope

### Add Workflow

- Keep single-file toggle behavior.
- Add visual selection mode in Files panel (`v` to enter/exit).
- In visual mode, `j/k` expands selection range from anchor to cursor.
- Keep `Space` as stage/unstage trigger for selected range.
- When a selected range fully covers a directory subtree, treat it as one directory target.

### Commit Workflow

- Block commit early when there is no staged change.
- Keep message validation in input flow.
- `c` stays a global commit shortcut.
- In Files visual mode, `c` first stages selected targets, then opens commit input.
- Ensure stable post-commit refresh:
  - status/tree/commits/diff refreshed
  - selection stays predictable
  - focus remains in Files panel

### UX and Feedback

- Standardize command log text for add/unstage/commit outcomes.
- Keep failure messages actionable.

## Out of Scope

- Stash write operations.
- Remote operations.
- Cherry-pick/rebase flows.

## Deliverables

- New messages/key bindings for visual selection and batch stage toggle.
- `GitRepository` APIs and `Git2Repository` implementation for multi-path add/unstage.
- Tests for batch stage/unstage behavior and commit guard behavior.

## Acceptance Criteria

- User can finish `stage -> commit` without leaving TUI.
- User can recover quickly from invalid commit attempts.
- No architecture boundary violations.

## Completion Notes

- Completed visual selection flow in Files panel (`v`, range select with `j/k`, `Esc` exit).
- Completed batch stage/unstage on selected range via `Space`.
- Completed directory-coverage behavior (full subtree selection treated as directory target).
- Completed commit workflow uplift:
  - `c` remains global commit shortcut
  - in visual mode, selected targets are staged before commit editor opens
  - separate commit editor panel with `message + description`, `Tab` switching, and multi-line description input
  - guard for commit attempts when there are no staged changes
- Verified local quality gates:
  - `cargo check`
  - `cargo test`
  - `cargo clippy --all-targets --all-features -- -D warnings`
