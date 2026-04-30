# Refactoring TODO

Source: `docs/REFACTORING_ANALYSIS.md`

## Validation Strategy

- Run focused tests after each small refactor slice.
- Defer full workspace validation until every planned refactor below is
  complete.
- Final validation:
  - `cargo fmt`
  - `cargo clippy --workspace --lib --bins -- -D warnings`
  - `cargo test`

## Phase 1: Foundations

- [x] Add `MenuChoice` trait and migrate repeated choice navigation.
- [x] Add `ConfirmDialog<T>` and migrate `PushForceConfirmState`.
- [x] Migrate `DiscardConfirmState`.
- [x] Migrate `StageAllConfirmState`.
- [x] Migrate `BranchDeleteConfirmState`.
- [x] Migrate `BranchForceDeleteConfirmState`.
- [x] Migrate `AutoStashConfirmState`.

## Phase 2: Menu State

- [x] Add reusable `Menu<T>` state.
- [x] Migrate `BranchDeleteMenuState`.
- [x] Migrate `BranchRebaseMenuState`.
- [x] Migrate `ResetMenuState` selection state while preserving reset danger
  confirmation context.
- [x] Simplify menu movement reducer arms with shared helpers.
- [x] Consider collapsing menu-specific movement `UiAction` variants only if it
  reduces code without making input semantics implicit.

## Phase 3: Tree Navigation

- [x] Add shared `TreeNavState`.
- [x] Migrate `FilesUiState` to use `TreeNavState`.
- [x] Migrate `CommitFilesUiState` to use `TreeNavState`.
- [x] Replace duplicate files and commit-files movement helpers with shared
  tree navigation helpers.
- [x] Replace duplicate multi-select range helpers where the shared state makes
  the behavior identical.
- [x] Keep Files-specific lightweight projection and Commit Files-specific
  `item_index_by_path` behavior explicit.

## Phase 4: Final Review

- [x] Re-run search for old duplicated state shapes and stale direct fields.
- [x] Run final full validation.
- [x] Update this TODO with completion status.
