#[allow(unused_imports)]
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
}
