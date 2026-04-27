use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use crate::scroll::{move_selected_index, move_selected_index_with_scroll_offset};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileInputMode {
    #[default]
    Normal,
    MultiSelect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesUiState {
    pub selected: usize,
    pub scroll_offset: usize,
    pub expanded_dirs: BTreeSet<String>,
    pub selected_rows: BTreeSet<String>,
    pub selection_anchor: Option<String>,
    pub mode: FileInputMode,
    pub tree_initialized: bool,
    pub tree_rows: Vec<FileTreeRow>,
    pub row_descendants: BTreeMap<String, Vec<String>>,
    pub row_index_by_path: BTreeMap<String, usize>,
    pub lightweight_tree_projection: bool,
    pub tree_index: FileTreeIndex,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommitFilesUiState {
    pub active: bool,
    pub commit_id: Option<String>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub expanded_dirs: BTreeSet<String>,
    pub selected_rows: BTreeSet<String>,
    pub selection_anchor: Option<String>,
    pub mode: FileInputMode,
    pub tree_rows: Vec<FileTreeRow>,
    pub row_descendants: BTreeMap<String, Vec<String>>,
    pub row_index_by_path: BTreeMap<String, usize>,
    pub item_index_by_path: BTreeMap<String, usize>,
    pub tree_index: FileTreeIndex,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileTreeIndex {
    root_paths: BTreeSet<String>,
    nodes: BTreeMap<String, FileTreeNode>,
    sources: BTreeMap<String, FileTreeSource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileTreeNode {
    kind: FileRowKind,
    staged: bool,
    untracked: bool,
    commit_status: Option<CommitFileStatus>,
    children: BTreeSet<String>,
    ref_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileTreeSource {
    path: String,
    kind: FileRowKind,
    staged: bool,
    untracked: bool,
    commit_status: Option<CommitFileStatus>,
}

impl FileTreeIndex {
    fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    fn from_sources(sources: BTreeMap<String, FileTreeSource>) -> Self {
        let mut index = Self::default();
        index.sync(sources);
        index
    }

    fn sync(&mut self, next_sources: BTreeMap<String, FileTreeSource>) {
        let removed = self
            .sources
            .keys()
            .filter(|path| !next_sources.contains_key(*path))
            .cloned()
            .collect::<Vec<_>>();
        for path in removed {
            if let Some(source) = self.sources.remove(&path) {
                self.remove_source(&source);
            }
        }

        for (path, next_source) in next_sources {
            match self.sources.get(&path).cloned() {
                Some(current) if current == next_source => {}
                Some(current)
                    if current.path == next_source.path && current.kind == next_source.kind =>
                {
                    self.apply_source_metadata(&next_source);
                    self.sources.insert(path, next_source);
                }
                Some(current) => {
                    self.sources.remove(&path);
                    self.remove_source(&current);
                    self.add_source(&next_source);
                    self.sources.insert(path, next_source);
                }
                None => {
                    self.add_source(&next_source);
                    self.sources.insert(path, next_source);
                }
            }
        }
    }

    fn add_source(&mut self, source: &FileTreeSource) {
        let chain = source_path_chain(&source.path);
        for (index, path) in chain.iter().enumerate() {
            let kind = if index + 1 == chain.len() {
                source.kind
            } else {
                FileRowKind::Directory
            };
            let node = self
                .nodes
                .entry(path.clone())
                .or_insert_with(|| FileTreeNode {
                    kind,
                    staged: false,
                    untracked: false,
                    commit_status: None,
                    children: BTreeSet::new(),
                    ref_count: 0,
                });
            if kind == FileRowKind::Directory {
                node.kind = FileRowKind::Directory;
            }
            node.ref_count = node.ref_count.saturating_add(1);

            if index == 0 {
                self.root_paths.insert(path.clone());
            } else if let Some(parent) = self.nodes.get_mut(&chain[index - 1]) {
                parent.children.insert(path.clone());
            }
        }
        self.apply_source_metadata(source);
    }

    fn remove_source(&mut self, source: &FileTreeSource) {
        let chain = source_path_chain(&source.path);
        for index in (0..chain.len()).rev() {
            let path = &chain[index];
            let remove_node = if let Some(node) = self.nodes.get_mut(path) {
                node.ref_count = node.ref_count.saturating_sub(1);
                if index + 1 == chain.len() {
                    node.staged = false;
                    node.untracked = false;
                    node.commit_status = None;
                }
                node.ref_count == 0
            } else {
                false
            };
            if remove_node {
                self.nodes.remove(path);
                if index == 0 {
                    self.root_paths.remove(path);
                } else if let Some(parent) = self.nodes.get_mut(&chain[index - 1]) {
                    parent.children.remove(path);
                }
            }
        }
    }

    fn apply_source_metadata(&mut self, source: &FileTreeSource) {
        if let Some(node) = self.nodes.get_mut(&source.path) {
            node.kind = source.kind;
            node.staged = source.staged;
            node.untracked = source.untracked;
            node.commit_status = source.commit_status;
        }
    }
}

impl Default for FilesUiState {
    fn default() -> Self {
        Self {
            selected: 0,
            scroll_offset: 0,
            expanded_dirs: BTreeSet::new(),
            selected_rows: BTreeSet::new(),
            selection_anchor: None,
            mode: FileInputMode::Normal,
            tree_initialized: false,
            tree_rows: Vec::new(),
            row_descendants: BTreeMap::new(),
            row_index_by_path: BTreeMap::new(),
            lightweight_tree_projection: false,
            tree_index: FileTreeIndex::default(),
        }
    }
}

pub fn initialize_tree_if_needed(items: &[FileEntry], state: &mut FilesUiState) {
    initialize_tree_with_initial_expansion(items, state, true);
}

pub fn initialize_tree_with_initial_expansion(
    items: &[FileEntry],
    state: &mut FilesUiState,
    expand_all_dirs: bool,
) {
    if state.tree_initialized {
        return;
    }
    state.lightweight_tree_projection = !expand_all_dirs;
    state.tree_index = if expand_all_dirs {
        FileTreeIndex::default()
    } else {
        FileTreeIndex::from_sources(file_tree_sources(items))
    };
    state.expanded_dirs = if expand_all_dirs {
        collect_directories(items)
    } else {
        BTreeSet::new()
    };
    state.tree_initialized = true;
    refresh_tree_projection(items, state);
}

pub fn initialize_commit_files_tree(items: &[CommitFileEntry], state: &mut CommitFilesUiState) {
    state.expanded_dirs = collect_directories_from_paths(items.iter().map(|item| &item.path));
    state.item_index_by_path = items
        .iter()
        .enumerate()
        .map(|(index, item)| (item.path.clone(), index))
        .collect();
    state.tree_index.sync(commit_file_tree_sources(items));
    refresh_commit_files_tree_projection(items, state);
    clamp_commit_file_selected(items, state);
}

pub fn reconcile_after_items_changed(items: &[FileEntry], state: &mut FilesUiState) {
    sync_file_tree_index_if_needed(items, state);
    refresh_tree_projection(items, state);
    let valid_rows = state
        .tree_rows
        .iter()
        .map(|row| row.path.clone())
        .collect::<BTreeSet<_>>();
    state.selected_rows.retain(|path| valid_rows.contains(path));
    refresh_tree_projection(items, state);
    clamp_selected(items, state);
    if state.mode == FileInputMode::MultiSelect {
        ensure_valid_selection_anchor(items, state);
        refresh_multi_select_range(items, state);
    }
}

pub fn clamp_selected(items: &[FileEntry], state: &mut FilesUiState) {
    let len = file_tree_row_len(items, state);
    state.selected = if len == 0 {
        0
    } else {
        state.selected.min(len - 1)
    };
    state.scroll_offset = 0;
}

pub fn move_selected(state: &mut FilesUiState, move_up: bool) {
    let len = state.tree_rows.len();
    move_selected_index(&mut state.selected, len, move_up);
    if state.mode == FileInputMode::MultiSelect {
        refresh_multi_select_range_from_rows(state);
    }
}

pub fn move_selected_in_viewport(state: &mut FilesUiState, move_up: bool, visible_lines: usize) {
    let len = state.tree_rows.len();
    move_selected_index_with_scroll_offset(
        &mut state.selected,
        &mut state.scroll_offset,
        len,
        move_up,
        visible_lines,
    );
    if state.mode == FileInputMode::MultiSelect {
        refresh_multi_select_range_from_rows(state);
    }
}

pub fn move_commit_file_selected(state: &mut CommitFilesUiState, move_up: bool) {
    let len = state.tree_rows.len();
    move_selected_index(&mut state.selected, len, move_up);
    if state.mode == FileInputMode::MultiSelect {
        refresh_commit_files_multi_select_range_from_rows(state);
    }
}

pub fn move_commit_file_selected_in_viewport(
    state: &mut CommitFilesUiState,
    move_up: bool,
    visible_lines: usize,
) {
    let len = state.tree_rows.len();
    move_selected_index_with_scroll_offset(
        &mut state.selected,
        &mut state.scroll_offset,
        len,
        move_up,
        visible_lines,
    );
    if state.mode == FileInputMode::MultiSelect {
        refresh_commit_files_multi_select_range_from_rows(state);
    }
}

pub fn toggle_selected_directory(items: &[FileEntry], state: &mut FilesUiState) -> bool {
    let Some(row) = selected_row(items, state) else {
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
    refresh_tree_projection(items, state);
    clamp_selected(items, state);
    if state.mode == FileInputMode::MultiSelect {
        ensure_valid_selection_anchor(items, state);
        refresh_multi_select_range(items, state);
    }
    true
}

pub fn toggle_commit_files_directory(
    items: &[CommitFileEntry],
    state: &mut CommitFilesUiState,
) -> bool {
    let Some(row) = selected_commit_file_row(items, state) else {
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
    refresh_commit_files_tree_projection(items, state);
    clamp_commit_file_selected(items, state);
    if state.mode == FileInputMode::MultiSelect {
        ensure_valid_commit_files_selection_anchor(items, state);
        refresh_commit_files_multi_select_range(items, state);
    }
    true
}

pub fn enter_multi_select(items: &[FileEntry], state: &mut FilesUiState) {
    state.mode = FileInputMode::MultiSelect;
    state.selection_anchor = selected_row(items, state).map(|row| row.path);
    refresh_multi_select_range(items, state);
}

pub fn toggle_current_row_selection(items: &[FileEntry], state: &mut FilesUiState) {
    if state.mode == FileInputMode::MultiSelect {
        leave_multi_select(items, state);
    } else {
        enter_multi_select(items, state);
    }
}

pub fn leave_multi_select(items: &[FileEntry], state: &mut FilesUiState) {
    state.mode = FileInputMode::Normal;
    state.selection_anchor = None;
    state.selected_rows.clear();
    refresh_tree_projection(items, state);
}

pub fn enter_commit_files_multi_select(items: &[CommitFileEntry], state: &mut CommitFilesUiState) {
    state.mode = FileInputMode::MultiSelect;
    state.selection_anchor = selected_commit_file_row(items, state).map(|row| row.path);
    refresh_commit_files_multi_select_range(items, state);
}

pub fn leave_commit_files_multi_select(items: &[CommitFileEntry], state: &mut CommitFilesUiState) {
    state.mode = FileInputMode::Normal;
    state.selection_anchor = None;
    state.selected_rows.clear();
    refresh_commit_files_tree_projection(items, state);
}

pub fn selected_target_paths(items: &[FileEntry], state: &FilesUiState) -> Vec<String> {
    selected_diff_targets(items, state)
        .into_iter()
        .map(|target| target.path)
        .collect()
}

pub fn selected_diff_targets(items: &[FileEntry], state: &FilesUiState) -> Vec<FileDiffTarget> {
    let keys = if state.mode == FileInputMode::MultiSelect && !state.selected_rows.is_empty() {
        state.selected_rows.iter().cloned().collect::<Vec<_>>()
    } else {
        selected_row(items, state)
            .map(|row| vec![row.path])
            .unwrap_or_default()
    };
    resolve_row_keys_to_diff_targets(items, state, &keys)
}

pub fn selected_row(items: &[FileEntry], state: &FilesUiState) -> Option<FileTreeRow> {
    if !state.tree_rows.is_empty() || items.is_empty() {
        return state.tree_rows.get(state.selected).cloned();
    }
    compute_tree_projection(items, state)
        .rows
        .into_iter()
        .nth(state.selected)
}

pub fn selected_commit_file(
    items: &[CommitFileEntry],
    state: &CommitFilesUiState,
) -> Option<CommitFileEntry> {
    let row = selected_commit_file_row(items, state)?;
    if row.kind != FileRowKind::File {
        return None;
    }
    if let Some(index) = state.item_index_by_path.get(&row.path) {
        return items.get(*index).cloned();
    }
    items.iter().find(|item| item.path == row.path).cloned()
}

pub fn selected_commit_file_targets(
    items: &[CommitFileEntry],
    state: &CommitFilesUiState,
) -> Vec<CommitFileEntry> {
    if state.mode == FileInputMode::MultiSelect && !state.selected_rows.is_empty() {
        return selected_commit_file_targets_for_keys(
            items,
            state,
            &state.selected_rows.iter().cloned().collect::<Vec<_>>(),
        );
    }
    let Some(row) = selected_commit_file_row(items, state) else {
        return Vec::new();
    };
    if row.kind == FileRowKind::File {
        return selected_commit_file(items, state).into_iter().collect();
    }
    vec![CommitFileEntry {
        path: row.path,
        old_path: None,
        status: CommitFileStatus::Unknown,
    }]
}

pub fn select_file_tree_path(items: &[FileEntry], state: &mut FilesUiState, path: &str) -> bool {
    expand_ancestors(&mut state.expanded_dirs, path);
    refresh_tree_projection(items, state);
    if let Some(index) = state.row_index_by_path.get(path).copied() {
        state.selected = index;
        return true;
    }
    false
}

pub fn select_commit_file_tree_path(
    items: &[CommitFileEntry],
    state: &mut CommitFilesUiState,
    path: &str,
) -> bool {
    expand_ancestors(&mut state.expanded_dirs, path);
    refresh_commit_files_tree_projection(items, state);
    if let Some(index) = state.row_index_by_path.get(path).copied() {
        state.selected = index;
        return true;
    }
    false
}

fn selected_commit_file_row_ref(state: &CommitFilesUiState) -> Option<&FileTreeRow> {
    state.tree_rows.get(state.selected)
}

fn selected_commit_file_row(
    items: &[CommitFileEntry],
    state: &CommitFilesUiState,
) -> Option<FileTreeRow> {
    if let Some(row) = selected_commit_file_row_ref(state) {
        return Some(row.clone());
    }
    if items.is_empty() {
        return None;
    }
    compute_commit_files_tree_projection(items, state)
        .rows
        .into_iter()
        .nth(state.selected)
}

pub fn build_file_tree_rows(items: &[FileEntry], state: &FilesUiState) -> Vec<FileTreeRow> {
    if !state.tree_rows.is_empty() || items.is_empty() {
        return state.tree_rows.clone();
    }
    compute_tree_projection(items, state).rows
}

pub fn build_commit_file_tree_rows(
    items: &[CommitFileEntry],
    state: &CommitFilesUiState,
) -> Vec<FileTreeRow> {
    if !state.tree_rows.is_empty() || items.is_empty() {
        return state.tree_rows.clone();
    }
    compute_commit_files_tree_projection(items, state).rows
}

pub fn file_tree_rows_for_read<'a>(
    items: &[FileEntry],
    state: &'a FilesUiState,
) -> Cow<'a, [FileTreeRow]> {
    if !state.tree_rows.is_empty() || items.is_empty() {
        Cow::Borrowed(&state.tree_rows)
    } else {
        Cow::Owned(compute_tree_projection(items, state).rows)
    }
}

pub fn commit_file_tree_rows_for_read<'a>(
    items: &[CommitFileEntry],
    state: &'a CommitFilesUiState,
) -> Cow<'a, [FileTreeRow]> {
    if !state.tree_rows.is_empty() || items.is_empty() {
        Cow::Borrowed(&state.tree_rows)
    } else {
        Cow::Owned(compute_commit_files_tree_projection(items, state).rows)
    }
}

pub fn file_tree_rows(state: &FilesUiState) -> &[FileTreeRow] {
    &state.tree_rows
}

pub fn commit_file_tree_rows(state: &CommitFilesUiState) -> &[FileTreeRow] {
    &state.tree_rows
}

pub fn refresh_tree_projection(items: &[FileEntry], state: &mut FilesUiState) {
    sync_file_tree_index_if_needed(items, state);
    let projection = compute_tree_projection(items, state);
    state.tree_rows = projection.rows;
    state.row_descendants = projection.row_descendants;
    state.row_index_by_path = projection.row_index_by_path;
}

pub fn refresh_commit_files_tree_projection(
    items: &[CommitFileEntry],
    state: &mut CommitFilesUiState,
) {
    state.tree_index.sync(commit_file_tree_sources(items));
    let projection = compute_commit_files_tree_projection(items, state);
    state.tree_rows = projection.rows;
    state.row_descendants = projection.row_descendants;
    state.row_index_by_path = projection.row_index_by_path;
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

fn compute_tree_projection(items: &[FileEntry], state: &FilesUiState) -> TreeProjection {
    if state.lightweight_tree_projection {
        return compute_lightweight_tree_projection(items, state);
    }
    let entry_status = items
        .iter()
        .map(|entry| (entry.path.clone(), (entry.staged, entry.untracked)))
        .collect::<BTreeMap<_, _>>();
    compute_tree_projection_from_paths(
        items.iter().map(|entry| entry.path.clone()),
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

fn compute_lightweight_tree_projection(
    items: &[FileEntry],
    state: &FilesUiState,
) -> TreeProjection {
    let fallback_index;
    let index = if state.tree_index.is_empty() && !items.is_empty() {
        fallback_index = FileTreeIndex::from_sources(file_tree_sources(items));
        &fallback_index
    } else {
        &state.tree_index
    };
    let mut rows = Vec::new();
    for path in &index.root_paths {
        append_lightweight_tree_rows(
            index,
            path,
            &state.expanded_dirs,
            &state.selected_rows,
            &mut rows,
        );
    }
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

fn append_lightweight_tree_rows(
    index: &FileTreeIndex,
    path: &str,
    expanded_dirs: &BTreeSet<String>,
    selected_rows: &BTreeSet<String>,
    rows: &mut Vec<FileTreeRow>,
) {
    let Some(node) = index.nodes.get(path) else {
        return;
    };
    rows.push(FileTreeRow {
        depth: path_depth(path),
        expanded: expanded_dirs.contains(path),
        selected_for_batch: selected_rows.contains(path),
        name: tree_row_name(path),
        path: path.to_string(),
        kind: node.kind,
        staged: node.staged,
        untracked: node.untracked,
        commit_status: node.commit_status,
        matched: false,
    });
    if node.kind != FileRowKind::Directory || !expanded_dirs.contains(path) {
        return;
    }
    for child_path in &node.children {
        append_lightweight_tree_rows(index, child_path, expanded_dirs, selected_rows, rows);
    }
}

fn compute_commit_files_tree_projection(
    items: &[CommitFileEntry],
    state: &CommitFilesUiState,
) -> TreeProjection {
    let fallback_index;
    let index = if state.tree_index.is_empty() && !items.is_empty() {
        fallback_index = FileTreeIndex::from_sources(commit_file_tree_sources(items));
        &fallback_index
    } else {
        &state.tree_index
    };
    let mut rows = Vec::new();
    for path in &index.root_paths {
        append_lightweight_tree_rows(
            index,
            path,
            &state.expanded_dirs,
            &state.selected_rows,
            &mut rows,
        );
    }
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

fn sync_file_tree_index_if_needed(items: &[FileEntry], state: &mut FilesUiState) {
    if state.lightweight_tree_projection {
        state.tree_index.sync(file_tree_sources(items));
    } else if !state.tree_index.is_empty() {
        state.tree_index = FileTreeIndex::default();
    }
}

pub fn mark_file_items_changed(items: &[FileEntry], state: &mut FilesUiState) {
    state.tree_index.sync(file_tree_sources(items));
}

pub fn mark_commit_file_items_changed(items: &[CommitFileEntry], state: &mut CommitFilesUiState) {
    state.tree_index.sync(commit_file_tree_sources(items));
}

fn file_tree_sources(items: &[FileEntry]) -> BTreeMap<String, FileTreeSource> {
    items
        .iter()
        .filter_map(|entry| {
            let path = normalize_tree_path(&entry.path);
            if path.is_empty() {
                return None;
            }
            let kind = if is_directory_marker(&entry.path) {
                FileRowKind::Directory
            } else {
                FileRowKind::File
            };
            Some((
                path.clone(),
                FileTreeSource {
                    path,
                    kind,
                    staged: entry.staged,
                    untracked: entry.untracked,
                    commit_status: None,
                },
            ))
        })
        .collect()
}

fn commit_file_tree_sources(items: &[CommitFileEntry]) -> BTreeMap<String, FileTreeSource> {
    items
        .iter()
        .filter_map(|entry| {
            let path = normalize_tree_path(&entry.path);
            if path.is_empty() {
                return None;
            }
            Some((
                path.clone(),
                FileTreeSource {
                    path,
                    kind: FileRowKind::File,
                    staged: false,
                    untracked: false,
                    commit_status: Some(entry.status),
                },
            ))
        })
        .collect()
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

fn clamp_commit_file_selected(items: &[CommitFileEntry], state: &mut CommitFilesUiState) {
    let len = commit_file_tree_row_len(items, state);
    state.selected = if len == 0 {
        0
    } else {
        state.selected.min(len - 1)
    };
    state.scroll_offset = 0;
}

fn file_tree_row_len(items: &[FileEntry], state: &FilesUiState) -> usize {
    if !state.tree_rows.is_empty() || items.is_empty() {
        state.tree_rows.len()
    } else {
        compute_tree_projection(items, state).rows.len()
    }
}

fn commit_file_tree_row_len(items: &[CommitFileEntry], state: &CommitFilesUiState) -> usize {
    if !state.tree_rows.is_empty() || items.is_empty() {
        state.tree_rows.len()
    } else {
        compute_commit_files_tree_projection(items, state)
            .rows
            .len()
    }
}

fn ensure_valid_selection_anchor(items: &[FileEntry], state: &mut FilesUiState) {
    let rows = build_file_tree_rows(items, state);
    let anchor_is_valid = state
        .selection_anchor
        .as_ref()
        .is_some_and(|anchor| rows.iter().any(|row| &row.path == anchor));
    if !anchor_is_valid {
        state.selection_anchor = rows.get(state.selected).map(|row| row.path.clone());
    }
}

fn refresh_multi_select_range(items: &[FileEntry], state: &mut FilesUiState) {
    let rows = build_file_tree_rows(items, state);
    state.selected_rows.clear();
    let Some(anchor_path) = state.selection_anchor.clone() else {
        refresh_tree_projection(items, state);
        return;
    };
    let Some(anchor_index) = rows.iter().position(|row| row.path == anchor_path) else {
        refresh_tree_projection(items, state);
        return;
    };
    if rows.is_empty() {
        refresh_tree_projection(items, state);
        return;
    }
    let selected_index = state.selected.min(rows.len() - 1);
    let start = anchor_index.min(selected_index);
    let end = anchor_index.max(selected_index);
    for row in &rows[start..=end] {
        state.selected_rows.insert(row.path.clone());
    }
    refresh_tree_projection(items, state);
}

fn refresh_multi_select_range_from_rows(state: &mut FilesUiState) {
    state.selected_rows.clear();
    let Some(anchor_path) = state.selection_anchor.clone() else {
        sync_file_tree_batch_marks(state);
        return;
    };
    let Some(anchor_index) = state
        .tree_rows
        .iter()
        .position(|row| row.path == anchor_path)
    else {
        sync_file_tree_batch_marks(state);
        return;
    };
    if state.tree_rows.is_empty() {
        sync_file_tree_batch_marks(state);
        return;
    }
    let selected_index = state.selected.min(state.tree_rows.len() - 1);
    let start = anchor_index.min(selected_index);
    let end = anchor_index.max(selected_index);
    for row in &state.tree_rows[start..=end] {
        state.selected_rows.insert(row.path.clone());
    }
    sync_file_tree_batch_marks(state);
}

fn ensure_valid_commit_files_selection_anchor(
    items: &[CommitFileEntry],
    state: &mut CommitFilesUiState,
) {
    let rows = build_commit_file_tree_rows(items, state);
    let anchor_is_valid = state
        .selection_anchor
        .as_ref()
        .is_some_and(|anchor| rows.iter().any(|row| &row.path == anchor));
    if !anchor_is_valid {
        state.selection_anchor = rows.get(state.selected).map(|row| row.path.clone());
    }
}

fn refresh_commit_files_multi_select_range(
    items: &[CommitFileEntry],
    state: &mut CommitFilesUiState,
) {
    let rows = build_commit_file_tree_rows(items, state);
    state.selected_rows.clear();
    let Some(anchor_path) = state.selection_anchor.clone() else {
        refresh_commit_files_tree_projection(items, state);
        return;
    };
    let Some(anchor_index) = rows.iter().position(|row| row.path == anchor_path) else {
        refresh_commit_files_tree_projection(items, state);
        return;
    };
    if rows.is_empty() {
        refresh_commit_files_tree_projection(items, state);
        return;
    }
    let selected_index = state.selected.min(rows.len() - 1);
    let start = anchor_index.min(selected_index);
    let end = anchor_index.max(selected_index);
    for row in &rows[start..=end] {
        state.selected_rows.insert(row.path.clone());
    }
    refresh_commit_files_tree_projection(items, state);
}

fn refresh_commit_files_multi_select_range_from_rows(state: &mut CommitFilesUiState) {
    state.selected_rows.clear();
    let Some(anchor_path) = state.selection_anchor.clone() else {
        sync_commit_file_tree_batch_marks(state);
        return;
    };
    let Some(anchor_index) = state
        .tree_rows
        .iter()
        .position(|row| row.path == anchor_path)
    else {
        sync_commit_file_tree_batch_marks(state);
        return;
    };
    if state.tree_rows.is_empty() {
        sync_commit_file_tree_batch_marks(state);
        return;
    }
    let selected_index = state.selected.min(state.tree_rows.len() - 1);
    let start = anchor_index.min(selected_index);
    let end = anchor_index.max(selected_index);
    for row in &state.tree_rows[start..=end] {
        state.selected_rows.insert(row.path.clone());
    }
    sync_commit_file_tree_batch_marks(state);
}

fn sync_file_tree_batch_marks(state: &mut FilesUiState) {
    for row in &mut state.tree_rows {
        row.selected_for_batch = state.selected_rows.contains(&row.path);
    }
}

fn sync_commit_file_tree_batch_marks(state: &mut CommitFilesUiState) {
    for row in &mut state.tree_rows {
        row.selected_for_batch = state.selected_rows.contains(&row.path);
    }
}

fn selected_commit_file_targets_for_keys(
    items: &[CommitFileEntry],
    state: &CommitFilesUiState,
    keys: &[String],
) -> Vec<CommitFileEntry> {
    let entries = items
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let mut selected = BTreeMap::<String, CommitFileEntry>::new();
    for key in keys {
        let normalized_key = normalize_tree_path(key);
        if normalized_key.is_empty() {
            continue;
        }
        let prefix = format!("{normalized_key}/");
        let row_kind = build_commit_file_tree_rows(items, state)
            .iter()
            .find(|row| row.path == *key)
            .map(|row| row.kind);
        if row_kind == Some(FileRowKind::Directory) {
            selected.insert(
                normalized_key.clone(),
                CommitFileEntry {
                    path: normalized_key,
                    old_path: None,
                    status: CommitFileStatus::Unknown,
                },
            );
            continue;
        }
        for entry in items.iter().filter(|entry| {
            normalize_tree_path(&entry.path) == normalized_key || entry.path.starts_with(&prefix)
        }) {
            selected.insert(entry.path.clone(), entry.clone());
        }
        if let Some(entry) = entries.get(key.as_str()) {
            selected.insert(entry.path.clone(), (*entry).clone());
        }
    }
    selected.into_values().collect()
}

fn expand_ancestors(expanded_dirs: &mut BTreeSet<String>, path: &str) {
    let mut parts = path.split('/').collect::<Vec<_>>();
    while parts.len() > 1 {
        parts.pop();
        expanded_dirs.insert(parts.join("/"));
    }
}

fn resolve_row_keys_to_diff_targets(
    items: &[FileEntry],
    state: &FilesUiState,
    keys: &[String],
) -> Vec<FileDiffTarget> {
    let descendants = if state.row_descendants.is_empty() && !items.is_empty() {
        compute_tree_projection(items, state).row_descendants
    } else {
        state.row_descendants.clone()
    };
    let entries = items
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let mut paths = BTreeSet::new();
    for key in keys {
        if let Some(row_descendants) = descendants.get(key) {
            paths.extend(row_descendants.iter().cloned());
        } else {
            paths.extend(paths_for_row_key(items, key));
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

fn paths_for_row_key(items: &[FileEntry], key: &str) -> BTreeSet<String> {
    let normalized_key = normalize_tree_path(key);
    if normalized_key.is_empty() {
        return BTreeSet::new();
    }
    let prefix = format!("{normalized_key}/");
    items
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

fn source_path_chain(path: &str) -> Vec<String> {
    let parts = path.split('/').collect::<Vec<_>>();
    (0..parts.len())
        .map(|index| parts[..=index].join("/"))
        .collect()
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

    fn files() -> (Vec<FileEntry>, FilesUiState) {
        let items = vec![
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
        ];
        let mut state = FilesUiState::default();
        initialize_tree_if_needed(&items, &mut state);
        (items, state)
    }

    #[test]
    fn tree_rows_include_directories_and_files() {
        let (items, state) = files();
        let rows = build_file_tree_rows(&items, &state);
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
        let (items, mut state) = files();
        state.selected = build_file_tree_rows(&items, &state)
            .iter()
            .position(|row| row.path == "src")
            .expect("src row exists");
        assert_eq!(
            selected_target_paths(&items, &state),
            vec!["src/main.rs".to_string(), "src/ui/list.rs".to_string()]
        );
    }

    #[test]
    fn select_file_tree_path_expands_ancestors() {
        let (items, mut state) = files();
        state.expanded_dirs.remove("src");
        refresh_tree_projection(&items, &mut state);

        assert!(select_file_tree_path(&items, &mut state, "src/main.rs"));
        assert_eq!(
            selected_row(&items, &state).map(|row| row.path),
            Some("src/main.rs".to_string())
        );
        assert!(state.expanded_dirs.contains("src"));
    }

    #[test]
    fn multi_select_resolves_unique_file_targets() {
        let (items, mut state) = files();
        state.selected_rows.insert("src".to_string());
        state.selected_rows.insert("src/main.rs".to_string());
        state.mode = FileInputMode::MultiSelect;
        assert_eq!(
            selected_target_paths(&items, &state),
            vec!["src/main.rs".to_string(), "src/ui/list.rs".to_string()]
        );
    }

    #[test]
    fn untracked_directory_marker_renders_as_directory_node() {
        let items = vec![FileEntry {
            path: "libs/ratagit-git/tests/".to_string(),
            staged: false,
            untracked: true,
        }];
        let mut state = FilesUiState::default();
        initialize_tree_if_needed(&items, &mut state);

        let rows = build_file_tree_rows(&items, &state);
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

    #[test]
    fn lightweight_tree_projection_uses_cached_child_index_for_toggles() {
        let items = vec![
            FileEntry {
                path: "src/lib.rs".to_string(),
                staged: false,
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
        ];
        let mut state = FilesUiState {
            lightweight_tree_projection: true,
            ..FilesUiState::default()
        };
        refresh_tree_projection(&items, &mut state);

        assert!(!state.tree_index.is_empty());
        assert!(state.row_descendants.is_empty());
        assert_eq!(
            state
                .tree_rows
                .iter()
                .map(|row| row.path.as_str())
                .collect::<Vec<_>>(),
            vec!["README.md", "src"]
        );

        state.expanded_dirs.insert("src".to_string());
        refresh_tree_projection(&items, &mut state);

        assert_eq!(
            state
                .tree_rows
                .iter()
                .map(|row| row.path.as_str())
                .collect::<Vec<_>>(),
            vec!["README.md", "src", "src/lib.rs", "src/ui"]
        );

        state.expanded_dirs.insert("src/ui".to_string());
        refresh_tree_projection(&items, &mut state);

        assert_eq!(
            state
                .tree_rows
                .iter()
                .map(|row| row.path.as_str())
                .collect::<Vec<_>>(),
            vec!["README.md", "src", "src/lib.rs", "src/ui", "src/ui/list.rs"]
        );
    }

    #[test]
    fn commit_files_tree_projection_uses_shared_index_for_toggles() {
        let items = vec![
            CommitFileEntry {
                path: "src/lib.rs".to_string(),
                old_path: None,
                status: CommitFileStatus::Added,
            },
            CommitFileEntry {
                path: "src/ui/list.rs".to_string(),
                old_path: None,
                status: CommitFileStatus::Modified,
            },
        ];
        let mut state = CommitFilesUiState::default();
        initialize_commit_files_tree(&items, &mut state);

        assert!(!state.tree_index.is_empty());
        assert_eq!(
            state
                .tree_rows
                .iter()
                .map(|row| row.path.as_str())
                .collect::<Vec<_>>(),
            vec!["src", "src/lib.rs", "src/ui", "src/ui/list.rs"]
        );

        state.expanded_dirs.remove("src");
        refresh_commit_files_tree_projection(&items, &mut state);

        assert_eq!(
            state
                .tree_rows
                .iter()
                .map(|row| row.path.as_str())
                .collect::<Vec<_>>(),
            vec!["src"]
        );
        assert!(state.row_descendants.is_empty());
    }

    #[test]
    fn shared_tree_index_syncs_status_metadata_without_changing_children() {
        let mut index = FileTreeIndex::from_sources(file_tree_sources(&[FileEntry {
            path: "src/lib.rs".to_string(),
            staged: false,
            untracked: false,
        }]));
        let before_children = index.nodes.get("src").expect("src node").children.clone();

        index.sync(file_tree_sources(&[FileEntry {
            path: "src/lib.rs".to_string(),
            staged: true,
            untracked: false,
        }]));

        assert_eq!(
            index.nodes.get("src").expect("src node").children,
            before_children
        );
        assert!(index.nodes.get("src/lib.rs").expect("file node").staged);
    }
}
