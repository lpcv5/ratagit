# EXEC_PLAN.md

## Current Slice

Terminal visual polish: rounded panel borders, badge-style titles, and
badge-style bottom shortcuts.

## Goal

- replace square terminal panel corners with rounded corners
- avoid double-drawn shared panel edges by letting right-side and lower panels
  omit the shared border side
- render the focused panel with a complete border even when that side would be
  shared while inactive
- render numbered focus hints as reverse-video title badges
- render bottom shortcut keys as reverse-video badges without `keys(panel):`
  prefixes or pipe separators
- preserve pure rendering and `AppState` as the only source of truth

## Vertical Slice

1. Terminal frame rendering
- add deterministic panel border selection per grid position
- make focused panel border selection override shared-edge omission
- use rounded border symbols for terminal panel blocks
- render terminal panel titles from styled spans so the numbered hint can be a
  badge while the icon/title text keeps the panel accent
- render terminal shortcuts from structured key/action segments

2. Tests
- add focused UI buffer coverage for rounded/shared panel chrome and title badge
  styling
- add focused UI buffer coverage for shortcut key badge styling
- update terminal snapshots for the panel chrome and shortcut changes
- update harness coverage that asserts the visible title labels and Git state

3. Documentation
- update PRODUCT behavior notes for badge-style numbered focus hints
- update DESIGN visual theme notes for rounded/shared terminal panel chrome

4. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`

## Latest Validation

- `cargo fmt`
- `cargo clippy --workspace --lib --bins -- -D warnings`
- `cargo test`
