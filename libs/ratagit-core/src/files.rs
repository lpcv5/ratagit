use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use crate::scroll::{ScrollDirection, move_selected_index_with_scroll, reset_scroll_origin};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub path: String,
    pub staged: bool,
    pub untracked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiffTarget {
    pub path: String,
    pub untracked: bool,
    pub is_directory_marker: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitFileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitFileEntry {
    pub path: String,
    pub old_path: Option<String>,
    pub status: CommitFileStatus,
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
    pub commit_status: Option<CommitFileStatus>,
    pub selected_for_batch: bool,
    pub matched: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileInputMode {
    Normal,
    MultiSelect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesPanelState {
    pub items: Vec<FileEntry>,
    pub selected: usize,
    pub expanded_dirs: BTreeSet<String>,
    pub selected_rows: BTreeSet<String>,
    pub selection_anchor: Option<String>,
    pub mode: FileInputMode,
    pub tree_initialized: bool,
    pub scroll_direction: Option<ScrollDirection>,
    pub scroll_direction_origin: usize,
    pub tree_rows: Vec<FileTreeRow>,
    pub row_descendants: BTreeMap<String, Vec<String>>,
    pub row_index_by_path: BTreeMap<String, usize>,
    pub lightweight_tree_projection: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommitFilesPanelState {
    pub active: bool,
    pub commit_id: Option<String>,
    pub items: Vec<CommitFileEntry>,
    pub selected: usize,
    pub expanded_dirs: BTreeSet<String>,
    pub loading: bool,
    pub scroll_direction: Option<ScrollDirection>,
    pub scroll_direction_origin: usize,
    pub tree_rows: Vec<FileTreeRow>,
    pub row_descendants: BTreeMap<String, Vec<String>>,
    pub row_index_by_path: BTreeMap<String, usize>,
    pub item_index_by_path: BTreeMap<String, usize>,
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
            tree_initialized: false,
            scroll_direction: None,
            scroll_direction_origin: 0,
            tree_rows: Vec::new(),
            row_descendants: BTreeMap::new(),
            row_index_by_path: BTreeMap::new(),
            lightweight_tree_projection: false,
        }
    }
}

pub fn initialize_tree_if_needed(state: &mut FilesPanelState) {
    initialize_tree_with_initial_expansion(state, true);
}

pub fn initialize_tree_with_initial_expansion(state: &mut FilesPanelState, expand_all_dirs: bool) {
    if state.tree_initialized {
        return;
    }
    state.lightweight_tree_projection = !expand_all_dirs;
    state.expanded_dirs = if expand_all_dirs {
        collect_directories(&state.items)
    } else {
        BTreeSet::new()
    };
    state.tree_initialized = true;
    refresh_tree_projection(state);
}

pub fn initialize_commit_files_tree(state: &mut CommitFilesPanelState) {
    state.expanded_dirs = collect_directories_from_paths(state.items.iter().map(|item| &item.path));
    state.item_index_by_path = state
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| (item.path.clone(), index))
        .collect();
    refresh_commit_files_tree_projection(state);
}

pub fn reconcile_after_items_changed(state: &mut FilesPanelState) {
    refresh_tree_projection(state);
    let valid_rows = state
        .tree_rows
        .iter()
        .map(|row| row.path.clone())
        .collect::<BTreeSet<_>>();
    state.selected_rows.retain(|path| valid_rows.contains(path));
    refresh_tree_projection(state);
    clamp_selected(state);
    if state.mode == FileInputMode::MultiSelect {
        ensure_valid_selection_anchor(state);
        refresh_multi_select_range(state);
    }
}

pub fn clamp_selected(state: &mut FilesPanelState) {
    let len = file_tree_row_len(state);
    state.selected = if len == 0 {
        0
    } else {
        state.selected.min(len - 1)
    };
    reset_scroll_origin(
        state.selected,
        len,
        &mut state.scroll_direction,
        &mut state.scroll_direction_origin,
    );
}

pub fn move_selected(state: &mut FilesPanelState, move_up: bool) {
    let len = file_tree_row_len(state);
    move_selected_index_with_scroll(
        &mut state.selected,
        len,
        move_up,
        &mut state.scroll_direction,
        &mut state.scroll_direction_origin,
    );
    if state.mode == FileInputMode::MultiSelect {
        refresh_multi_select_range(state);
    }
}

pub fn move_commit_file_selected(state: &mut CommitFilesPanelState, move_up: bool) {
    let len = commit_file_tree_row_len(state);
    move_selected_index_with_scroll(
        &mut state.selected,
        len,
        move_up,
        &mut state.scroll_direction,
        &mut state.scroll_direction_origin,
    );
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
    refresh_tree_projection(state);
    clamp_selected(state);
    if state.mode == FileInputMode::MultiSelect {
        ensure_valid_selection_anchor(state);
        refresh_multi_select_range(state);
    }
    true
}

pub fn toggle_commit_files_directory(state: &mut CommitFilesPanelState) -> bool {
    let Some(row) = selected_commit_file_row(state) else {
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
    refresh_commit_files_tree_projection(state);
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
    refresh_tree_projection(state);
}

pub fn selected_target_paths(state: &FilesPanelState) -> Vec<String> {
    selected_diff_targets(state)
        .into_iter()
        .map(|target| target.path)
        .collect()
}

pub fn selected_diff_targets(state: &FilesPanelState) -> Vec<FileDiffTarget> {
    let keys = if state.mode == FileInputMode::MultiSelect && !state.selected_rows.is_empty() {
        state.selected_rows.iter().cloned().collect::<Vec<_>>()
    } else {
        selected_row(state)
            .map(|row| vec![row.path])
            .unwrap_or_default()
    };
    resolve_row_keys_to_diff_targets(state, &keys)
}

pub fn selected_row(state: &FilesPanelState) -> Option<FileTreeRow> {
    if !state.tree_rows.is_empty() || state.items.is_empty() {
        return state.tree_rows.get(state.selected).cloned();
    }
    compute_tree_projection(state)
        .rows
        .into_iter()
        .nth(state.selected)
}

pub fn selected_commit_file(state: &CommitFilesPanelState) -> Option<CommitFileEntry> {
    let row = selected_commit_file_row(state)?;
    if row.kind != FileRowKind::File {
        return None;
    }
    if let Some(index) = state.item_index_by_path.get(&row.path) {
        return state.items.get(*index).cloned();
    }
    state
        .items
        .iter()
        .find(|item| item.path == row.path)
        .cloned()
}

pub fn selected_commit_file_targets(state: &CommitFilesPanelState) -> Vec<CommitFileEntry> {
    let Some(row) = selected_commit_file_row(state) else {
        return Vec::new();
    };
    if row.kind == FileRowKind::File {
        return selected_commit_file(state).into_iter().collect();
    }
    vec![CommitFileEntry {
        path: row.path,
        old_path: None,
        status: CommitFileStatus::Unknown,
    }]
}

pub fn select_file_tree_path(state: &mut FilesPanelState, path: &str) -> bool {
    expand_ancestors(&mut state.expanded_dirs, path);
    refresh_tree_projection(state);
    if let Some(index) = state.row_index_by_path.get(path).copied() {
        state.selected = index;
        return true;
    }
    false
}

pub fn select_commit_file_tree_path(state: &mut CommitFilesPanelState, path: &str) -> bool {
    expand_ancestors(&mut state.expanded_dirs, path);
    refresh_commit_files_tree_projection(state);
    if let Some(index) = state.row_index_by_path.get(path).copied() {
        state.selected = index;
        return true;
    }
    false
}

fn selected_commit_file_row_ref(state: &CommitFilesPanelState) -> Option<&FileTreeRow> {
    state.tree_rows.get(state.selected)
}

fn selected_commit_file_row(state: &CommitFilesPanelState) -> Option<FileTreeRow> {
    if let Some(row) = selected_commit_file_row_ref(state) {
        return Some(row.clone());
    }
    if state.items.is_empty() {
        return None;
    }
    compute_commit_files_tree_projection(state)
        .rows
        .into_iter()
        .nth(state.selected)
}

pub fn build_file_tree_rows(state: &FilesPanelState) -> Vec<FileTreeRow> {
    if !state.tree_rows.is_empty() || state.items.is_empty() {
        return state.tree_rows.clone();
    }
    compute_tree_projection(state).rows
}

pub fn build_commit_file_tree_rows(state: &CommitFilesPanelState) -> Vec<FileTreeRow> {
    if !state.tree_rows.is_empty() || state.items.is_empty() {
        return state.tree_rows.clone();
    }
    compute_commit_files_tree_projection(state).rows
}

pub fn file_tree_rows_for_read(state: &FilesPanelState) -> Cow<'_, [FileTreeRow]> {
    if !state.tree_rows.is_empty() || state.items.is_empty() {
        Cow::Borrowed(&state.tree_rows)
    } else {
        Cow::Owned(compute_tree_projection(state).rows)
    }
}

pub fn commit_file_tree_rows_for_read(state: &CommitFilesPanelState) -> Cow<'_, [FileTreeRow]> {
    if !state.tree_rows.is_empty() || state.items.is_empty() {
        Cow::Borrowed(&state.tree_rows)
    } else {
        Cow::Owned(compute_commit_files_tree_projection(state).rows)
    }
}

pub fn file_tree_rows(state: &FilesPanelState) -> &[FileTreeRow] {
    &state.tree_rows
}

pub fn commit_file_tree_rows(state: &CommitFilesPanelState) -> &[FileTreeRow] {
    &state.tree_rows
}

pub fn refresh_tree_projection(state: &mut FilesPanelState) {
    let projection = compute_tree_projection(state);
    state.tree_rows = projection.rows;
    state.row_descendants = projection.row_descendants;
    state.row_index_by_path = projection.row_index_by_path;
}

pub fn refresh_commit_files_tree_projection(state: &mut CommitFilesPanelState) {
    let projection = compute_commit_files_tree_projection(state);
    state.tree_rows = projection.rows;
    state.row_descendants = projection.row_descendants;
    state.row_index_by_path = projection.row_index_by_path;
    clamp_commit_file_selected(state);
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TreeProjection {
    rows: Vec<FileTreeRow>,
    row_descendants: BTreeMap<String, Vec<String>>,
    row_index_by_path: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TreeRowContext {
    path: String,
    name: String,
    depth: usize,
    kind: FileRowKind,
    expanded: bool,
    descendants: Vec<String>,
}

fn compute_tree_projection(state: &FilesPanelState) -> TreeProjection {
    if state.lightweight_tree_projection {
        return compute_lightweight_tree_projection(state);
    }
    let entry_status = state
        .items
        .iter()
        .map(|entry| (entry.path.clone(), (entry.staged, entry.untracked)))
        .collect::<BTreeMap<_, _>>();
    compute_tree_projection_from_paths(
        state.items.iter().map(|entry| entry.path.clone()),
        &state.expanded_dirs,
        |context| {
            let staged = !context.descendants.is_empty()
                && context
                    .descendants
                    .iter()
                    .all(|path| entry_status.get(path).is_some_and(|status| status.0));
            let untracked = !context.descendants.is_empty()
                && context
                    .descendants
                    .iter()
                    .all(|path| entry_status.get(path).is_some_and(|status| status.1));
            FileTreeRow {
                depth: context.depth,
                expanded: context.expanded,
                selected_for_batch: state.selected_rows.contains(&context.path),
                path: context.path,
                name: context.name,
                kind: context.kind,
                staged,
                untracked,
                commit_status: None,
                matched: false,
            }
        },
    )
}

fn compute_lightweight_tree_projection(state: &FilesPanelState) -> TreeProjection {
    let entry_status = state
        .items
        .iter()
        .map(|entry| (entry.path.clone(), (entry.staged, entry.untracked)))
        .collect::<BTreeMap<_, _>>();
    let mut keys = Vec::new();
    for entry in &state.items {
        let normalized = normalize_tree_path(&entry.path);
        if normalized.is_empty() {
            continue;
        }
        let parts = normalized.split('/').collect::<Vec<_>>();
        for index in 0..parts.len() {
            let path = parts[..=index].join("/");
            let kind = if index + 1 < parts.len() || is_directory_marker(&entry.path) {
                FileRowKind::Directory
            } else {
                FileRowKind::File
            };
            if row_is_visible(&path, &state.expanded_dirs) {
                keys.push((path, kind));
            }
        }
    }
    keys.sort_by(compare_tree_keys);
    keys.dedup();

    let rows = keys
        .into_iter()
        .map(|(path, kind)| {
            let status = entry_status
                .get(&path)
                .copied()
                .or_else(|| entry_status.get(&format!("{path}/")).copied())
                .unwrap_or_default();
            FileTreeRow {
                depth: path_depth(&path),
                expanded: state.expanded_dirs.contains(&path),
                selected_for_batch: state.selected_rows.contains(&path),
                name: tree_row_name(&path),
                path,
                kind,
                staged: status.0,
                untracked: status.1,
                commit_status: None,
                matched: false,
            }
        })
        .collect::<Vec<_>>();
    let row_index_by_path = rows
        .iter()
        .enumerate()
        .map(|(index, row)| (row.path.clone(), index))
        .collect::<BTreeMap<_, _>>();
    TreeProjection {
        rows,
        row_descendants: BTreeMap::new(),
        row_index_by_path,
    }
}

fn compute_commit_files_tree_projection(state: &CommitFilesPanelState) -> TreeProjection {
    let entry_status = state
        .items
        .iter()
        .map(|entry| (entry.path.clone(), entry.status))
        .collect::<BTreeMap<_, _>>();
    compute_tree_projection_from_paths(
        state.items.iter().map(|entry| entry.path.clone()),
        &state.expanded_dirs,
        |context| {
            let commit_status = if context.kind == FileRowKind::File {
                entry_status.get(&context.path).copied()
            } else {
                None
            };
            FileTreeRow {
                depth: context.depth,
                expanded: context.expanded,
                selected_for_batch: false,
                path: context.path,
                name: context.name,
                kind: context.kind,
                staged: false,
                untracked: false,
                commit_status,
                matched: false,
            }
        },
    )
}

fn compute_tree_projection_from_paths(
    paths: impl IntoIterator<Item = String>,
    expanded_dirs: &BTreeSet<String>,
    mut build_row: impl FnMut(TreeRowContext) -> FileTreeRow,
) -> TreeProjection {
    let paths = paths.into_iter().collect::<Vec<_>>();
    let dirs = collect_directories_from_paths(paths.iter());
    let descendants = collect_row_descendants_from_paths(paths.iter());
    let mut keys = dirs
        .iter()
        .map(|path| (path.clone(), FileRowKind::Directory))
        .chain(
            paths
                .iter()
                .filter(|path| !is_directory_marker(path))
                .map(|path| (path.clone(), FileRowKind::File)),
        )
        .collect::<Vec<_>>();
    keys.sort_by(compare_tree_keys);

    let rows = keys
        .into_iter()
        .filter(|(path, _)| row_is_visible(path, expanded_dirs))
        .map(|(path, kind)| {
            let context = TreeRowContext {
                name: tree_row_name(&path),
                depth: path_depth(&path),
                expanded: expanded_dirs.contains(&path),
                descendants: descendants
                    .get(&path)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .collect(),
                path,
                kind,
            };
            build_row(context)
        })
        .collect::<Vec<_>>();
    let row_index_by_path = rows
        .iter()
        .enumerate()
        .map(|(index, row)| (row.path.clone(), index))
        .collect::<BTreeMap<_, _>>();
    TreeProjection {
        rows,
        row_descendants: descendants
            .into_iter()
            .map(|(path, descendants)| (path, descendants.into_iter().collect()))
            .collect(),
        row_index_by_path,
    }
}

fn tree_row_name(path: &str) -> String {
    path.rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_string()
}

pub fn collect_directories(items: &[FileEntry]) -> BTreeSet<String> {
    collect_directories_from_paths(items.iter().map(|item| &item.path))
}

fn collect_directories_from_paths<'a>(paths: impl Iterator<Item = &'a String>) -> BTreeSet<String> {
    let mut dirs = BTreeSet::new();
    for path in paths {
        let normalized = normalize_tree_path(path);
        if normalized.is_empty() {
            continue;
        }
        if is_directory_marker(path) {
            dirs.insert(normalized.clone());
        }
        let mut parts = normalized.split('/').collect::<Vec<_>>();
        while parts.len() > 1 {
            parts.pop();
            dirs.insert(parts.join("/"));
        }
    }
    dirs
}

fn collect_row_descendants_from_paths<'a>(
    paths: impl Iterator<Item = &'a String>,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut descendants = BTreeMap::<String, BTreeSet<String>>::new();
    for path in paths {
        let normalized = normalize_tree_path(path);
        if normalized.is_empty() {
            continue;
        }
        if is_directory_marker(path) {
            descendants
                .entry(normalized.clone())
                .or_default()
                .insert(path.clone());
        } else {
            descendants
                .entry(path.clone())
                .or_default()
                .insert(path.clone());
        }

        let mut parts = normalized.split('/').collect::<Vec<_>>();
        while parts.len() > 1 {
            parts.pop();
            descendants
                .entry(parts.join("/"))
                .or_default()
                .insert(path.clone());
        }
    }
    descendants
}

fn clamp_commit_file_selected(state: &mut CommitFilesPanelState) {
    let len = commit_file_tree_row_len(state);
    state.selected = if len == 0 {
        0
    } else {
        state.selected.min(len - 1)
    };
    reset_scroll_origin(
        state.selected,
        len,
        &mut state.scroll_direction,
        &mut state.scroll_direction_origin,
    );
}

fn file_tree_row_len(state: &FilesPanelState) -> usize {
    if !state.tree_rows.is_empty() || state.items.is_empty() {
        state.tree_rows.len()
    } else {
        compute_tree_projection(state).rows.len()
    }
}

fn commit_file_tree_row_len(state: &CommitFilesPanelState) -> usize {
    if !state.tree_rows.is_empty() || state.items.is_empty() {
        state.tree_rows.len()
    } else {
        compute_commit_files_tree_projection(state).rows.len()
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
        refresh_tree_projection(state);
        return;
    };
    let Some(anchor_index) = rows.iter().position(|row| row.path == anchor_path) else {
        refresh_tree_projection(state);
        return;
    };
    if rows.is_empty() {
        refresh_tree_projection(state);
        return;
    }
    let selected_index = state.selected.min(rows.len() - 1);
    let start = anchor_index.min(selected_index);
    let end = anchor_index.max(selected_index);
    for row in &rows[start..=end] {
        state.selected_rows.insert(row.path.clone());
    }
    refresh_tree_projection(state);
}

fn expand_ancestors(expanded_dirs: &mut BTreeSet<String>, path: &str) {
    let mut parts = path.split('/').collect::<Vec<_>>();
    while parts.len() > 1 {
        parts.pop();
        expanded_dirs.insert(parts.join("/"));
    }
}

fn resolve_row_keys_to_diff_targets(
    state: &FilesPanelState,
    keys: &[String],
) -> Vec<FileDiffTarget> {
    let descendants = if state.row_descendants.is_empty() && !state.items.is_empty() {
        compute_tree_projection(state).row_descendants
    } else {
        state.row_descendants.clone()
    };
    let entries = state
        .items
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let mut paths = BTreeSet::new();
    for key in keys {
        if let Some(row_descendants) = descendants.get(key) {
            paths.extend(row_descendants.iter().cloned());
        } else {
            paths.extend(paths_for_row_key(state, key));
        }
    }
    paths
        .into_iter()
        .map(|path| {
            let entry = entries.get(path.as_str()).copied();
            FileDiffTarget {
                is_directory_marker: is_directory_marker(&path),
                untracked: entry.is_some_and(|entry| entry.untracked),
                path,
            }
        })
        .collect()
}

fn is_directory_marker(path: &str) -> bool {
    path.ends_with('/')
}

fn paths_for_row_key(state: &FilesPanelState, key: &str) -> BTreeSet<String> {
    let normalized_key = normalize_tree_path(key);
    if normalized_key.is_empty() {
        return BTreeSet::new();
    }
    let prefix = format!("{normalized_key}/");
    state
        .items
        .iter()
        .filter(|entry| {
            normalize_tree_path(&entry.path) == normalized_key || entry.path.starts_with(&prefix)
        })
        .map(|entry| entry.path.clone())
        .collect()
}

fn normalize_tree_path(path: &str) -> String {
    path.trim_end_matches('/').to_string()
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
    fn select_file_tree_path_expands_ancestors() {
        let mut state = files();
        state.expanded_dirs.remove("src");
        refresh_tree_projection(&mut state);

        assert!(select_file_tree_path(&mut state, "src/main.rs"));
        assert_eq!(
            selected_row(&state).map(|row| row.path),
            Some("src/main.rs".to_string())
        );
        assert!(state.expanded_dirs.contains("src"));
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

    #[test]
    fn untracked_directory_marker_renders_as_directory_node() {
        let mut state = FilesPanelState {
            items: vec![FileEntry {
                path: "libs/ratagit-git/tests/".to_string(),
                staged: false,
                untracked: true,
            }],
            ..FilesPanelState::default()
        };
        initialize_tree_if_needed(&mut state);

        let rows = build_file_tree_rows(&state);
        let tests_row = rows
            .iter()
            .find(|row| row.path == "libs/ratagit-git/tests")
            .expect("tests directory row should exist");
        assert_eq!(tests_row.kind, FileRowKind::Directory);
        assert_eq!(tests_row.name, "tests");
        assert!(tests_row.untracked);
        assert!(
            !rows
                .iter()
                .any(|row| row.path == "libs/ratagit-git/tests/" && row.kind == FileRowKind::File)
        );
    }
}
