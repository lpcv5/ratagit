use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub path: String,
    pub staged: bool,
    pub untracked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileRowKind {
    Directory,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTreeRow {
    pub path: String,
    pub name: String,
    pub depth: usize,
    pub kind: FileRowKind,
    pub expanded: bool,
    pub staged: bool,
    pub untracked: bool,
    pub selected_for_batch: bool,
    pub matched: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileInputMode {
    Normal,
    MultiSelect,
    SearchInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesPanelState {
    pub items: Vec<FileEntry>,
    pub selected: usize,
    pub expanded_dirs: BTreeSet<String>,
    pub selected_rows: BTreeSet<String>,
    pub selection_anchor: Option<String>,
    pub mode: FileInputMode,
    pub search_query: String,
    pub search_matches: Vec<String>,
    pub current_match: Option<usize>,
    pub tree_initialized: bool,
    pub scroll_direction: Option<ScrollDirection>,
    pub scroll_direction_origin: usize,
}

impl Default for FilesPanelState {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            selected: 0,
            expanded_dirs: BTreeSet::new(),
            selected_rows: BTreeSet::new(),
            selection_anchor: None,
            mode: FileInputMode::Normal,
            search_query: String::new(),
            search_matches: Vec::new(),
            current_match: None,
            tree_initialized: false,
            scroll_direction: None,
            scroll_direction_origin: 0,
        }
    }
}

pub fn initialize_tree_if_needed(state: &mut FilesPanelState) {
    if state.tree_initialized {
        return;
    }
    state.expanded_dirs = collect_directories(&state.items);
    state.tree_initialized = true;
}

pub fn reconcile_after_items_changed(state: &mut FilesPanelState) {
    let valid_rows = build_file_tree_rows(state)
        .into_iter()
        .map(|row| row.path)
        .collect::<BTreeSet<_>>();
    state.selected_rows.retain(|path| valid_rows.contains(path));
    recompute_search_matches(state);
    clamp_selected(state);
    if state.mode == FileInputMode::MultiSelect {
        ensure_valid_selection_anchor(state);
        refresh_multi_select_range(state);
    }
}

pub fn clamp_selected(state: &mut FilesPanelState) {
    let len = build_file_tree_rows(state).len();
    state.selected = if len == 0 {
        0
    } else {
        state.selected.min(len - 1)
    };
    state.scroll_direction_origin = state.selected;
}

pub fn move_selected(state: &mut FilesPanelState, move_up: bool) {
    let len = build_file_tree_rows(state).len();
    if len == 0 {
        state.selected = 0;
        state.scroll_direction = None;
        state.scroll_direction_origin = 0;
        return;
    }
    let old_selected = state.selected;
    let next_direction = if move_up {
        ScrollDirection::Up
    } else {
        ScrollDirection::Down
    };
    if move_up {
        state.selected = state.selected.saturating_sub(1);
    } else {
        state.selected = (state.selected + 1).min(len - 1);
    }
    if state.selected != old_selected && state.scroll_direction != Some(next_direction) {
        state.scroll_direction = Some(next_direction);
        state.scroll_direction_origin = old_selected;
    }
    if state.mode == FileInputMode::MultiSelect {
        refresh_multi_select_range(state);
    }
}

pub fn toggle_selected_directory(state: &mut FilesPanelState) -> bool {
    let Some(row) = selected_row(state) else {
        return false;
    };
    if row.kind != FileRowKind::Directory {
        return false;
    }
    if state.expanded_dirs.contains(&row.path) {
        state.expanded_dirs.remove(&row.path);
    } else {
        state.expanded_dirs.insert(row.path);
    }
    clamp_selected(state);
    if state.mode == FileInputMode::MultiSelect {
        ensure_valid_selection_anchor(state);
        refresh_multi_select_range(state);
    }
    true
}

pub fn enter_multi_select(state: &mut FilesPanelState) {
    state.mode = FileInputMode::MultiSelect;
    state.selection_anchor = selected_row(state).map(|row| row.path);
    refresh_multi_select_range(state);
}

pub fn toggle_current_row_selection(state: &mut FilesPanelState) {
    if state.mode == FileInputMode::MultiSelect {
        leave_multi_select(state);
    } else {
        enter_multi_select(state);
    }
}

pub fn leave_multi_select(state: &mut FilesPanelState) {
    state.mode = FileInputMode::Normal;
    state.selection_anchor = None;
    state.selected_rows.clear();
}

pub fn start_search(state: &mut FilesPanelState) {
    if state.mode == FileInputMode::MultiSelect {
        leave_multi_select(state);
    }
    state.mode = FileInputMode::SearchInput;
    state.search_query.clear();
    state.search_matches.clear();
    state.current_match = None;
}

pub fn push_search_char(state: &mut FilesPanelState, ch: char) {
    state.search_query.push(ch);
}

pub fn pop_search_char(state: &mut FilesPanelState) {
    state.search_query.pop();
}

pub fn confirm_search(state: &mut FilesPanelState) {
    state.mode = FileInputMode::Normal;
    recompute_search_matches(state);
    if !state.search_matches.is_empty() {
        state.current_match = Some(0);
        select_match(state);
    }
}

pub fn cancel_search(state: &mut FilesPanelState) {
    if state.mode == FileInputMode::SearchInput || !state.search_query.is_empty() {
        state.mode = FileInputMode::Normal;
        state.search_query.clear();
        state.search_matches.clear();
        state.current_match = None;
    }
}

pub fn jump_search_match(state: &mut FilesPanelState, previous: bool) {
    if state.search_matches.is_empty() {
        return;
    }
    let len = state.search_matches.len();
    let next = match (state.current_match, previous) {
        (Some(index), true) => (index + len - 1) % len,
        (Some(index), false) => (index + 1) % len,
        (None, _) => 0,
    };
    state.current_match = Some(next);
    select_match(state);
}

pub fn selected_target_paths(state: &FilesPanelState) -> Vec<String> {
    let keys = if state.mode == FileInputMode::MultiSelect && !state.selected_rows.is_empty() {
        state.selected_rows.iter().cloned().collect::<Vec<_>>()
    } else {
        selected_row(state)
            .map(|row| vec![row.path])
            .unwrap_or_default()
    };
    resolve_row_keys_to_files(state, &keys)
}

pub fn selected_row(state: &FilesPanelState) -> Option<FileTreeRow> {
    build_file_tree_rows(state).into_iter().nth(state.selected)
}

pub fn build_file_tree_rows(state: &FilesPanelState) -> Vec<FileTreeRow> {
    let dirs = collect_directories(&state.items);
    let mut keys = dirs
        .iter()
        .map(|path| (path.clone(), FileRowKind::Directory))
        .chain(
            state
                .items
                .iter()
                .map(|entry| (entry.path.clone(), FileRowKind::File)),
        )
        .collect::<Vec<_>>();
    keys.sort_by(compare_tree_keys);

    let query = state.search_query.to_lowercase();
    keys.into_iter()
        .filter(|(path, _)| row_is_visible(path, &state.expanded_dirs))
        .map(|(path, kind)| {
            let descendants = descendants_for_key(&state.items, &path, kind);
            let staged = !descendants.is_empty() && descendants.iter().all(|entry| entry.staged);
            let untracked =
                !descendants.is_empty() && descendants.iter().all(|entry| entry.untracked);
            let name = path
                .rsplit('/')
                .next()
                .filter(|name| !name.is_empty())
                .unwrap_or(&path)
                .to_string();
            let matched = !query.is_empty() && path.to_lowercase().contains(&query);
            FileTreeRow {
                depth: path_depth(&path),
                expanded: state.expanded_dirs.contains(&path),
                selected_for_batch: state.selected_rows.contains(&path),
                path,
                name,
                kind,
                staged,
                untracked,
                matched,
            }
        })
        .collect()
}

pub fn collect_directories(items: &[FileEntry]) -> BTreeSet<String> {
    let mut dirs = BTreeSet::new();
    for item in items {
        let mut parts = item.path.split('/').collect::<Vec<_>>();
        while parts.len() > 1 {
            parts.pop();
            dirs.insert(parts.join("/"));
        }
    }
    dirs
}

fn recompute_search_matches(state: &mut FilesPanelState) {
    if state.search_query.is_empty() {
        state.search_matches.clear();
        state.current_match = None;
        return;
    }
    let query = state.search_query.to_lowercase();
    state.search_matches = build_file_tree_rows(state)
        .into_iter()
        .filter(|row| row.path.to_lowercase().contains(&query))
        .map(|row| row.path)
        .collect();
    if state.search_matches.is_empty() {
        state.current_match = None;
    } else {
        state.current_match = Some(
            state
                .current_match
                .unwrap_or(0)
                .min(state.search_matches.len() - 1),
        );
    }
}

fn ensure_valid_selection_anchor(state: &mut FilesPanelState) {
    let rows = build_file_tree_rows(state);
    let anchor_is_valid = state
        .selection_anchor
        .as_ref()
        .is_some_and(|anchor| rows.iter().any(|row| &row.path == anchor));
    if !anchor_is_valid {
        state.selection_anchor = rows.get(state.selected).map(|row| row.path.clone());
    }
}

fn refresh_multi_select_range(state: &mut FilesPanelState) {
    let rows = build_file_tree_rows(state);
    state.selected_rows.clear();
    let Some(anchor_path) = state.selection_anchor.clone() else {
        return;
    };
    let Some(anchor_index) = rows.iter().position(|row| row.path == anchor_path) else {
        return;
    };
    if rows.is_empty() {
        return;
    }
    let selected_index = state.selected.min(rows.len() - 1);
    let start = anchor_index.min(selected_index);
    let end = anchor_index.max(selected_index);
    for row in &rows[start..=end] {
        state.selected_rows.insert(row.path.clone());
    }
}

fn select_match(state: &mut FilesPanelState) {
    let Some(match_index) = state.current_match else {
        return;
    };
    let Some(path) = state.search_matches.get(match_index).cloned() else {
        return;
    };
    expand_ancestors(state, &path);
    if let Some(index) = build_file_tree_rows(state)
        .iter()
        .position(|row| row.path == path)
    {
        state.selected = index;
    }
}

fn expand_ancestors(state: &mut FilesPanelState, path: &str) {
    let mut parts = path.split('/').collect::<Vec<_>>();
    while parts.len() > 1 {
        parts.pop();
        state.expanded_dirs.insert(parts.join("/"));
    }
}

fn resolve_row_keys_to_files(state: &FilesPanelState, keys: &[String]) -> Vec<String> {
    let rows = build_file_tree_rows(state);
    let mut paths = BTreeSet::new();
    for key in keys {
        let Some(row) = rows.iter().find(|row| &row.path == key) else {
            continue;
        };
        for entry in descendants_for_key(&state.items, &row.path, row.kind) {
            paths.insert(entry.path.clone());
        }
    }
    paths.into_iter().collect()
}

fn descendants_for_key<'a>(
    items: &'a [FileEntry],
    key: &str,
    kind: FileRowKind,
) -> Vec<&'a FileEntry> {
    match kind {
        FileRowKind::File => items.iter().filter(|entry| entry.path == key).collect(),
        FileRowKind::Directory => {
            let prefix = format!("{key}/");
            items
                .iter()
                .filter(|entry| entry.path.starts_with(&prefix))
                .collect()
        }
    }
}

fn row_is_visible(path: &str, expanded_dirs: &BTreeSet<String>) -> bool {
    let mut parts = path.split('/').collect::<Vec<_>>();
    while parts.len() > 1 {
        parts.pop();
        if !expanded_dirs.contains(&parts.join("/")) {
            return false;
        }
    }
    true
}

fn compare_tree_keys(
    left: &(String, FileRowKind),
    right: &(String, FileRowKind),
) -> std::cmp::Ordering {
    let left_parts = left.0.split('/').collect::<Vec<_>>();
    let right_parts = right.0.split('/').collect::<Vec<_>>();
    for (left_part, right_part) in left_parts.iter().zip(right_parts.iter()) {
        let ordering = left_part.cmp(right_part);
        if ordering != std::cmp::Ordering::Equal {
            return ordering;
        }
    }
    left_parts
        .len()
        .cmp(&right_parts.len())
        .then_with(|| match (left.1, right.1) {
            (FileRowKind::Directory, FileRowKind::File) => std::cmp::Ordering::Less,
            (FileRowKind::File, FileRowKind::Directory) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        })
}

fn path_depth(path: &str) -> usize {
    path.split('/').count().saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> FilesPanelState {
        let mut state = FilesPanelState {
            items: vec![
                FileEntry {
                    path: "src/main.rs".to_string(),
                    staged: true,
                    untracked: false,
                },
                FileEntry {
                    path: "src/ui/list.rs".to_string(),
                    staged: false,
                    untracked: false,
                },
                FileEntry {
                    path: "README.md".to_string(),
                    staged: false,
                    untracked: true,
                },
            ],
            ..FilesPanelState::default()
        };
        initialize_tree_if_needed(&mut state);
        state
    }

    #[test]
    fn tree_rows_include_directories_and_files() {
        let state = files();
        let rows = build_file_tree_rows(&state);
        let paths = rows.into_iter().map(|row| row.path).collect::<Vec<_>>();
        assert_eq!(
            paths,
            vec![
                "README.md",
                "src",
                "src/main.rs",
                "src/ui",
                "src/ui/list.rs"
            ]
        );
    }

    #[test]
    fn selected_directory_resolves_to_visible_snapshot_descendants() {
        let mut state = files();
        state.selected = build_file_tree_rows(&state)
            .iter()
            .position(|row| row.path == "src")
            .expect("src row exists");
        assert_eq!(
            selected_target_paths(&state),
            vec!["src/main.rs".to_string(), "src/ui/list.rs".to_string()]
        );
    }

    #[test]
    fn search_confirm_selects_first_match_and_navigates() {
        let mut state = files();
        start_search(&mut state);
        for ch in "src".chars() {
            push_search_char(&mut state, ch);
        }
        confirm_search(&mut state);
        assert_eq!(
            selected_row(&state).map(|row| row.path),
            Some("src".to_string())
        );
        jump_search_match(&mut state, false);
        assert_eq!(
            selected_row(&state).map(|row| row.path),
            Some("src/main.rs".to_string())
        );
    }

    #[test]
    fn multi_select_resolves_unique_file_targets() {
        let mut state = files();
        state.selected_rows.insert("src".to_string());
        state.selected_rows.insert("src/main.rs".to_string());
        state.mode = FileInputMode::MultiSelect;
        assert_eq!(
            selected_target_paths(&state),
            vec!["src/main.rs".to_string(), "src/ui/list.rs".to_string()]
        );
    }
}
