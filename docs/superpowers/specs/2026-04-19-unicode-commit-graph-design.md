# Unicode Commit Graph Design

**Date:** 2026-04-19  
**Status:** Approved

## Goal

Replace ratagit's current `git log --graph` ASCII prefix extraction with a pure-Rust port of lazygit's pipe-based graph algorithm, rendering unicode box-drawing characters with per-branch colors.

## Data Model

### New type: `GraphCell`

```rust
pub struct GraphCell {
    pub chars: String,   // always 2 chars (e.g. "│ ", "╭─", "◯ ")
    pub color_idx: u8,   // branch color index 0–7; 0 = default/muted
}
```

Color palette (8 entries, cycled by branch): cyan, yellow, green, magenta, blue, red, white, gray.

### `CommitEntry` change

```
- pub graph_prefix: String
+ pub graph_cells: Vec<GraphCell>
```

## Algorithm (`src/backend/git_ops/commit_graph.rs`)

Direct port of lazygit `graph.go` + `cell.go`.

### Core types

```rust
enum PipeKind { Starts, Continues, Terminates }

struct Pipe {
    from_pos: i16,
    to_pos:   i16,
    from_hash: String,
    to_hash:   String,
    kind:      PipeKind,
    color_idx: u8,
}
```

### Entry point

```rust
pub fn render_commit_graph(commits: &[CommitEntry]) -> Vec<Vec<GraphCell>>
```

Iterates commits, calling `get_next_pipes` then `render_pipe_set` for each row.

### `get_next_pipes`

Mirrors lazygit's algorithm exactly:
1. Filter out `TERMINATES` pipes from previous row.
2. Find position for current commit (first matching `toHash`, else `maxPos + 1`).
3. Emit a `STARTS` pipe for the commit's first parent.
4. For each continuing pipe: emit `CONTINUES`, potentially moving left into empty slots.
5. For merge commits: emit additional `STARTS` pipes for extra parents.
6. Sort pipes by `to_pos`, then by kind.

Color assignment: each new `STARTS` pipe gets `color_idx = branch_counter % 8` from a monotonic counter passed through the iteration.

### `render_pipe_set`

Builds a `Vec<Cell>` (one per column position), sets `up/down/left/right` flags per pipe, then calls `get_box_drawing_chars` to produce the 2-char string per cell.

### `get_box_drawing_chars`

Maps `(up, down, left, right)` → unicode pair, identical to lazygit:

| up | dn | lt | rt | char1 | char2 |
|----|----|----|-----|-------|-------|
| ✓  | ✓  | *  | *  | `│`   | ` `/`─` |
| ✓  | ✗  | ✗  | ✓  | `╰`   | `─`   |
| ✓  | ✗  | ✓  | ✗  | `╯`   | ` `   |
| ✗  | ✓  | ✗  | ✓  | `╭`   | `─`   |
| ✗  | ✓  | ✓  | ✗  | `╮`   | ` `   |
| ✓  | ✗  | ✓  | ✓  | `┴`   | `─`   |
| ✗  | ✓  | ✓  | ✓  | `┬`   | `─`   |
| ✗  | ✗  | ✓  | ✓  | `─`   | `─`   |
| …  | …  | …  | …  | …     | …     |

Commit cell: `◯ ` (regular) or `⏣ ` (merge, when `start_count > 1`).

## Integration Points

### `commits.rs`
- Remove `git_graph_prefix_map` function and its `git log --graph` CLI call.
- After `parse_commit_records`, call `render_commit_graph(&commits)` and zip results into `commit.graph_cells`.

### `CommitEntry`
- Replace `graph_prefix: String` with `graph_cells: Vec<GraphCell>`.
- Update `CommitRowRenderData.graph_prefix: String` → `graph_cells: Vec<GraphCell>`.

### `commit_panel.rs`
- In `row_columns()`: copy `graph_cells` from `CommitEntry` into `CommitRowRenderData`.
- In `render()`: replace the single `Span::styled(row.graph_prefix, muted_text_style())` with a loop over `graph_cells`, each becoming `Span::styled(cell.chars, color_for_idx(cell.color_idx))`.

## Testing

- Unit tests in `commit_graph.rs`:
  - Linear history: `◯ `, `│ `, `◯ `
  - Branch + merge: verify `╭─`, `⏣ `, `╰─` shapes
  - Mirroring key cases from lazygit's `graph_test.go`
- Existing `commits.rs` tests must continue to pass (no change to commit parsing logic).

## Out of Scope

- Selected-commit highlight (lazygit highlights the selected row's pipes in bold white) — can be added later
- Parallel rendering (lazygit uses goroutines) — not needed at typical commit counts
