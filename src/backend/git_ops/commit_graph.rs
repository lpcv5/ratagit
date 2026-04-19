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
