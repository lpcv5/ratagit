//! Commit graph topology and render projection.

use crate::git::GraphCell;
use std::collections::HashSet;

#[derive(Clone, Copy)]
pub(super) struct GraphCharset {
    node: char,
    merge_node: char,
    horizontal: char,
    ascii: bool,
}

impl GraphCharset {
    fn unicode() -> Self {
        Self {
            node: '◯',
            merge_node: '⏣',
            horizontal: '─',
            ascii: false,
        }
    }

    fn ascii() -> Self {
        Self {
            node: '*',
            merge_node: '#',
            horizontal: '-',
            ascii: true,
        }
    }
}

#[derive(Clone)]
pub(super) struct GraphCommitRow {
    pub(super) oid: git2::Oid,
    pub(super) parents: Vec<git2::Oid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PipeKind {
    Terminates,
    Starts,
    Continues,
}

#[derive(Debug, Clone)]
struct Pipe {
    from_pos: i16,
    to_pos: i16,
    from_hash: Option<git2::Oid>,
    to_hash: Option<git2::Oid>,
    kind: PipeKind,
}

#[derive(Debug, Clone, Copy, Default)]
struct CellConnections {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

pub(super) fn graph_charset_from_env() -> GraphCharset {
    graph_charset(std::env::var("RATAGIT_GRAPH_ASCII").ok().as_deref())
}

fn graph_charset(value: Option<&str>) -> GraphCharset {
    match value {
        Some("1") => GraphCharset::ascii(),
        _ => GraphCharset::unicode(),
    }
}

pub(super) fn build_commit_graph_lines(
    rows: &[GraphCommitRow],
    charset: GraphCharset,
) -> Vec<Vec<GraphCell>> {
    if rows.is_empty() {
        return Vec::new();
    }

    let pipe_sets = get_pipe_sets(rows);
    rows.iter()
        .zip(pipe_sets.iter())
        .map(|(row, pipes)| render_pipe_set(row, pipes, charset))
        .collect()
}

fn get_pipe_sets(rows: &[GraphCommitRow]) -> Vec<Vec<Pipe>> {
    if rows.is_empty() {
        return Vec::new();
    }

    let mut sets = Vec::with_capacity(rows.len());
    let mut prev = vec![Pipe {
        from_pos: 0,
        to_pos: 0,
        from_hash: None,
        to_hash: Some(rows[0].oid),
        kind: PipeKind::Starts,
    }];

    for row in rows {
        let next = get_next_pipes(&prev, row);
        sets.push(next.clone());
        prev = next;
    }

    sets
}

fn get_next_pipes(prev: &[Pipe], row: &GraphCommitRow) -> Vec<Pipe> {
    let active: Vec<Pipe> = prev
        .iter()
        .filter(|p| p.kind != PipeKind::Terminates)
        .cloned()
        .collect();

    let max_pos = active.iter().map(|p| p.to_pos).max().unwrap_or(-1);
    let commit_pos = active
        .iter()
        .find(|p| p.to_hash == Some(row.oid))
        .map(|p| p.to_pos)
        .unwrap_or(max_pos + 1);

    // Pass 1: identify terminating pipes and free their spots from taken_spots.
    let mut taken_spots: HashSet<i16> = active.iter().map(|p| p.to_pos).collect();
    for pipe in &active {
        if pipe.to_hash == Some(row.oid) {
            taken_spots.remove(&pipe.to_pos);
        }
    }

    // traversed_spots: only mark positions for non-vertical (diagonal/horizontal) moves.
    let mut traversed_spots: HashSet<i16> = HashSet::new();
    for pipe in &active {
        if pipe.from_pos != pipe.to_pos {
            for pos in pipe.from_pos.min(pipe.to_pos)..=pipe.from_pos.max(pipe.to_pos) {
                traversed_spots.insert(pos);
            }
        }
    }

    let mut next = Vec::new();
    let mut has_terminating_incoming = false;

    // Pass 2: process terminating pipes first so their freed columns are visible.
    for pipe in &active {
        if pipe.to_hash == Some(row.oid) {
            has_terminating_incoming = true;
            next.push(Pipe {
                from_pos: pipe.to_pos,
                to_pos: commit_pos,
                from_hash: pipe.to_hash,
                to_hash: Some(row.oid),
                kind: PipeKind::Terminates,
            });
        }
    }

    // Pass 3: compact and continue non-terminating pipes.
    for pipe in &active {
        if pipe.to_hash == Some(row.oid) {
            continue;
        }

        let mut target = pipe.to_pos;
        if pipe.to_pos < commit_pos {
            while target > 0 {
                let candidate = target - 1;
                if candidate == commit_pos
                    || taken_spots.contains(&candidate)
                    || traversed_spots.contains(&candidate)
                {
                    break;
                }
                target = candidate;
            }
        } else if pipe.to_pos > commit_pos {
            while target > commit_pos + 1 {
                let candidate = target - 1;
                if candidate == commit_pos
                    || taken_spots.contains(&candidate)
                    || traversed_spots.contains(&candidate)
                {
                    break;
                }
                target = candidate;
            }
        }

        next.push(Pipe {
            from_pos: pipe.to_pos,
            to_pos: target,
            from_hash: pipe.from_hash,
            to_hash: pipe.to_hash,
            kind: PipeKind::Continues,
        });
        taken_spots.insert(target);
        if pipe.to_pos != target {
            for pos in pipe.to_pos.min(target)..=pipe.to_pos.max(target) {
                traversed_spots.insert(pos);
            }
        }
    }

    if let Some(first_parent) = row.parents.first().copied() {
        let first_kind = if has_terminating_incoming {
            PipeKind::Continues
        } else {
            PipeKind::Starts
        };
        next.push(Pipe {
            from_pos: commit_pos,
            to_pos: commit_pos,
            from_hash: Some(row.oid),
            to_hash: Some(first_parent),
            kind: first_kind,
        });
        taken_spots.insert(commit_pos);
        traversed_spots.insert(commit_pos);

        for parent in row.parents.iter().skip(1).copied() {
            // Try to reuse a freed column before allocating rightward.
            let mut pos = commit_pos + 1;
            // Find lowest free slot not in taken or traversed.
            let mut found = false;
            for candidate in (0..commit_pos).rev() {
                if !taken_spots.contains(&candidate) && !traversed_spots.contains(&candidate) {
                    pos = candidate;
                    found = true;
                    break;
                }
            }
            if !found {
                pos = commit_pos + 1;
                while taken_spots.contains(&pos) || traversed_spots.contains(&pos) {
                    pos += 1;
                }
            }
            next.push(Pipe {
                from_pos: commit_pos,
                to_pos: pos,
                from_hash: Some(row.oid),
                to_hash: Some(parent),
                kind: PipeKind::Starts,
            });
            taken_spots.insert(pos);
            for p in commit_pos.min(pos)..=commit_pos.max(pos) {
                traversed_spots.insert(p);
            }
        }
    }

    next.sort_by_key(|p| (p.to_pos, p.kind));
    next
}

fn render_pipe_set(row: &GraphCommitRow, pipes: &[Pipe], charset: GraphCharset) -> Vec<GraphCell> {
    let commit_pos = commit_position(row, pipes).max(0) as usize;
    let width = pipes
        .iter()
        .map(|p| p.from_pos.max(p.to_pos) as usize)
        .max()
        .unwrap_or(commit_pos)
        .max(commit_pos)
        + 1;

    // Build a map from column index to all OIDs that pass through each column.
    let mut col_oids: Vec<HashSet<String>> = vec![HashSet::new(); width];
    for pipe in pipes {
        let from = pipe.from_pos.max(0) as usize;
        let to = pipe.to_pos.max(0) as usize;
        if from >= width || to >= width {
            continue;
        }
        let start = from.min(to);
        let end = from.max(to);
        for oid_set in col_oids.iter_mut().take(end + 1).skip(start) {
            if let Some(h) = pipe.from_hash {
                oid_set.insert(h.to_string());
            }
            if let Some(h) = pipe.to_hash {
                oid_set.insert(h.to_string());
            }
        }
    }
    if commit_pos < width {
        // Ensure commit node cell always belongs to the row commit itself.
        col_oids[commit_pos].insert(row.oid.to_string());
    }

    let mut cells = vec![CellConnections::default(); width];
    for pipe in pipes {
        draw_pipe(pipe, &mut cells);
    }

    let mut out = Vec::with_capacity(width * 2);
    for (idx, conn) in cells.into_iter().enumerate() {
        let is_node = idx == commit_pos;
        let first = if is_node {
            if row.parents.len() > 1 {
                charset.merge_node
            } else {
                charset.node
            }
        } else {
            box_char(conn, charset)
        };
        let second = if is_node {
            ' '
        } else if conn.right {
            charset.horizontal
        } else {
            ' '
        };

        let mut pipe_oids: Vec<String> = col_oids[idx].iter().cloned().collect();
        pipe_oids.sort();
        let pipe_oid = pipe_oids.first().cloned();
        out.push(GraphCell {
            text: first.to_string(),
            lane: idx,
            pipe_oid: pipe_oid.clone(),
            pipe_oids: pipe_oids.clone(),
        });
        out.push(GraphCell {
            text: second.to_string(),
            lane: idx,
            pipe_oid,
            pipe_oids,
        });
    }

    while out.last().map(|c| c.text == " ").unwrap_or(false) {
        out.pop();
    }
    if out.is_empty() {
        out.push(GraphCell {
            text: " ".to_string(),
            lane: 0,
            pipe_oid: None,
            pipe_oids: Vec::new(),
        });
    }
    out
}

fn commit_position(row: &GraphCommitRow, pipes: &[Pipe]) -> i16 {
    pipes
        .iter()
        .find(|p| p.from_hash == Some(row.oid))
        .map(|p| p.from_pos)
        .or_else(|| {
            pipes
                .iter()
                .find(|p| p.to_hash == Some(row.oid))
                .map(|p| p.to_pos)
        })
        .unwrap_or(0)
}

fn draw_pipe(pipe: &Pipe, cells: &mut [CellConnections]) {
    let from = pipe.from_pos.max(0) as usize;
    let to = pipe.to_pos.max(0) as usize;
    if from >= cells.len() || to >= cells.len() {
        return;
    }

    if pipe.kind != PipeKind::Starts {
        cells[from].up = true;
    }
    if pipe.kind != PipeKind::Terminates {
        cells[to].down = true;
    }

    if from < to {
        cells[from].right = true;
        for cell in cells.iter_mut().take(to).skip(from + 1) {
            cell.left = true;
            cell.right = true;
        }
        cells[to].left = true;
    } else if from > to {
        cells[from].left = true;
        for cell in cells.iter_mut().take(from).skip(to + 1) {
            cell.left = true;
            cell.right = true;
        }
        cells[to].right = true;
    }
}

fn box_char(conn: CellConnections, charset: GraphCharset) -> char {
    let CellConnections {
        up,
        down,
        left,
        right,
    } = conn;
    if charset.ascii {
        return match (up, down, left, right) {
            (true, true, true, true) => '+',
            (true, true, true, false) => '+',
            (true, true, false, true) => '+',
            (true, true, false, false) => '|',
            (true, false, true, true) => '+',
            (true, false, true, false) => '\\',
            (true, false, false, true) => '/',
            (true, false, false, false) => '|',
            (false, true, true, true) => '+',
            (false, true, true, false) => '\\',
            (false, true, false, true) => '/',
            (false, true, false, false) => '|',
            (false, false, true, true) => '-',
            (false, false, true, false) => '-',
            (false, false, false, true) => '-',
            _ => ' ',
        };
    }

    match (up, down, left, right) {
        (true, true, true, true) => '┼',
        (true, true, true, false) => '┤',
        (true, true, false, true) => '├',
        (true, true, false, false) => '│',
        (true, false, true, true) => '┴',
        (true, false, true, false) => '╯',
        (true, false, false, true) => '╰',
        (true, false, false, false) => '╵',
        (false, true, true, true) => '┬',
        (false, true, true, false) => '╮',
        (false, true, false, true) => '╭',
        (false, true, false, false) => '╷',
        (false, false, true, true) => '─',
        (false, false, true, false) => '╴',
        (false, false, false, true) => '╶',
        (false, false, false, false) => ' ',
    }
}

/// Helper to flatten graph cells into a plain string (for tests and backward compat).
#[cfg(test)]
fn graph_cells_to_string(cells: &[GraphCell]) -> String {
    cells.iter().map(|c| c.text.as_str()).collect()
}

#[cfg(test)]
mod tests;
