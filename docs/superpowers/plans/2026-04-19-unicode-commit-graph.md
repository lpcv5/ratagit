# Unicode Commit Graph Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace ratagit's `git log --graph` ASCII prefix extraction with a pure-Rust port of lazygit's pipe-based graph algorithm, rendering unicode box-drawing characters with per-branch colors.

**Architecture:** A new `commit_graph.rs` module implements the pipe algorithm (ported from lazygit's `graph.go`/`cell.go`). `CommitEntry.graph_prefix: String` becomes `graph_cells: Vec<GraphCell>`. The commit panel renders each cell as a colored ratatui `Span`.

**Tech Stack:** Rust, ratatui (Color/Style/Span), git2 (parent hashes already in CommitEntry.parents)

---

## File Map

| Action | File | Responsibility |
|--------|------|----------------|
| Create | `src/backend/git_ops/commit_graph.rs` | Full pipe algorithm + GraphCell type |
| Modify | `src/backend/git_ops/mod.rs` | Add `mod commit_graph; pub use commit_graph::GraphCell;` |
| Modify | `src/backend/git_ops/commits.rs` | Replace `git_graph_prefix_map` with `render_commit_graph` |
| Modify | `src/components/panels/commit_panel.rs` | Render `graph_cells` as colored spans |

---

### Task 1: Create `commit_graph.rs` with core types and `get_box_drawing_chars`

**Files:**
- Create: `src/backend/git_ops/commit_graph.rs`
- Modify: `src/backend/git_ops/mod.rs`

- [ ] **Step 1: Write the failing test for `get_box_drawing_chars`**

Add to `src/backend/git_ops/commit_graph.rs`:

```rust
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub struct GraphCell {
    pub chars: String,
    pub color_idx: u8,
}

#[derive(Debug, Clone, PartialEq)]
enum PipeKind {
    Starts,
    Continues,
    Terminates,
}

#[derive(Debug, Clone)]
struct Pipe {
    from_pos: i16,
    to_pos: i16,
    from_hash: String,
    to_hash: String,
    kind: PipeKind,
    color_idx: u8,
}

impl Pipe {
    fn left(&self) -> i16 { self.from_pos.min(self.to_pos) }
    fn right(&self) -> i16 { self.from_pos.max(self.to_pos) }
}

#[derive(Debug, Clone, PartialEq)]
enum CellType { Connection, Commit, Merge }

#[derive(Debug, Clone)]
struct Cell {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    cell_type: CellType,
    color_idx: u8,
}

impl Cell {
    fn new() -> Self {
        Cell { up: false, down: false, left: false, right: false,
               cell_type: CellType::Connection, color_idx: 0 }
    }
}

fn get_box_drawing_chars(up: bool, down: bool, left: bool, right: bool) -> (&'static str, &'static str) {
    match (up, down, left, right) {
        (true,  true,  _,     true)  => ("│", "─"),
        (true,  true,  _,     false) => ("│", " "),
        (true,  false, true,  true)  => ("┴", "─"),
        (true,  false, true,  false) => ("╯", " "),
        (true,  false, false, true)  => ("╰", "─"),
        (true,  false, false, false) => ("╵", " "),
        (false, true,  true,  true)  => ("┬", "─"),
        (false, true,  true,  false) => ("╮", " "),
        (false, true,  false, true)  => ("╭", "─"),
        (false, true,  false, false) => ("╷", " "),
        (false, false, true,  true)  => ("─", "─"),
        (false, false, true,  false) => ("─", " "),
        (false, false, false, true)  => ("╶", "─"),
        (false, false, false, false) => (" ", " "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_drawing_vertical() {
        assert_eq!(get_box_drawing_chars(true, true, false, false), ("│", " "));
    }

    #[test]
    fn test_box_drawing_corner_top_left() {
        assert_eq!(get_box_drawing_chars(false, true, false, true), ("╭", "─"));
    }

    #[test]
    fn test_box_drawing_corner_bottom_right() {
        assert_eq!(get_box_drawing_chars(true, false, true, false), ("╯", " "));
    }

    #[test]
    fn test_box_drawing_t_bottom() {
        assert_eq!(get_box_drawing_chars(true, false, true, true), ("┴", "─"));
    }
}
```

- [ ] **Step 2: Register the module in `mod.rs`**

In `src/backend/git_ops/mod.rs`, add at the top:
```rust
mod commit_graph;
pub use commit_graph::GraphCell;
```

- [ ] **Step 3: Run tests to verify they pass**

```bash
rtk cargo test commit_graph
```
Expected: 4 tests pass.

- [ ] **Step 4: Commit**

```bash
rtk git add src/backend/git_ops/commit_graph.rs src/backend/git_ops/mod.rs
rtk git commit -m "feat: add commit_graph module with GraphCell type and box-drawing chars"
```

---

### Task 2: Implement `render_pipe_set`

**Files:**
- Modify: `src/backend/git_ops/commit_graph.rs`

- [ ] **Step 1: Write the failing test for `render_pipe_set`**

Add to the `tests` module in `commit_graph.rs`:

```rust
    #[test]
    fn test_render_pipe_set_single_commit() {
        // A single commit with no branches: one STARTS pipe at pos 0
        let pipes = vec![Pipe {
            from_pos: 0, to_pos: 0,
            from_hash: "abc".to_string(), to_hash: "def".to_string(),
            kind: PipeKind::Starts, color_idx: 1,
        }];
        let cells = render_pipe_set(&pipes);
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].chars, "◯ ");
        assert_eq!(cells[0].color_idx, 1);
    }

    #[test]
    fn test_render_pipe_set_continuing_pipe() {
        // A commit at pos 0 with a continuing pipe at pos 1
        // STARTS at pos 0, CONTINUES from pos 1 to pos 1
        let pipes = vec![
            Pipe { from_pos: 0, to_pos: 0, from_hash: "a".to_string(), to_hash: "b".to_string(),
                   kind: PipeKind::Starts, color_idx: 1 },
            Pipe { from_pos: 1, to_pos: 1, from_hash: "x".to_string(), to_hash: "y".to_string(),
                   kind: PipeKind::Continues, color_idx: 2 },
        ];
        let cells = render_pipe_set(&pipes);
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0].chars, "◯ ");
        assert_eq!(cells[1].chars, "│ ");
    }

    #[test]
    fn test_render_pipe_set_merge_commit() {
        // Two STARTS pipes at same pos = merge commit symbol
        let pipes = vec![
            Pipe { from_pos: 0, to_pos: 0, from_hash: "m".to_string(), to_hash: "p1".to_string(),
                   kind: PipeKind::Starts, color_idx: 1 },
            Pipe { from_pos: 0, to_pos: 1, from_hash: "m".to_string(), to_hash: "p2".to_string(),
                   kind: PipeKind::Starts, color_idx: 2 },
        ];
        let cells = render_pipe_set(&pipes);
        assert_eq!(cells[0].chars, "⏣ ");
    }
```

- [ ] **Step 2: Implement `render_pipe_set`**

Add before the `#[cfg(test)]` block in `commit_graph.rs`:

```rust
fn apply_pipe(cells: &mut Vec<Cell>, pipe: &Pipe, override_color: bool) {
    let left = pipe.left() as usize;
    let right = pipe.right() as usize;

    if left != right {
        for i in (left + 1)..right {
            cells[i].left = true;
            cells[i].right = true;
            if override_color || cells[i].color_idx == 0 {
                cells[i].color_idx = pipe.color_idx;
            }
        }
        cells[left].right = true;
        if override_color || cells[left].color_idx == 0 {
            cells[left].color_idx = pipe.color_idx;
        }
        cells[right].left = true;
        if override_color || cells[right].color_idx == 0 {
            cells[right].color_idx = pipe.color_idx;
        }
    }

    match pipe.kind {
        PipeKind::Starts | PipeKind::Continues => {
            let idx = pipe.to_pos as usize;
            cells[idx].down = true;
            if override_color || cells[idx].color_idx == 0 {
                cells[idx].color_idx = pipe.color_idx;
            }
        }
        PipeKind::Terminates => {}
    }
    match pipe.kind {
        PipeKind::Terminates | PipeKind::Continues => {
            let idx = pipe.from_pos as usize;
            cells[idx].up = true;
            if override_color || cells[idx].color_idx == 0 {
                cells[idx].color_idx = pipe.color_idx;
            }
        }
        PipeKind::Starts => {}
    }
}

fn render_pipe_set(pipes: &[Pipe]) -> Vec<GraphCell> {
    let max_pos = pipes.iter().map(|p| p.right()).max().unwrap_or(0) as usize;
    let mut commit_pos = 0usize;
    let mut start_count = 0u32;

    for pipe in pipes {
        if pipe.kind == PipeKind::Starts {
            start_count += 1;
            commit_pos = pipe.from_pos as usize;
        } else if pipe.kind == PipeKind::Terminates {
            commit_pos = pipe.to_pos as usize;
        }
    }

    let is_merge = start_count > 1;
    let mut cells: Vec<Cell> = (0..=max_pos).map(|_| Cell::new()).collect();

    for pipe in pipes {
        if pipe.kind == PipeKind::Starts {
            apply_pipe(&mut cells, pipe, true);
        }
    }
    for pipe in pipes {
        if pipe.kind != PipeKind::Starts
            && !(pipe.kind == PipeKind::Terminates
                && pipe.from_pos as usize == commit_pos
                && pipe.to_pos as usize == commit_pos)
        {
            apply_pipe(&mut cells, pipe, false);
        }
    }

    cells[commit_pos].cell_type = if is_merge { CellType::Merge } else { CellType::Commit };

    cells
        .iter()
        .map(|cell| {
            let chars = match cell.cell_type {
                CellType::Commit => "◯ ".to_string(),
                CellType::Merge => "⏣ ".to_string(),
                CellType::Connection => {
                    let (a, b) = get_box_drawing_chars(cell.up, cell.down, cell.left, cell.right);
                    format!("{a}{b}")
                }
            };
            GraphCell { chars, color_idx: cell.color_idx }
        })
        .collect()
}
```

- [ ] **Step 3: Run tests**

```bash
rtk cargo test commit_graph
```
Expected: 7 tests pass.

- [ ] **Step 4: Commit**

```bash
rtk git add src/backend/git_ops/commit_graph.rs
rtk git commit -m "feat: implement render_pipe_set with unicode box-drawing chars"
```

---

### Task 3: Implement `get_next_pipes` and `render_commit_graph`

**Files:**
- Modify: `src/backend/git_ops/commit_graph.rs`

- [ ] **Step 1: Write failing tests**

Add to the `tests` module:

```rust
    use crate::backend::git_ops::CommitEntry;
    use crate::backend::git_ops::{CommitDivergence, CommitStatus};

    fn make_commit(id: &str, parents: &[&str]) -> CommitEntry {
        CommitEntry {
            id: id.to_string(),
            short_id: id[..7.min(id.len())].to_string(),
            summary: "test".to_string(),
            body: None,
            author: String::new(),
            author_name: String::new(),
            author_email: String::new(),
            timestamp: 0,
            parents: parents.iter().map(|s| s.to_string()).collect(),
            divergence: CommitDivergence::None,
            decorations: String::new(),
            tags: vec![],
            status: CommitStatus::None,
            graph_cells: vec![],
            is_branch_head: false,
        }
    }

    #[test]
    fn test_linear_graph_two_commits() {
        let commits = vec![
            make_commit("aaa", &["bbb"]),
            make_commit("bbb", &[]),
        ];
        let graph = render_commit_graph(&commits);
        assert_eq!(graph.len(), 2);
        // First commit: single commit symbol
        assert_eq!(graph[0].len(), 1);
        assert_eq!(graph[0][0].chars, "◯ ");
        // Second commit: commit symbol (pipe terminates here)
        assert_eq!(graph[1][0].chars, "◯ ");
    }

    #[test]
    fn test_branch_graph() {
        // aaa has two children: bbb and ccc (bbb is first parent)
        // Graph (top = newest):
        //   aaa  ← merge commit (two parents)
        //   bbb  ← first parent branch
        //   ccc  ← second parent branch
        let commits = vec![
            make_commit("aaa", &["bbb", "ccc"]),
            make_commit("bbb", &[]),
            make_commit("ccc", &[]),
        ];
        let graph = render_commit_graph(&commits);
        assert_eq!(graph.len(), 3);
        // aaa is a merge commit
        assert_eq!(graph[0][0].chars, "⏣ ");
    }
```

- [ ] **Step 2: Implement `get_next_pipes`**

Add before `render_pipe_set` in `commit_graph.rs`:

```rust
fn traverse(from: i16, to: i16, traversed: &mut HashSet<i16>, taken: &mut HashSet<i16>) {
    let (left, right) = if from <= to { (from, to) } else { (to, from) };
    for i in left..=right {
        traversed.insert(i);
    }
    taken.insert(to);
}

fn next_available(exclude1: &HashSet<i16>, exclude2: &HashSet<i16>) -> i16 {
    let mut i = 0i16;
    loop {
        if !exclude1.contains(&i) && !exclude2.contains(&i) {
            return i;
        }
        i += 1;
    }
}

fn get_next_pipes(prev_pipes: &[Pipe], commit: &CommitEntry, color_idx: u8) -> Vec<Pipe> {
    let max_pos = prev_pipes.iter().map(|p| p.to_pos).max().unwrap_or(-1);

    let current: Vec<&Pipe> = prev_pipes
        .iter()
        .filter(|p| p.kind != PipeKind::Terminates)
        .collect();

    // Find position for this commit
    let mut pos = max_pos + 1;
    for pipe in &current {
        if pipe.to_hash == commit.id {
            pos = pipe.to_pos;
            break;
        }
    }

    let mut new_pipes: Vec<Pipe> = Vec::new();
    let mut taken: HashSet<i16> = HashSet::new();
    let mut traversed: HashSet<i16> = HashSet::new();

    // Spots occupied by continuing pipes (not terminating here)
    let continuing_spots: HashSet<i16> = current
        .iter()
        .filter(|p| p.to_hash != commit.id)
        .map(|p| p.to_pos)
        .collect();

    // STARTS pipe for first parent
    let to_hash = commit.parents.first().cloned().unwrap_or_default();
    new_pipes.push(Pipe {
        from_pos: pos, to_pos: pos,
        from_hash: commit.id.clone(), to_hash,
        kind: PipeKind::Starts, color_idx,
    });

    // TERMINATES pipes (pipes ending at this commit)
    for pipe in &current {
        if pipe.to_hash == commit.id {
            new_pipes.push(Pipe {
                from_pos: pipe.to_pos, to_pos: pos,
                from_hash: pipe.from_hash.clone(), to_hash: pipe.to_hash.clone(),
                kind: PipeKind::Terminates, color_idx: pipe.color_idx,
            });
            traverse(pipe.to_pos, pos, &mut traversed, &mut taken);
        }
    }

    // CONTINUES pipes with to_pos < pos
    for pipe in &current {
        if pipe.to_hash != commit.id && pipe.to_pos < pos {
            let avail = next_available(&traversed, &HashSet::new());
            new_pipes.push(Pipe {
                from_pos: pipe.to_pos, to_pos: avail,
                from_hash: pipe.from_hash.clone(), to_hash: pipe.to_hash.clone(),
                kind: PipeKind::Continues, color_idx: pipe.color_idx,
            });
            traverse(pipe.to_pos, avail, &mut traversed, &mut taken);
        }
    }

    // Additional STARTS pipes for merge parents
    if commit.parents.len() > 1 {
        for (i, parent) in commit.parents[1..].iter().enumerate() {
            let avail = next_available(&taken, &continuing_spots);
            new_pipes.push(Pipe {
                from_pos: pos, to_pos: avail,
                from_hash: commit.id.clone(), to_hash: parent.clone(),
                kind: PipeKind::Starts,
                color_idx: color_idx.wrapping_add(1 + i as u8) % 8,
            });
            taken.insert(avail);
        }
    }

    // CONTINUES pipes with to_pos > pos (potentially moving left)
    for pipe in &current {
        if pipe.to_hash != commit.id && pipe.to_pos > pos {
            let mut last = pipe.to_pos;
            for i in (pos + 1..=pipe.to_pos).rev() {
                if taken.contains(&i) || traversed.contains(&i) {
                    break;
                }
                last = i;
            }
            new_pipes.push(Pipe {
                from_pos: pipe.to_pos, to_pos: last,
                from_hash: pipe.from_hash.clone(), to_hash: pipe.to_hash.clone(),
                kind: PipeKind::Continues, color_idx: pipe.color_idx,
            });
            traverse(pipe.to_pos, last, &mut traversed, &mut taken);
        }
    }

    // Sort by to_pos, then by kind (Terminates=0, Starts=1, Continues=2)
    new_pipes.sort_by(|a, b| {
        a.to_pos.cmp(&b.to_pos).then_with(|| {
            let ord = |k: &PipeKind| match k {
                PipeKind::Terminates => 0,
                PipeKind::Starts => 1,
                PipeKind::Continues => 2,
            };
            ord(&a.kind).cmp(&ord(&b.kind))
        })
    });

    new_pipes
}
```

- [ ] **Step 3: Implement `render_commit_graph`**

Add after `get_next_pipes`:

```rust
pub fn render_commit_graph(commits: &[CommitEntry]) -> Vec<Vec<GraphCell>> {
    if commits.is_empty() {
        return vec![];
    }

    let sentinel = "START".to_string();
    let mut pipes = vec![Pipe {
        from_pos: 0, to_pos: 0,
        from_hash: sentinel.clone(),
        to_hash: commits[0].id.clone(),
        kind: PipeKind::Starts,
        color_idx: 0,
    }];

    commits
        .iter()
        .enumerate()
        .map(|(i, commit)| {
            let color_idx = (i % 8) as u8;
            pipes = get_next_pipes(&pipes, commit, color_idx);
            render_pipe_set(&pipes)
        })
        .collect()
}
```

- [ ] **Step 4: Run tests**

```bash
rtk cargo test commit_graph
```
Expected: 9 tests pass.

- [ ] **Step 5: Commit**

```bash
rtk git add src/backend/git_ops/commit_graph.rs
rtk git commit -m "feat: implement get_next_pipes and render_commit_graph"
```

---

### Task 4: Update `CommitEntry` and `commits.rs`

**Files:**
- Modify: `src/backend/git_ops/commits.rs` (lines ~30-57, ~72-110, ~206-245)

- [ ] **Step 1: Replace `graph_prefix` with `graph_cells` in `CommitEntry`**

In `src/backend/git_ops/commits.rs`, find:
```rust
    #[allow(dead_code)] // One-line graph prefix
    pub graph_prefix: String,
```
Replace with:
```rust
    pub graph_cells: Vec<crate::backend::git_ops::GraphCell>,
```

- [ ] **Step 2: Update `parse_commit_records` to initialize `graph_cells`**

In `parse_commit_records`, find:
```rust
            graph_prefix: String::new(),
```
Replace with:
```rust
            graph_cells: vec![],
```

- [ ] **Step 3: Replace `git_graph_prefix_map` call in `load_commits`**

In `load_commits`, find:
```rust
    let graph_prefixes = git_graph_prefix_map(workdir, ref_spec, limit)?;
```
Delete that line.

Find:
```rust
    for commit in &mut commits {
        commit.graph_prefix = graph_prefixes.get(&commit.id).cloned().unwrap_or_default();
        commit.is_branch_head = branch_heads.contains(&commit.id);
```
Replace with:
```rust
    let graph = crate::backend::git_ops::commit_graph::render_commit_graph(&commits);
    for (commit, cells) in commits.iter_mut().zip(graph.into_iter()) {
        commit.graph_cells = cells;
        commit.is_branch_head = branch_heads.contains(&commit.id);
```

Note: `commit_graph` is a private module, so use the full path or add `use super::commit_graph;` at the top of the function.

- [ ] **Step 4: Delete `git_graph_prefix_map` function**

Remove the entire `git_graph_prefix_map` function (lines ~206-245 in the original file).

- [ ] **Step 5: Run tests**

```bash
rtk cargo check && rtk cargo test
```
Expected: all tests pass, no compile errors.

- [ ] **Step 6: Commit**

```bash
rtk git add src/backend/git_ops/commits.rs
rtk git commit -m "feat: wire render_commit_graph into load_commits, remove git_graph_prefix_map"
```

---

### Task 5: Update `commit_panel.rs` to render colored spans

**Files:**
- Modify: `src/components/panels/commit_panel.rs` (lines ~279-401, ~458-462)

- [ ] **Step 1: Update `CommitRowRenderData`**

Find:
```rust
struct CommitRowRenderData {
    columns: [String; 6],
    hash_style: ratatui::style::Style,
    author_style: ratatui::style::Style,
    graph_prefix: String,
    branch_head_marker: bool,
    tags: String,
    summary: String,
}
```
Replace with:
```rust
struct CommitRowRenderData {
    columns: [String; 6],
    hash_style: ratatui::style::Style,
    author_style: ratatui::style::Style,
    graph_cells: Vec<crate::backend::git_ops::GraphCell>,
    branch_head_marker: bool,
    tags: String,
    summary: String,
}
```

- [ ] **Step 2: Update `row_columns` to copy `graph_cells`**

Find:
```rust
        graph_prefix: commit.graph_prefix.clone(),
```
Replace with:
```rust
        graph_cells: commit.graph_cells.clone(),
```

- [ ] **Step 3: Add color helper function**

Add after `commit_hash_style` function (around line 362):

```rust
fn graph_cell_style(color_idx: u8) -> ratatui::style::Style {
    use ratatui::style::Color;
    let color = match color_idx % 8 {
        0 => Color::DarkGray,
        1 => Color::Cyan,
        2 => Color::Yellow,
        3 => Color::Green,
        4 => Color::Magenta,
        5 => Color::Blue,
        6 => Color::Red,
        7 => Color::White,
        _ => Color::DarkGray,
    };
    ratatui::style::Style::default().fg(color)
}
```

- [ ] **Step 4: Update render loop to use colored spans**

Find:
```rust
                        if !row.graph_prefix.is_empty() {
                            spans.push(Span::styled(row.graph_prefix.clone(), muted_text_style()));
                        }
```
Replace with:
```rust
                        for cell in &row.graph_cells {
                            spans.push(Span::styled(
                                cell.chars.clone(),
                                graph_cell_style(cell.color_idx),
                            ));
                        }
```

- [ ] **Step 5: Run full test suite**

```bash
rtk cargo check && rtk cargo test
```
Expected: all tests pass.

- [ ] **Step 6: Run clippy**

```bash
rtk cargo clippy --all-targets --all-features -- -D warnings
```
Expected: 0 warnings.

- [ ] **Step 7: Commit**

```bash
rtk git add src/components/panels/commit_panel.rs
rtk git commit -m "feat: render commit graph cells as colored ratatui spans"
```

---

### Task 6: Final gate

- [ ] **Step 1: Run full local gate**

```bash
cargo fmt --check && rtk cargo check && rtk cargo test && rtk cargo clippy --all-targets --all-features -- -D warnings
```
Expected: all pass.

- [ ] **Step 2: Commit if any fmt fixes needed**

```bash
cargo fmt
rtk git add -u
rtk git commit -m "chore: fmt after unicode graph implementation"
```
