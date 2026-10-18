//! Sibling repository views share a physical panel, never its action targets.
use std::collections::BTreeMap;

use crate::{AppContext, Command, LinearListSelection, PanelFocus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RepositoryTab {
    Submodules,
    Worktrees,
    Remotes,
    Tags,
    Reflog,
}

impl RepositoryTab {
    pub fn title(self) -> &'static str {
        match self {
            Self::Submodules => "Submodules",
            Self::Worktrees => "Worktrees",
            Self::Remotes => "Remotes",
            Self::Tags => "Tags",
            Self::Reflog => "Reflog",
        }
    }
    pub fn panel(self) -> PanelFocus {
        match self {
            Self::Submodules | Self::Worktrees => PanelFocus::Files,
            Self::Remotes | Self::Tags => PanelFocus::Branches,
            Self::Reflog => PanelFocus::Commits,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryItem {
    Submodule {
        path: String,
        url: String,
        head: String,
        status: String,
    },
    Worktree {
        path: String,
        head: String,
        branch: String,
        detached: bool,
        locked: Option<String>,
        prunable: Option<String>,
        main: bool,
    },
    Remote {
        name: String,
        fetch_url: String,
        push_url: String,
    },
    RemoteBranch {
        name: String,
        target: String,
    },
    Tag {
        name: String,
        target: String,
        summary: String,
    },
    Reflog {
        selector: String,
        target: String,
        message: String,
    },
}

impl RepositoryItem {
    pub fn key(&self) -> String {
        match self {
            Self::Submodule { path, .. } | Self::Worktree { path, .. } => path.clone(),
            Self::Remote { name, .. } => format!("remote:{name}"),
            Self::RemoteBranch { name, .. } => format!("remote-branch:{name}"),
            Self::Tag { name, .. } => name.clone(),
            Self::Reflog { selector, .. } => selector.clone(),
        }
    }
    pub fn label(&self) -> String {
        match self {
            Self::Submodule { path, status, .. } => format!("{path}  {status}"),
            Self::Worktree {
                path,
                branch,
                detached,
                locked,
                prunable,
                main,
                ..
            } => format!(
                "{}{}  {}{}{}",
                if *main { "* " } else { "" },
                path,
                if *detached { "detached" } else { branch },
                if locked.is_some() { " [locked]" } else { "" },
                if prunable.is_some() {
                    " [prunable]"
                } else {
                    ""
                }
            ),
            Self::Remote {
                name, fetch_url, ..
            } => format!("{name}  {fetch_url}"),
            Self::RemoteBranch { name, target } => format!("  {name}  {}", short_hash(target)),
            Self::Tag { name, target, .. } => format!("{name}  {}", short_hash(target)),
            Self::Reflog {
                selector,
                target,
                message,
            } => format!("{selector} {} {message}", short_hash(target)),
        }
    }
    pub fn details(&self) -> String {
        match self {
            Self::Submodule {
                path,
                url,
                head,
                status,
            } => format!("Submodule: {path}\nURL: {url}\nHEAD: {head}\nStatus: {status}"),
            Self::Worktree {
                path,
                head,
                branch,
                detached,
                locked,
                prunable,
                main,
            } => format!(
                "Worktree: {path}\nHEAD: {head}\nBranch: {branch}\nDetached: {detached}\nMain worktree: {main}\nLocked: {}\nPrunable: {}",
                locked.as_deref().unwrap_or("no"),
                prunable.as_deref().unwrap_or("no")
            ),
            Self::Remote {
                name,
                fetch_url,
                push_url,
            } => format!(
                "Remote: {name}\nFetch URL: {fetch_url}\nPush URL: {push_url}\n\nRemote-tracking branches follow their remote in the list."
            ),
            Self::RemoteBranch { name, target } => format!(
                "Remote-tracking branch: {name}\nRef: refs/remotes/{name}\nTarget: {target}"
            ),
            Self::Tag {
                name,
                target,
                summary,
            } => format!("Tag: {name}\nRef: refs/tags/{name}\nTarget: {target}\n\n{summary}"),
            Self::Reflog {
                selector,
                target,
                message,
            } => format!("Reflog: {selector}\nCommit: {target}\n\n{message}"),
        }
    }
}

fn short_hash(hash: &str) -> &str {
    hash.get(..8).unwrap_or(hash)
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RepositoryList {
    pub items: Vec<RepositoryItem>,
    pub loaded: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TabUiState {
    pub files: Option<RepositoryTab>,
    pub branches: Option<RepositoryTab>,
    pub commits: Option<RepositoryTab>,
    pub selections: BTreeMap<RepositoryTab, LinearListSelection>,
}

impl TabUiState {
    pub fn active(&self, panel: PanelFocus) -> Option<RepositoryTab> {
        match panel {
            PanelFocus::Files => self.files,
            PanelFocus::Branches => self.branches,
            PanelFocus::Commits => self.commits,
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TabWork {
    pub next_request: u64,
    pub pending: BTreeMap<RepositoryTab, u64>,
}

pub const REPOSITORY_LIST_LIMIT: usize = 1000;

pub(crate) fn cycle(state: &mut AppContext, previous: bool) -> Vec<Command> {
    if state.ui.editor.is_active()
        || state.ui.workflow.dialog.is_some()
        || state.ui.workflow.remote_request.is_some()
        || state.ui.custom_patch.preview_return_focus.is_some()
    {
        return Vec::new();
    }
    let (slot, siblings): (&mut Option<RepositoryTab>, &[Option<RepositoryTab>]) =
        match state.ui.focus {
            PanelFocus::Files => (
                &mut state.ui.tabs.files,
                &[
                    None,
                    Some(RepositoryTab::Submodules),
                    Some(RepositoryTab::Worktrees),
                ],
            ),
            PanelFocus::Branches => (
                &mut state.ui.tabs.branches,
                &[
                    None,
                    Some(RepositoryTab::Remotes),
                    Some(RepositoryTab::Tags),
                ],
            ),
            PanelFocus::Commits => (
                &mut state.ui.tabs.commits,
                &[None, Some(RepositoryTab::Reflog)],
            ),
            _ => return Vec::new(),
        };
    let index = siblings.iter().position(|tab| tab == slot).unwrap_or(0);
    *slot = siblings[(index + if previous { siblings.len() - 1 } else { 1 }) % siblings.len()];
    state.ui.search.clear();
    state.work.clear_details_pending();
    state.work.details.patch_request = None;
    state.ui.details.patch.active = false;
    state.ui.details.scroll_offset = 0;
    state.ui.last_left_focus = state.ui.focus;
    crate::details::refresh_for_focus(state)
}

pub(crate) fn load(state: &mut AppContext, tab: RepositoryTab, force: bool) -> Vec<Command> {
    if !force
        && (state.repo.tabs.get(&tab).is_some_and(|list| list.loaded)
            || state.work.tabs.pending.contains_key(&tab))
    {
        return Vec::new();
    }
    state.work.tabs.next_request += 1;
    let request_id = state.work.tabs.next_request;
    state.work.tabs.pending.insert(tab, request_id);
    vec![Command::LoadRepositoryTab { tab, request_id }]
}

pub(crate) fn loaded(
    state: &mut AppContext,
    tab: RepositoryTab,
    request_id: u64,
    result: Result<Vec<RepositoryItem>, String>,
) -> Vec<Command> {
    if state.work.tabs.pending.get(&tab) != Some(&request_id) {
        return Vec::new();
    }
    state.work.tabs.pending.remove(&tab);
    let list = state.repo.tabs.entry(tab).or_default();
    let selection = state.ui.tabs.selections.entry(tab).or_default();
    let selected_key = list.items.get(selection.selected).map(|item| item.key());
    list.loaded = true;
    match result {
        Ok(mut items) => {
            items.truncate(REPOSITORY_LIST_LIMIT);
            list.items = items;
            list.error = None;
            selection.selected = selected_key
                .and_then(|key| list.items.iter().position(|item| item.key() == key))
                .unwrap_or(selection.selected)
                .min(list.items.len().saturating_sub(1));
            selection.scroll_offset = selection.scroll_offset.min(selection.selected);
        }
        Err(error) => {
            list.error = Some(error);
            list.items.clear();
            selection.selected = 0;
            selection.scroll_offset = 0;
        }
    }
    Vec::new()
}

pub(crate) fn load_visible(state: &mut AppContext) -> Vec<Command> {
    [
        state.ui.tabs.files,
        state.ui.tabs.branches,
        state.ui.tabs.commits,
    ]
    .into_iter()
    .flatten()
    .flat_map(|tab| load(state, tab, false))
    .collect()
}

pub(crate) fn invalidate(state: &mut AppContext) {
    state.work.tabs.pending.clear();
    for list in state.repo.tabs.values_mut() {
        list.loaded = false;
    }
}

pub fn selected_item(state: &AppContext, tab: RepositoryTab) -> Option<&RepositoryItem> {
    let selected = state
        .ui
        .tabs
        .selections
        .get(&tab)
        .map_or(0, |selection| selection.selected);
    state.repo.tabs.get(&tab)?.items.get(selected)
}

pub(crate) fn move_selection(
    state: &mut AppContext,
    tab: RepositoryTab,
    up: bool,
    visible_lines: usize,
) {
    let len = state.repo.tabs.get(&tab).map_or(0, |list| list.items.len());
    let selection = state.ui.tabs.selections.entry(tab).or_default();
    crate::scroll::move_selected_index_with_scroll_offset(
        &mut selection.selected,
        &mut selection.scroll_offset,
        len,
        up,
        visible_lines,
    );
    state.ui.details.scroll_offset = 0;
}

pub fn details_text(state: &AppContext, tab: RepositoryTab) -> String {
    if state.work.tabs.pending.contains_key(&tab) {
        return format!("Loading {}…", tab.title());
    }
    if let Some(error) = state
        .repo
        .tabs
        .get(&tab)
        .and_then(|list| list.error.as_deref())
    {
        return format!("{}: {error}", tab.title());
    }
    selected_item(state, tab).map_or_else(
        || format!("No {}", tab.title().to_lowercase()),
        RepositoryItem::details,
    )
}
