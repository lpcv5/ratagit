use super::{AppState, CommitFieldFocus, InputMode, SelectionView, SidePanel, ViewContext};
use crate::config::keymap::{key_to_string, Keymap};
use crate::flux::action::DomainAction;
use crossterm::event::KeyEvent;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandId {
    FocusDiff,
    DiffSide,
    HunkNext,
    HunkPrevious,
    HunkSelect,
    HunkStage,
    HunkUnstage,
    Back,
    Confirm,
    SwitchField,
    Backspace,
    Quit,
    ListUp,
    ListDown,
    PanelNext,
    PanelPrev,
    DiffUp,
    DiffDown,
    Files,
    Branches,
    Commits,
    Stashes,
    Commit,
    Palette,
    Search,
    SearchNext,
    SearchPrev,
    Refresh,
    ToggleStage,
    Discard,
    StashPush,
    Visual,
    ToggleDir,
    Collapse,
    Expand,
    Checkout,
    CreateBranch,
    DeleteBranch,
    Fetch,
    OpenRevision,
    StashApply,
    StashPop,
    StashDrop,
}

pub struct CommandEnv<'a> {
    pub diff: super::HunkView<'a>,
    pub selection: SelectionView<'a>,
    pub input: Option<InputMode>,
    pub commit_focus: CommitFieldFocus,
    pub has_search: bool,
    pub has_search_query: bool,
}

impl CommandEnv<'_> {
    pub fn view(&self) -> &ViewContext {
        &self.selection.context
    }
}

#[derive(Clone, Copy)]
pub enum Binding {
    Global(&'static str, &'static [&'static str]),
    Panel(SidePanel, &'static str, &'static [&'static str]),
    Fixed,
}

pub struct CommandSpec {
    pub id: CommandId,
    pub binding: Binding,
    pub aliases: &'static [&'static str],
    pub label: &'static str,
    pub footer_rank: Option<u16>,
}

macro_rules! command {
    ($id:ident, $binding:expr, [$($alias:literal),*], $label:literal, $rank:expr) => {
        CommandSpec { id: CommandId::$id, binding: $binding, aliases: &[$($alias),*], label: $label, footer_rank: $rank }
    };
}

/// Declaration order is also the deterministic key-collision priority. Global
/// bindings retain precedence; contextual Escape and modal input come first.
pub static COMMANDS: &[CommandSpec] = &[
    command!(Back, Binding::Fixed, ["back"], "Back", Some(1)),
    command!(Confirm, Binding::Fixed, [], "Confirm", Some(10)),
    command!(SwitchField, Binding::Fixed, [], "SwitchField", Some(5)),
    command!(Backspace, Binding::Fixed, [], "Backspace", None),
    command!(
        DiffSide,
        Binding::Global("diff_side", &["Enter"]),
        ["diff side"],
        "SwitchSide",
        Some(7)
    ),
    command!(
        FocusDiff,
        Binding::Global("focus_diff", &["Tab"]),
        ["diff focus"],
        "FocusHunks",
        Some(2)
    ),
    command!(
        HunkNext,
        Binding::Global("hunk_next", &["j", "Down", "]"]),
        ["hunk next"],
        "NextHunk",
        Some(5)
    ),
    command!(
        HunkPrevious,
        Binding::Global("hunk_previous", &["k", "Up", "["]),
        ["hunk previous", "hunk prev"],
        "PrevHunk",
        Some(6)
    ),
    command!(
        HunkSelect,
        Binding::Global("hunk_select", &["Space"]),
        ["hunk select"],
        "SelectHunk",
        Some(4)
    ),
    command!(
        HunkStage,
        Binding::Global("hunk_stage", &["s"]),
        ["hunk stage"],
        "StageHunks",
        Some(3)
    ),
    command!(
        HunkUnstage,
        Binding::Global("hunk_unstage", &["u"]),
        ["hunk unstage"],
        "UnstageHunks",
        Some(3)
    ),
    command!(
        Quit,
        Binding::Global("quit", &["q"]),
        ["quit", "q", "exit"],
        "Quit",
        Some(1000)
    ),
    command!(
        ListUp,
        Binding::Global("list_up", &["k", "Up"]),
        ["up"],
        "Up",
        None
    ),
    command!(
        ListDown,
        Binding::Global("list_down", &["j", "Down"]),
        ["down"],
        "Down",
        None
    ),
    command!(
        PanelNext,
        Binding::Global("panel_next", &["l", "Right"]),
        ["panel next"],
        "NextPanel",
        None
    ),
    command!(
        PanelPrev,
        Binding::Global("panel_prev", &["h", "Left"]),
        ["panel prev"],
        "PrevPanel",
        None
    ),
    command!(
        DiffUp,
        Binding::Global("diff_scroll_up", &["C-u"]),
        ["diff up"],
        "ScrollUp",
        None
    ),
    command!(
        DiffDown,
        Binding::Global("diff_scroll_down", &["C-d"]),
        ["diff down"],
        "ScrollDown",
        None
    ),
    command!(
        Files,
        Binding::Global("panel_1", &["1"]),
        ["files", "panel files", "panel 1"],
        "Files",
        None
    ),
    command!(
        Branches,
        Binding::Global("panel_2", &["2"]),
        ["branches", "panel branches", "panel 2"],
        "Branches",
        None
    ),
    command!(
        Commits,
        Binding::Global("panel_3", &["3"]),
        ["commits", "panel commits", "panel 3"],
        "Commits",
        None
    ),
    command!(
        Stashes,
        Binding::Global("panel_4", &["4"]),
        ["stash panel", "panel stash", "panel 4"],
        "Stashes",
        None
    ),
    command!(
        Commit,
        Binding::Global("commit", &["c"]),
        ["commit", "c"],
        "Commit",
        Some(30)
    ),
    command!(
        Palette,
        Binding::Global("command_palette", &[":"]),
        ["command", ":"],
        "Command",
        Some(900)
    ),
    command!(
        Search,
        Binding::Global("search_start", &["/"]),
        ["search", "/"],
        "Search",
        None
    ),
    command!(
        SearchNext,
        Binding::Global("search_next", &["n"]),
        ["search next"],
        "NextMatch",
        None
    ),
    command!(
        SearchPrev,
        Binding::Global("search_prev", &["N"]),
        ["search prev"],
        "PrevMatch",
        None
    ),
    command!(
        Refresh,
        Binding::Global("refresh", &["r"]),
        ["refresh"],
        "Refresh",
        None
    ),
    command!(
        ToggleStage,
        Binding::Panel(SidePanel::Files, "toggle_stage", &["Space"]),
        ["stage", "toggle stage"],
        "Stage",
        Some(10)
    ),
    command!(
        Discard,
        Binding::Panel(SidePanel::Files, "discard", &["d"]),
        ["discard"],
        "Discard",
        None
    ),
    command!(
        StashPush,
        Binding::Panel(SidePanel::Files, "stash_push", &["s"]),
        ["stash", "stash push"],
        "Stash",
        None
    ),
    command!(
        Visual,
        Binding::Panel(SidePanel::Files, "toggle_visual_select", &["v"]),
        ["visual"],
        "Visual",
        Some(20)
    ),
    command!(
        ToggleDir,
        Binding::Panel(SidePanel::Files, "toggle_dir", &["Enter"]),
        ["toggle dir"],
        "ToggleDir",
        None
    ),
    command!(
        Collapse,
        Binding::Panel(SidePanel::Files, "collapse_all", &["-"]),
        ["collapse"],
        "Collapse",
        None
    ),
    command!(
        Expand,
        Binding::Panel(SidePanel::Files, "expand_all", &["="]),
        ["expand"],
        "Expand",
        None
    ),
    command!(
        Checkout,
        Binding::Panel(SidePanel::LocalBranches, "checkout_branch", &["Space"]),
        ["checkout"],
        "Checkout",
        Some(10)
    ),
    command!(
        CreateBranch,
        Binding::Panel(SidePanel::LocalBranches, "create_branch", &["n"]),
        ["branch new", "branch create", "branch"],
        "New",
        Some(30)
    ),
    command!(
        DeleteBranch,
        Binding::Panel(SidePanel::LocalBranches, "delete_branch", &["d"]),
        ["branch delete"],
        "Delete",
        None
    ),
    command!(
        Fetch,
        Binding::Panel(SidePanel::LocalBranches, "fetch_remote", &["f"]),
        ["fetch", "fetch remote"],
        "Fetch",
        Some(40)
    ),
    command!(
        OpenRevision,
        Binding::Panel(SidePanel::LocalBranches, "open_tree", &["Enter"]),
        ["open", "open files"],
        "Files",
        Some(20)
    ),
    command!(
        StashApply,
        Binding::Panel(SidePanel::Stash, "stash_apply", &["a"]),
        ["stash apply"],
        "Apply",
        Some(10)
    ),
    command!(
        StashPop,
        Binding::Panel(SidePanel::Stash, "stash_pop", &["p"]),
        ["stash pop"],
        "Pop",
        Some(20)
    ),
    command!(
        StashDrop,
        Binding::Panel(SidePanel::Stash, "stash_drop", &["d"]),
        ["stash drop"],
        "Drop",
        None
    ),
];

impl CommandSpec {
    pub fn action(&self, env: &CommandEnv<'_>) -> Option<DomainAction> {
        use CommandId::*;
        if env.input.is_some() && !matches!(self.id, Back | Confirm | SwitchField | Backspace) {
            return None;
        }
        if env.diff.interaction.focused
            && !matches!(
                self.id,
                Back | FocusDiff
                    | DiffSide
                    | HunkNext
                    | HunkPrevious
                    | HunkSelect
                    | HunkStage
                    | HunkUnstage
                    | Quit
                    | Palette
                    | Refresh
                    | DiffUp
                    | DiffDown
                    | PanelNext
                    | PanelPrev
                    | Files
                    | Branches
                    | Commits
                    | Stashes
                    | Confirm
                    | SwitchField
                    | Backspace
            )
        {
            return None;
        }
        Some(match self.id {
            DiffSide if env.diff.can_switch_side(&env.selection.diff_target()) => {
                DomainAction::SwitchDiffSide
            }
            FocusDiff if env.diff.can_toggle_focus() => DomainAction::ToggleDiffFocus,
            HunkNext if env.diff.can_navigate() => DomainAction::HunkNext,
            HunkPrevious if env.diff.can_navigate() => DomainAction::HunkPrevious,
            HunkSelect if env.diff.can_navigate() => DomainAction::ToggleHunkSelection,
            HunkStage if env.diff.can_apply(crate::git::patch::PatchDirection::Stage) => {
                DomainAction::ApplySelectedHunks(crate::git::patch::PatchDirection::Stage)
            }
            HunkUnstage
                if env
                    .diff
                    .can_apply(crate::git::patch::PatchDirection::Unstage) =>
            {
                DomainAction::ApplySelectedHunks(crate::git::patch::PatchDirection::Unstage)
            }
            Back => match env.input {
                Some(InputMode::BranchSwitchConfirm) => DomainAction::BranchSwitchConfirm(false),
                Some(InputMode::CommitAllConfirm) => DomainAction::CommitAllConfirm(false),
                Some(_) => DomainAction::InputEsc,
                None if env.diff.interaction.focused
                    && !env.diff.interaction.selected.is_empty() =>
                {
                    DomainAction::ClearHunkSelection
                }
                None if env.diff.interaction.focused => DomainAction::ToggleDiffFocus,
                None if env.has_search_query => DomainAction::SearchClear,
                None if env.selection.file_selection().is_visual_mode() => {
                    DomainAction::ToggleVisualSelectMode
                }
                None if env.view().parent().is_some() => DomainAction::RevisionCloseTree,
                None => return None,
            },
            Confirm => match env.input {
                Some(InputMode::BranchSwitchConfirm) => DomainAction::BranchSwitchConfirm(true),
                Some(InputMode::CommitAllConfirm) => DomainAction::CommitAllConfirm(true),
                Some(_) => DomainAction::InputEnter,
                None => return None,
            },
            SwitchField if matches!(env.input, Some(mode) if !matches!(mode, InputMode::BranchSwitchConfirm | InputMode::CommitAllConfirm)) => {
                DomainAction::InputTab
            }
            Backspace if matches!(env.input, Some(mode) if !matches!(mode, InputMode::BranchSwitchConfirm | InputMode::CommitAllConfirm)) => {
                DomainAction::InputBackspace
            }
            SwitchField | Backspace => return None,
            Quit => DomainAction::Quit,
            ListUp => DomainAction::ListUp,
            ListDown => DomainAction::ListDown,
            PanelNext => DomainAction::PanelNext,
            PanelPrev => DomainAction::PanelPrev,
            DiffUp => DomainAction::DiffScrollUp,
            DiffDown => DomainAction::DiffScrollDown,
            Files => DomainAction::PanelGoto(1),
            Branches => DomainAction::PanelGoto(2),
            Commits => DomainAction::PanelGoto(3),
            Stashes => DomainAction::PanelGoto(4),
            Commit if env.selection.file_selection().is_visual_mode() => {
                DomainAction::PrepareCommitFromSelection
            }
            Commit => DomainAction::StartCommitInput,
            Palette => DomainAction::StartCommandPalette,
            Search => DomainAction::StartSearchInput,
            SearchNext if env.has_search => DomainAction::SearchNext,
            SearchPrev if env.has_search => DomainAction::SearchPrev,
            SearchNext | SearchPrev => return None,
            Refresh => DomainAction::Refresh,
            ToggleStage if *env.view() == ViewContext::Files => {
                if env.selection.file_selection().is_visual_mode() {
                    DomainAction::ToggleStageSelection
                } else {
                    return env
                        .selection
                        .file_selection()
                        .toggle_stage_target()
                        .map(|target| match target {
                            super::files::StageTarget::Stage(path) => DomainAction::StageFile(path),
                            super::files::StageTarget::Unstage(path) => {
                                DomainAction::UnstageFile(path)
                            }
                        });
                }
            }
            Discard if *env.view() == ViewContext::Files => {
                if env.selection.file_selection().is_visual_mode() {
                    DomainAction::DiscardSelection
                } else {
                    let paths = env
                        .selection
                        .file_selection()
                        .prepare_discard_targets_from_selection();
                    if paths.is_empty() {
                        return None;
                    }
                    DomainAction::DiscardPaths(paths)
                }
            }
            StashPush if *env.view() == ViewContext::Files => DomainAction::StartStashInput,
            Visual if *env.view() == ViewContext::Files => DomainAction::ToggleVisualSelectMode,
            ToggleDir if *env.view() == ViewContext::Files => DomainAction::ToggleDir,
            Collapse if *env.view() == ViewContext::Files => DomainAction::CollapseAll,
            Expand if *env.view() == ViewContext::Files => DomainAction::ExpandAll,
            Checkout if *env.view() == ViewContext::Branches => {
                DomainAction::CheckoutSelectedBranch
            }
            DeleteBranch if *env.view() == ViewContext::Branches => {
                DomainAction::DeleteSelectedBranch
            }
            CreateBranch => DomainAction::StartBranchCreateInput,
            Fetch => DomainAction::FetchRemote,
            OpenRevision if env.view().panel() != SidePanel::Files => {
                DomainAction::RevisionOpenTreeOrToggleDir
            }
            StashApply if env.view().panel() == SidePanel::Stash => {
                DomainAction::StashApplySelected
            }
            StashPop if env.view().panel() == SidePanel::Stash => DomainAction::StashPopSelected,
            StashDrop if env.view().panel() == SidePanel::Stash => DomainAction::StashDropSelected,
            _ => return None,
        })
    }

    pub fn label(&self, env: &CommandEnv<'_>) -> String {
        use CommandId::*;
        let label = match self.id {
            Back if env.input.is_some() => "Cancel",
            Back if env.diff.interaction.focused && !env.diff.interaction.selected.is_empty() => {
                "ClearHunks"
            }
            Back if env.diff.interaction.focused => "BackToList",
            FocusDiff if env.diff.interaction.focused => "FocusList",
            HunkSelect
                if env
                    .diff
                    .interaction
                    .cursor
                    .is_some_and(|id| env.diff.interaction.selected.contains(&id)) =>
            {
                "DeselectHunk"
            }
            Back if env.has_search_query => "ClearSearch",
            Back if env.selection.file_selection().is_visual_mode() => "ExitVisual",
            Confirm => match env.input {
                Some(InputMode::BranchSwitchConfirm) => "AutoStash+Switch",
                Some(InputMode::CommitAllConfirm) => "StageAll+Commit",
                Some(InputMode::CommandPalette) => "Run",
                Some(InputMode::CommitEditor)
                    if env.commit_focus == CommitFieldFocus::Description =>
                {
                    "Newline"
                }
                _ => self.label,
            },
            Commit if env.selection.file_selection().is_visual_mode() => "CommitSelection",
            ToggleStage if env.selection.file_selection().is_visual_mode() => "ToggleStage",
            ToggleStage if matches!(self.action(env), Some(DomainAction::UnstageFile(_))) => {
                "Unstage"
            }
            OpenRevision => match env.view() {
                ViewContext::Branches => "Commits",
                ViewContext::BranchCommits { .. } => "Refresh",
                ViewContext::CommitFiles { .. } | ViewContext::StashFiles { .. } => "ToggleDir",
                _ => self.label,
            },
            _ => self.label,
        };
        label.into()
    }

    fn keys<'a>(&self, keymap: &'a Keymap, env: &CommandEnv<'_>) -> Vec<&'a str> {
        if self.id == CommandId::OpenRevision && env.view().panel() != SidePanel::Files {
            return keymap.panel_keys(env.view().panel().keymap_name(), "open_tree");
        }
        match self.binding {
            Binding::Global(name, _) => keymap.global_keys(name),
            Binding::Panel(panel, name, _) if panel == env.view().panel() => {
                keymap.panel_keys(panel.keymap_name(), name)
            }
            Binding::Panel(_, _, _) => Vec::new(),
            Binding::Fixed => match self.id {
                CommandId::Back
                    if matches!(
                        env.input,
                        Some(InputMode::BranchSwitchConfirm | InputMode::CommitAllConfirm)
                    ) =>
                {
                    vec!["n", "N", "Esc"]
                }
                CommandId::Back => vec!["Esc"],
                CommandId::Confirm
                    if matches!(
                        env.input,
                        Some(InputMode::BranchSwitchConfirm | InputMode::CommitAllConfirm)
                    ) =>
                {
                    vec!["y", "Y", "Enter"]
                }
                CommandId::Confirm => vec!["Enter"],
                CommandId::SwitchField => vec!["Tab"],
                CommandId::Backspace => vec!["Backspace"],
                _ => Vec::new(),
            },
        }
    }

    fn matches_key(&self, key: &str, keymap: &Keymap, env: &CommandEnv<'_>) -> bool {
        let binding =
            if self.id == CommandId::OpenRevision && env.view().panel() != SidePanel::Files {
                Binding::Panel(env.view().panel(), "open_tree", &[])
            } else {
                self.binding
            };
        match binding {
            Binding::Global(name, _) => keymap
                .global_actions(key)
                .iter()
                .any(|candidate| candidate == name),
            Binding::Panel(panel, name, _) if panel == env.view().panel() => keymap
                .panel_actions(panel.keymap_name(), key)
                .iter()
                .any(|candidate| candidate == name),
            Binding::Panel(_, _, _) => false,
            Binding::Fixed => self.keys(keymap, env).contains(&key),
        }
    }

    fn visible_in_footer(&self, env: &CommandEnv<'_>) -> bool {
        match self.id {
            CommandId::Commit => *env.view() == ViewContext::Files,
            CommandId::CreateBranch | CommandId::Fetch => *env.view() == ViewContext::Branches,
            CommandId::SwitchField => env.input == Some(InputMode::CommitEditor),
            _ => true,
        }
    }
}

fn resolve_key_text(
    key: &str,
    keymap: &Keymap,
    env: &CommandEnv<'_>,
) -> Option<&'static CommandSpec> {
    COMMANDS
        .iter()
        .find(|spec| spec.matches_key(key, keymap, env) && spec.action(env).is_some())
}

pub fn resolve_key(
    key: &KeyEvent,
    keymap: &Keymap,
    env: &CommandEnv<'_>,
) -> Option<&'static CommandSpec> {
    resolve_key_text(&key_to_string(key), keymap, env)
}

pub fn normalize_input(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub fn resolve_palette(input: &str, env: &CommandEnv<'_>) -> Option<DomainAction> {
    let normalized = normalize_input(input);
    COMMANDS
        .iter()
        .find(|spec| spec.aliases.contains(&normalized.as_str()))?
        .action(env)
}

#[derive(Debug, Clone)]
pub struct CommandHelp {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub label: String,
    pub keys: Vec<String>,
}

fn active_keys(spec: &CommandSpec, keymap: &Keymap, env: &CommandEnv<'_>) -> Vec<String> {
    spec.keys(keymap, env)
        .into_iter()
        .filter(|key| {
            resolve_key_text(key, keymap, env).is_some_and(|resolved| resolved.id == spec.id)
        })
        .map(str::to_owned)
        .collect()
}

pub fn available_commands(keymap: &Keymap, env: &CommandEnv<'_>) -> Vec<CommandHelp> {
    let mut available: Vec<_> = COMMANDS
        .iter()
        .filter(|spec| !spec.aliases.is_empty() && spec.action(env).is_some())
        .collect();
    available.sort_by_key(|spec| spec.footer_rank.unwrap_or(500));
    available
        .into_iter()
        .map(|spec| CommandHelp {
            name: spec.aliases[0],
            aliases: spec.aliases,
            label: spec.label(env),
            keys: active_keys(spec, keymap, env),
        })
        .collect()
}

pub fn shortcut_hints(keymap: &Keymap, env: &CommandEnv<'_>) -> Vec<(String, String)> {
    let mut hints: Vec<_> = COMMANDS
        .iter()
        .filter(|spec| {
            spec.footer_rank.is_some() && spec.visible_in_footer(env) && spec.action(env).is_some()
        })
        .filter_map(|spec| {
            active_keys(spec, keymap, env)
                .into_iter()
                .next()
                .map(|key| (spec.footer_rank.unwrap(), key, spec.label(env)))
        })
        .collect();
    hints.sort_by_key(|(rank, _, _)| *rank);
    hints
        .into_iter()
        .map(|(_, key, label)| (key, label))
        .collect()
}

pub fn default_bindings(panel: Option<SidePanel>) -> HashMap<String, Vec<String>> {
    let mut result = HashMap::new();
    for spec in COMMANDS {
        let binding = match spec.binding {
            Binding::Global(name, keys) if panel.is_none() => Some((name, keys)),
            Binding::Panel(expected, name, keys) if Some(expected) == panel => Some((name, keys)),
            _ => None,
        };
        if let Some((name, keys)) = binding {
            result.insert(name.into(), keys.iter().map(|key| (*key).into()).collect());
        }
    }
    if matches!(panel, Some(SidePanel::Commits | SidePanel::Stash)) {
        // One contextual command uses the same binding ID in three windows.
        let spec = COMMANDS
            .iter()
            .find(|spec| spec.id == CommandId::OpenRevision)
            .unwrap();
        if let Binding::Panel(_, name, keys) = spec.binding {
            result.insert(name.into(), keys.iter().map(|key| (*key).into()).collect());
        }
    }
    result
}

impl AppState {
    pub fn command_env(&self) -> CommandEnv<'_> {
        CommandEnv {
            diff: self.hunk_view(),
            selection: self.selection(),
            input: self.input.mode,
            commit_focus: self.input.commit_focus,
            has_search: self.has_search_for_active_scope(),
            has_search_query: self.has_search_query_for_active_scope(),
        }
    }

    pub fn available_commands(&self) -> Vec<CommandHelp> {
        let mut env = self.command_env();
        env.input = None;
        available_commands(self.keymap(), &env)
    }
}
