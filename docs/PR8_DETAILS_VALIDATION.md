# PR #8 Details migration and validation

Status: completed. Source: PR #8 (`origin/main`, tree at `9e40757`).
Target: `feature/newarch`, beginning at `eef701b`.

The user superseded the initial decision to keep the simplified newarch Details
diff. PR #8 is authoritative for that diff; the surrounding newarch UI and pure
AppContext/Command/GitBackend architecture remain in use.

## Tracking

- [x] Restore mode, range and independent selection semantics.
- [x] Port Details gutter, markers, diff colors and status presentation.
- [x] Add unit regressions, reviewed snapshots and real Git keyboard scenarios.
- [x] Actually run Files -> Enter -> Details in Windows ConPTY.
- [x] Complete formatting, compilation, workspace tests and strict Clippy.

## Required matrix

Fixtures contain three native hunks separated by unchanged text. Every scenario
starts in Files and enters Details with Enter. Verify exact index and worktree
contents, including the changes deliberately left unselected.

| Selection | Stage | Unstage | Discard |
| --- | --- | --- | --- |
| Single hunk | PASS | PASS | PASS |
| Multiple nonadjacent hunks | PASS | PASS | PASS |
| `v` range spanning hunks | PASS | PASS | PASS |

## Evidence

Each matrix cell passed through the production input mapper with Runtime +
HybridGitBackend, and twice through the compiled executable in Windows ConPTY
(80 x 24), including a rerun after cursor/viewport fixes. Real fixtures contain
60 lines, with edits on lines
5, 25 and 45, producing three native hunks. A separate staged `b.txt` sentinel
must remain unchanged in the index in every case.

| Selection | Keys after Files `Enter` | Chosen changes |
| --- | --- | --- |
| Single hunk | `j` | line 25 |
| Multiple nonadjacent hunks | `x j j x` | lines 5 and 45 |
| Visual hunk range | `v j` | lines 5 and 25 |

Stage/unstage use `Space` on the appropriate side in the automated matrix and
the initial terminal run. The final terminal rerun uses staged-side `d` for all
three unstage cases; an additional automated test covers that key. Staged-only
fixtures select the staged side on entry. Discard uses `d`, verifies the confirmation, then
`Enter`. Git assertions compare complete index/worktree contents, not just
status letters. Unselected edits, worktree bytes during stage/unstage, and the
staged sentinel all matched. Every real terminal run exited normally with `q`.

Reproducible terminal runner: `scripts/verify-pr8-details.ps1 -Binary <exe>`.
Rerun evidence: `tmp/pr8-details-terminal/20261016-050603-683/results.txt`;
initial run: `tmp/pr8-details-terminal/20261016-045452-431/results.txt`.
Each run contains `verification.txt` and retained Git fixtures in each of the nine case
directories. Automated keyboard scenarios save screen, AppContext, input
sequence and Git state under `tmp/pr8-details-keyboard/<selection-operation>`;
artifacts are also emitted on assertion failures. These local artifacts are
ignored by Git; the runner and assertions are versioned.

The six new Details snapshots cover independent marks, line range, hunk range,
staged palette, loading, and narrow gutters. Six existing Details/stash/scroll
snapshots are intentionally updated. Style assertions check cursor, mark and
staged backgrounds, preserving addition/removal polarity.

## Behavioral fixes

- `v` no longer forces line mode; it selects whole contiguous change blocks in
  Hunk mode, matching PR #8. A second `v` freezes the range into marks.
- `x` exposes PR #8's independent selection action without conflicting with
  `Space` apply. Marked hunks take precedence over an unmarked cursor.
- `a` toggles Hunk/Line directly. Line navigation includes context; writes
  exclude it. Mode/side/document changes clear stale marks.
- `Esc` clears marks before leaving Details. The range's live cursor drives
  scrolling and viewport resize, rather than its first selected line.
- Hunk scrolling fits the whole replacement block before reserving context;
  oversized changes start at their first row. Clamped movement retains manual
  scrolling. The viewport height now uses the renderer's ratatui constraints:
  the old remainder calculation could overestimate it by one row and clip the
  added half of a replacement. Rendering tests cover terminal heights 14–39.
- Cursor/write membership uses hash sets rather than repeated linear searches
  across a large selected block. The renderer builds presentation rows once per
  frame and uses that same projection for counts, gutters and visible content.
- Like PR #8, staged-side `d` unstages. Hunk writes relocate the cursor using
  path, reviewed changed bytes and nearest index coordinate, following an applied
  change to its destination side. An unmarked cursor stays on its surviving
  change when an earlier marked hunk disappears. Explicit movement cancels
  pending relocation; failed writes do not relocate.
- Dual line numbers come from presentation metadata, including combined
  previews. Native reviewed bytes and source LineIds continue to authorize
  writes; display strings never do. Narrow panes hide the number columns.
- The PR #8 add/remove backgrounds distinguish staged rows, cursor and marks;
  normal context keeps its default style. List highlighting no longer replaces
  diff polarity. Loading/applying disables old selection highlights.

## Final validation

All passed on Windows stable Rust:

- `cargo +stable fmt --check`
- `cargo +stable check --locked`
- `cargo +stable test --locked`: 589 passed, zero failures/ignored.
- `cargo +stable clippy --locked --all-targets --all-features -- -D warnings`
- `git diff --check`

Full test output: `tmp/pr8-details-final-tests.log`. The final ConPTY run's raw
terminal output is retained as `terminal-transcript.txt` beside its results.
The viewport fix intentionally updates the old 14-row Details scroll snapshot:
its last text line is now reachable. Outer panel layout and colors are unchanged.
No Unix terminal verification is claimed on this Windows host.
