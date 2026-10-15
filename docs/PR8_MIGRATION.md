# PR #8 migration to newarch

Follow-up: the user made PR #8 authoritative for the right Details diff.
`PR8_DETAILS_VALIDATION.md` supersedes this document's original simplified
Details UI/selection decision and records the actual nine-case terminal matrix.
The surrounding newarch panels and architecture remain the baseline.

## Contract and baseline

- Requested baseline: feature/newarch at 7a2221d.
- Source: [PR #8](https://github.com/lpcv5/ratagit/pull/8), main at 9e40757
  (15 commits, 303 files). Its head fd13e41 has the same tree.
- Common ancestor: f5a7805; PRs #3–#7 are already shared ancestry.
- Migration branch: codex/newarch-pr8-migration, based directly on newarch.
- Delivery branch: local feature/newarch, fast-forwarded to the migration commit.
- Original checkout has unrelated uncommitted branch-management work. It remains
  intact and is outside the merged PR #8 source snapshot.
- Preserve newarch UI, AppContext, pure reducers/rendering, GitBackend, workspace
  decomposition, and advanced branch/commit/rebase operations.

## Progress

- [x] Fetch refs, inspect PR metadata, isolate dirty checkout.
- [x] Audit all 15 PR #8 commits with source/target decisions.
- [x] Implement and verify migration slices.
- [x] Review new/changed UI snapshots; preserve existing newarch presentation.
- [x] Formatting, compilation and strict Clippy.
- [x] Final full-workspace regression; local feature/newarch branch handover.

## Migration matrix

Every PR #8 commit appears below. Equivalent means newarch already has the
behavior and its existing regression remains authoritative.

| ID | Source commits | Target decision | Evidence |
| --- | --- | --- | --- |
| A1 | 7e7caa6, b5707fa, 0f5fdc5 | Keep newarch workspace/core/UI/GitBackend boundaries, not legacy stores | Existing capability tests; ARCHITECTURE.md; pure panel projections |
| A2 | 02c1e60 | Keep immutable newarch presentation and pure rendering | panel_projection tests and existing snapshots |
| A3 | b79e69e | Keep newarch branch/commit subviews, typed work and editor state | active_left_view_routes_branch_and_main_subviews; stale_branch_subview_results_are_ignored; commit editor/rewrite suites |
| G1 | 7e7caa6, 6fae4b5 | Port lossless patch model, block/hunk/line ranges, stage/unstage/discard | 10 native backend tests; 7 partial core tests; real Git partial UI scenario |
| G2 | 6fae4b5 | Native combined previews map to original source IDs; overlap falls back safely | test_combined_preview_maps_to_independent_source_bytes; test_overlapping_changes_keep_reviewed_source_previews |
| F1 | 8954a6c | Add stage-all toggle, edit, copy path, anchored literal ignore and next/previous file | extras backend tests; input/editor tests; real Git file/stash scenario |
| F2 | b4c8338 | Keep cached newarch tree topology and path reconciliation | shared_tree_index_syncs_status_metadata_without_changing_children; lightweight tree tests; existing stage/unstage harness |
| D1 | eb64856, 6fae4b5 | Keep existing three-row list padding; add patch viewport/side preservation and explicit resize inputs | scroll_offset_*; reversing-direction harness; mode/viewport core test |
| D2 | 0c1ee8c | Add byte budgets, hit promotion and oversized-replacement rejection to target caches | details_cache tests; existing cache invalidation/stale-result suites |
| R1 | 0c1ee8c, 0f5fdc5 | Keep mutation barriers/request IDs; add patch ownership and bounded shutdown | stale_read_results_are_dropped_after_a_queued_mutation; partial stale-load test; blocked-read shutdown test |
| R2 | 8f33ad7 | Require ratatui 0.30.2; retain RAII cleanup, add panic restoration/editor resume and no-TTY guard | Windows lifecycle tests; Unix-only PTY restore test added |
| U1 | f33a302, 9e40757 | Keep newarch modal/input theme and add executable text-filtered contextual palette | input precedence tests; palette filter/no-match snapshots; existing editor/modal snapshots |
| T1 | d3af3b4 | Retain insta/TestBackend rather than main golden infrastructure | Existing snapshots preserved; 7 new snapshots; one intentional command-modal update |
| T2 | 0c1ee8c | Port ProcessFixture environment isolation, bounded children, native diagnostic code/class text | startup/config-isolation process tests; no parent environment mutation |
| C1 | eab067b, 8f33ad7, 0c1ee8c | Respect newarch instructions/docs; lock dependencies; add profiling profile; Ubuntu/Windows four-check CI | CI/docs updated; local checks below |

Additional missing main workflows now available: fetch (F, preserving newarch's
lowercase f fixup), read-only stash preview, apply without removal and confirmed
drop. These use the existing newarch layout and command/runtime boundaries.

## Regression findings fixed during migration

1. Newarch had no partial-staging flow. All writes now use reviewed native bytes
   and reject stale, duplicate/context-only, wrong-direction, binary, truncated,
   renamed, mode-changing, or read-only selections.
2. Filtering by path before rename detection hid staged renames as additions.
   Added/deleted index patches now check full rename metadata before authorizing
   partial writes; the real Git rename regression passes.
3. EOF pseudo-lines could split a replacement into two change blocks. They no
   longer split blocks; CRLF and absent-final-newline bytes remain lossless.
4. Cache entry count alone allowed oversized retention. Each of three cache
   kinds now has 16 entries and an 8 MiB owned key/value capacity budget (24 MiB
   combined payload ceiling); hits update recency.
5. Runtime Drop could wait indefinitely for a blocked read. Worker joining is
   bounded to 100 ms and queued jobs stop at shutdown.
6. A redirected Windows child could enter the event loop and wait for input.
   Startup now requires an interactive terminal before alternate-screen entry.
7. Commit fixup's f binding conflicted with newly added fetch. Fetch uses F.
8. New stash selection/search now schedules the matching read-only patch, with
   request IDs preventing stale responses from replacing current content.

## Validation

Windows stable toolchain: C:\Users\pclinger\.cargo\bin (verified at task time).
Build cache: C:\Users\pclinger\prj\ratagit-worktree\target.

- Formatting: passed.
- Compilation with --locked: passed.
- Clippy with --locked --all-targets --all-features -- -D warnings: passed.
- Focused checks: 10 native partial-patch regressions, 7 core selection/ownership
  tests, cache tests, file/stash backend tests, real Git UI scenarios, shutdown,
  isolated terminal/configuration checks and snapshots passed.
- Final full workspace run: passed, 567 tests, zero failures and zero ignored.
  Earlier failures were addressed individually rather than accepting all snapshot
  changes. The 567 include 76 UI tests, 83 mock harness scenarios, 8 real Git
  scenarios/infrastructure checks and 3 isolated process tests.
- Snapshot review: 7 new snapshots cover blocks, narrow range, partial discard,
  filtered/no-match palette, stash preview and stash drop. Only the existing
  command palette snapshot changes: labels, added commands and filter hints.
- Local Unix execution: not available (WSL Ubuntu has no Rust). The CI matrix
  now includes Ubuntu and Windows; the Unix PTY test must pass there.
- Windows interactive PTY smoke: opened a real repository file patch, selected
  a line range, filtered contextual commands, returned to Files and quit with
  exit code 0 and alternate-screen/cursor restoration. No Git mutation was
  performed during this smoke session.
- Baseline visual files theme.rs, layout.rs, modal.rs and editor_modal.rs are
  byte-for-byte identical to 7a2221d. All existing UI snapshots pass, with only
  the documented command-palette snapshot intentionally updated.

## Portability decisions

- Old main UI components/goldens, broad StateAccess stores, and old overlay
  styling are replaced by existing newarch equivalents, not transplanted.
- Main's separate help overlay and palette are consolidated into newarch's one
  executable contextual command modal; text filtering uses arrows for selection.
- Legacy keymap/config file formats and main's theme system are not introduced
  as an incompatible second configuration/state system. Current newarch keys
  and UI remain the contract; newly migrated commands are listed in PRODUCT.md.
- Main's latency-diagnosis goal uses existing newarch tracing and perf-suite;
  the profiling Cargo profile retains release optimization with symbols.
- Existing unrelated dirty branch rename/sort/checkout-by-name work is not
  silently included. It remains available in the original checkout.
- No remote branch is pushed or merged by this task.

## Practical limits

Native in-flight calls cannot be forcibly cancelled; detached workers exit after
their current call and skip queued jobs. External desktop clipboard/editor
integration depends on the user's OS services/editor; argument safety and input
routing are tested without overwriting the user's clipboard. Mock demonstration
patches have no native application bytes and remain read-only.
