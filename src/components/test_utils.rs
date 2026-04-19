//! Test utilities for component rendering tests using ratatui's TestBackend

use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

use crate::app::CachedData;
use crate::backend::git_ops::{BranchEntry, CommitEntry, StashEntry, StatusEntry};

/// Create a test terminal with the specified dimensions
pub fn create_test_terminal(width: u16, height: u16) -> Terminal<TestBackend> {
    let backend = TestBackend::new(width, height);
    Terminal::new(backend).expect("Failed to create test terminal")
}

/// Extract a line from the buffer as a string (trimmed)
pub fn get_buffer_line(buffer: &Buffer, row: u16) -> String {
    let area = buffer.area();
    if row >= area.height {
        return String::new();
    }

    let mut line = String::new();
    for col in 0..area.width {
        let cell = &buffer[(col, row)];
        line.push_str(cell.symbol());
    }
    line.trim_end().to_string()
}

/// Check if buffer contains text at a specific position
#[allow(dead_code)]
pub fn assert_buffer_contains(buffer: &Buffer, row: u16, col: u16, text: &str) {
    let area = buffer.area();
    assert!(
        row < area.height,
        "Row {} out of bounds (height: {})",
        row,
        area.height
    );
    assert!(
        col < area.width,
        "Col {} out of bounds (width: {})",
        col,
        area.width
    );

    let mut actual = String::new();
    for (i, _) in text.chars().enumerate() {
        let c = col + i as u16;
        if c >= area.width {
            break;
        }
        let cell = &buffer[(c, row)];
        actual.push_str(cell.symbol());
    }

    assert_eq!(
        actual, text,
        "Expected '{}' at ({}, {}), found '{}'",
        text, row, col, actual
    );
}

/// Create a CachedData instance with test file data
#[allow(dead_code)] // Used in tests
pub fn create_test_cached_data_with_files(files: Vec<StatusEntry>) -> CachedData {
    CachedData {
        files,
        ..Default::default()
    }
}

/// Create a CachedData instance with test branch data
#[allow(dead_code)] // Used in tests
pub fn create_test_cached_data_with_branches(branches: Vec<BranchEntry>) -> CachedData {
    CachedData {
        branches,
        ..Default::default()
    }
}

/// Create a CachedData instance with test commit data
#[allow(dead_code)]
pub fn create_test_cached_data_with_commits(commits: Vec<CommitEntry>) -> CachedData {
    CachedData {
        commits,
        ..Default::default()
    }
}

/// Create a CachedData instance with test stash data
#[allow(dead_code)] // Used in tests
pub fn create_test_cached_data_with_stashes(stashes: Vec<StashEntry>) -> CachedData {
    CachedData {
        stashes,
        ..Default::default()
    }
}

/// Create a test StatusEntry
#[allow(dead_code)] // Used in tests
pub fn test_status_entry(
    path: &str,
    is_staged: bool,
    is_unstaged: bool,
    is_untracked: bool,
) -> StatusEntry {
    StatusEntry {
        path: path.to_string(),
        is_staged,
        is_unstaged,
        is_untracked,
    }
}

/// Create a test BranchEntry
#[allow(dead_code)] // Used in tests
pub fn test_branch_entry(name: &str, is_head: bool, upstream: Option<&str>) -> BranchEntry {
    BranchEntry {
        name: name.to_string(),
        is_head,
        upstream: upstream.map(|s| s.to_string()),
    }
}

/// Create a test CommitEntry
#[allow(dead_code)]
pub fn test_commit_entry(short_id: &str, summary: &str, author: &str) -> CommitEntry {
    CommitEntry {
        short_id: short_id.to_string(),
        id: format!("{}0000000000000000000000000000000000000", short_id),
        summary: summary.to_string(),
        body: None,
        author: author.to_string(),
        timestamp: 1234567890,
        ..Default::default()
    }
}

/// Create a test StashEntry
#[allow(dead_code)] // Used in tests
pub fn test_stash_entry(index: usize, id: &str, message: &str) -> StashEntry {
    StashEntry {
        index,
        id: id.to_string(),
        message: message.to_string(),
    }
}
