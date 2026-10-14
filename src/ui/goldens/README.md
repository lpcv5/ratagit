# UI golden buffers

`cargo test ui::` runs the UI tests without opening a terminal. The full test
suite (`cargo test`) and existing CI also compare these checked-in baselines.
No additional dependencies or snapshot tooling are required.

The fixtures use a temporary empty Git repository, the default keymap, and
fixed file/branch/commit/stash/patch data. They do not read the user's keymap or
repository. Full-layout tests require background work to be finished; the
loading indicator is tested separately with all ten explicit animation frames.

Coverage includes empty and populated layouts, all panel focus states,
overflow/scrolling, narrow terminals and resize, search and visual selection,
revision trees, branch commit subviews, graph highlighting, diff loading and
empty hints, hunk selection/applying/read-only states, ANSI colors, Unicode,
editors and confirmations, command filtering, and closing an overlay.
Input scenarios use the existing dispatcher/key mapping helpers.

## Reading a baseline

Each `.golden` file stores the buffer rectangle and visible input cursor, then:

- `text`: every cell's symbol, retaining all spaces and continuation cells after
  wide characters. These are cell dumps, so CJK/emoji may look wider than the
  original terminal when viewed in a text editor.
- `symbol boundaries`: `(x, y, symbol)` entries for empty or multi-codepoint
  symbols, preserving grapheme boundaries that concatenated text alone loses.
- `styles`: a deterministic palette of foreground/background/underline colors,
  modifiers and cell diff/skip flags, indexed in first-seen order.
- `style runs`: per-row cell ranges (`start..end`, end exclusive) and palette IDs.

Missing files and changes to text, positioning, colors, selection styles or
cursor positions fail with a diff. Baselines use UTF-8/LF; comparison also accepts
CRLF checkouts. Normal test runs never rewrite baselines.

## Updating an intentional UI change

Generate only the affected test's baseline(s), review the diff, then rerun tests
without the update variable. Never accept a change only to silence a failure.
Updates are rejected whenever `CI` is set.

PowerShell:

```powershell
$env:RATAGIT_UPDATE_GOLDEN = '1'
try {
    cargo test ui::golden_tests::test_empty_layout_golden
} finally {
    Remove-Item Env:RATAGIT_UPDATE_GOLDEN
}
git diff -- src/ui/goldens
cargo test ui::
```

Bash/WSL:

```bash
RATAGIT_UPDATE_GOLDEN=1 cargo test ui::golden_tests::test_empty_layout_golden
git diff -- src/ui/goldens
cargo test ui::
```

To regenerate all baselines, use `cargo test ui::golden_tests` as the filtered
command in the update step. Commit updated `.golden` files with their renderer
or fixture changes. Before submitting, run format, check, test and Clippy as
described in the repository README.
