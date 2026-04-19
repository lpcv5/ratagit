#[allow(unused_imports)]
use std::collections::HashSet;
use super::CommitEntry;

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

fn traverse(from: i16, to: i16, traversed: &mut HashSet<i16>, taken: &mut HashSet<i16>) {
    let (left, right) = if from <= to { (from, to) } else { (to, from) };
    for i in left..=right {
        traversed.insert(i);
    }
    taken.insert(to);
}

fn next_available(exclude1: &HashSet<i16>, exclude2: &HashSet<i16>) -> i16 {
    for i in 0..i16::MAX {
        if !exclude1.contains(&i) && !exclude2.contains(&i) {
            return i;
        }
    }
    panic!("No available position found (graph too wide - exceeded {} branches)", i16::MAX);
}

/// Computes the next set of pipes for a commit.
///
/// Algorithm phases:
/// 1. Determine commit position (from incoming pipe or new rightmost)
/// 2. Add STARTS pipe for first parent
/// 3. Add TERMINATES pipes (incoming pipes ending at this commit)
/// 4. Add CONTINUES pipes with to_pos < pos (moving right to avoid conflicts)
/// 5. Add additional STARTS pipes for merge parents (2nd+ parents)
/// 6. Add CONTINUES pipes with to_pos > pos (potentially moving left)
/// 7. Sort by to_pos, then by kind priority
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

    // Reuse empty set to avoid repeated allocations
    let empty_set = HashSet::new();

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
            let avail = next_available(&traversed, &empty_set);
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

fn apply_pipe(cells: &mut [Cell], pipe: &Pipe, is_start: bool) {
    let left = pipe.left() as usize;
    let right = pipe.right() as usize;

    for i in left..=right {
        if i < cells.len() {
            cells[i].color_idx = pipe.color_idx;
        }
    }

    if pipe.from_pos == pipe.to_pos {
        let pos = pipe.from_pos as usize;
        if pos < cells.len() {
            cells[pos].up = !is_start;
            cells[pos].down = pipe.kind != PipeKind::Terminates;
        }
    } else {
        let from = pipe.from_pos as usize;
        let to = pipe.to_pos as usize;
        if from < cells.len() {
            cells[from].up = !is_start;
            cells[from].right = pipe.from_pos < pipe.to_pos;
            cells[from].left = pipe.from_pos > pipe.to_pos;
        }
        if to < cells.len() {
            cells[to].up = !is_start;
            cells[to].down = pipe.kind != PipeKind::Terminates;
            cells[to].left = pipe.from_pos > pipe.to_pos;
            cells[to].right = pipe.from_pos < pipe.to_pos;
        }
        for i in (left + 1)..right {
            if i < cells.len() {
                cells[i].left = true;
                cells[i].right = true;
            }
        }
    }
}

fn render_pipe_set(pipes: &[Pipe]) -> Vec<GraphCell> {
    if pipes.is_empty() {
        return vec![];
    }

    let max_pos = pipes.iter().map(|p| p.to_pos.max(p.from_pos)).max().unwrap_or(0) as usize;
    let mut commit_pos = 0;
    let mut start_count = 0;

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
    fn test_render_pipe_set_merge() {
        // Merge commit: two STARTS pipes at pos 0
        let pipes = vec![
            Pipe { from_pos: 0, to_pos: 0, from_hash: "a".to_string(), to_hash: "b".to_string(),
                   kind: PipeKind::Starts, color_idx: 1 },
            Pipe { from_pos: 0, to_pos: 1, from_hash: "a".to_string(), to_hash: "c".to_string(),
                   kind: PipeKind::Starts, color_idx: 2 },
        ];
        let cells = render_pipe_set(&pipes);
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0].chars, "⏣ "); // Merge symbol
        assert_eq!(cells[1].chars, "╭─"); // Branch starts
    }

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
            graph_prefix: String::new(),
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
        // aaa has two parents: bbb and ccc (merge commit)
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
}
