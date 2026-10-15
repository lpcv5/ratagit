//! Contextual keybinding menu. State transitions are pure and use the same
//! effective bindings and command resolution as keyboard input and the palette.
use super::{commands, AppState, InputMode, ViewContext};
use crate::config::keymap::{key_to_string, Keymap};
use crate::flux::action::DomainAction;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HelpGroup {
    Local,
    Global,
    Custom,
}

impl HelpGroup {
    pub const ALL: [Self; 3] = [Self::Local, Self::Global, Self::Custom];

    pub fn title(self) -> &'static str {
        match self {
            Self::Local => "Local",
            Self::Global => "Global",
            Self::Custom => "Custom",
        }
    }
}

#[derive(Debug, Clone)]
pub struct HelpEntry {
    pub name: &'static str,
    pub description: String,
    pub keys: Vec<String>,
    pub group: HelpGroup,
}

#[derive(Debug, Clone, Default)]
pub struct HelpState {
    pub entries: crate::shared::Shared<Vec<HelpEntry>>,
    pub context: String,
    pub query: String,
    pub filtering: bool,
    pub selected: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpAction {
    Open,
    Close,
    Back,
    Move(isize),
    First,
    Last,
    Filter,
    Backspace,
    Char(char),
    Execute,
}

impl HelpState {
    pub fn filtered_entries(&self) -> Vec<&HelpEntry> {
        let query = self.query.to_lowercase();
        self.entries
            .iter()
            .filter(|entry| {
                entry.description.to_lowercase().contains(&query)
                    || entry.name.contains(&query)
                    || entry
                        .keys
                        .iter()
                        .any(|key| key.to_lowercase().contains(&query))
            })
            .collect()
    }

    pub fn map_key(&self, key: KeyEvent, keymap: &Keymap) -> Option<HelpAction> {
        use HelpAction::*;
        if key.code == KeyCode::Esc {
            return Some(Back);
        }
        if key.code == KeyCode::Enter {
            return Some(Execute);
        }
        if self.filtering {
            match key.code {
                KeyCode::Backspace => return Some(Backspace),
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    return Some(Char(c));
                }
                _ => {}
            }
        }
        let text = key_to_string(&key);
        let actions = keymap.global_actions(&text);
        if !self.filtering && actions.iter().any(|name| name == "help" || name == "quit") {
            return Some(Close);
        }
        match key.code {
            KeyCode::Up => Some(Move(-1)),
            KeyCode::Down => Some(Move(1)),
            KeyCode::PageUp => Some(Move(-10)),
            KeyCode::PageDown => Some(Move(10)),
            KeyCode::Home => Some(First),
            KeyCode::End => Some(Last),
            KeyCode::Char('/') if !self.filtering => Some(Filter),
            KeyCode::Char(' ') if !self.filtering => Some(Execute),
            _ if actions.iter().any(|name| name == "list_up") => Some(Move(-1)),
            _ if actions.iter().any(|name| name == "list_down") => Some(Move(1)),
            _ if actions.iter().any(|name| name == "diff_scroll_up") => Some(Move(-10)),
            _ if actions.iter().any(|name| name == "diff_scroll_down") => Some(Move(10)),
            _ => None,
        }
    }
}

impl AppState {
    pub(crate) fn reduce_help(&mut self, action: HelpAction) -> Option<DomainAction> {
        use HelpAction::*;
        if action == Open {
            if self.input.mode.is_some() {
                return None;
            }
            let env = self.command_env();
            let context = if env.diff.interaction.focused {
                if env.diff.interaction.line_mode {
                    "Diff / Lines"
                } else {
                    "Diff / Hunks"
                }
            } else {
                match env.view() {
                    ViewContext::Files => "Files",
                    ViewContext::Branches => "Branches",
                    ViewContext::BranchCommits { .. } => "Branches / Commits",
                    ViewContext::Commits => "Commits",
                    ViewContext::CommitFiles { .. } => "Commits / Files",
                    ViewContext::Stashes => "Stashes",
                    ViewContext::StashFiles { .. } => "Stashes / Files",
                }
            };
            self.input.help = HelpState {
                entries: commands::keybindings(self.keymap(), &env).into(),
                context: context.into(),
                ..HelpState::default()
            };
            self.input.mode = Some(InputMode::Keybindings);
            return None;
        }
        if self.input.mode != Some(InputMode::Keybindings) {
            return None;
        }
        let help = &mut self.input.help;
        match action {
            Open => unreachable!(),
            Back if help.filtering => {
                help.filtering = false;
                help.query.clear();
                help.selected = 0;
            }
            Close | Back => self.input.mode = None,
            Move(delta) => {
                let last = help.filtered_entries().len().saturating_sub(1);
                help.selected = help.selected.saturating_add_signed(delta).min(last);
            }
            First => help.selected = 0,
            Last => help.selected = help.filtered_entries().len().saturating_sub(1),
            Filter => help.filtering = true,
            Backspace => {
                help.query.pop();
                help.selected = 0;
            }
            Char(c) => {
                help.query.push(c);
                help.selected = 0;
            }
            Execute => {
                let name = help
                    .filtered_entries()
                    .get(help.selected)
                    .map(|entry| entry.name);
                if let Some(name) = name {
                    self.input.mode = None;
                    // Re-resolve against current state: background updates may have
                    // changed the selected file or made a command unavailable.
                    return commands::resolve_palette(name, &self.command_env());
                }
            }
        }
        None
    }
}

pub(super) fn description(id: commands::CommandId, label: &str) -> String {
    use commands::CommandId::*;
    let text = match id {
        FocusDiff => "Open diff / select hunks",
        DiffSide => "Switch staged / unstaged diff",
        DiffApply => return format!("{label} selected lines / hunks"),
        DiffNextLine if label == "NextHunk" => "Next hunk",
        DiffNextLine => "Next line",
        DiffPreviousLine if label == "PreviousHunk" => "Previous hunk",
        DiffPreviousLine => "Previous line",
        DiffClear => "Clear selection",
        DiffNextFile => "Next file",
        DiffPreviousFile => "Previous file",
        HunkNext => "Next hunk",
        HunkPrevious => "Previous hunk",
        HunkSelect => "Toggle selected line / hunk",
        HunkStage => "Stage selected lines / hunks",
        HunkUnstage => "Unstage selected lines / hunks",
        DiffLineMode => {
            if label == "HunkMode" {
                "Switch to hunk mode"
            } else {
                "Switch to line mode"
            }
        }
        DiffLineRange => "Select a range",
        DiffDiscard => {
            if label == "UnstageSelection" {
                "Unstage selection"
            } else {
                "Discard selection"
            }
        }
        Back => {
            if label == "BackToList" {
                "Return to file list"
            } else if label == "ClearSearch" {
                "Clear search"
            } else if label == "ExitVisual" {
                "Exit visual selection"
            } else {
                "Return to parent view"
            }
        }
        Confirm => "Confirm",
        SwitchField => "Switch input field",
        Backspace => "Delete previous character",
        Quit => "Quit",
        ListUp => "Move selection up",
        ListDown => "Move selection down",
        PanelNext => "Next side panel",
        PanelPrev => "Previous side panel",
        DiffUp => "Scroll diff up",
        DiffDown => "Scroll diff down",
        Files => "Go to Files",
        Branches => "Go to Branches",
        Commits => "Go to Commits",
        Stashes => "Go to Stashes",
        Commit => {
            if label == "CommitSelection" {
                "Commit selected files"
            } else {
                "Commit changes"
            }
        }
        Palette => "Open command palette",
        Help => "Open keybindings help",
        Search => "Search current list",
        SearchNext => "Next search match",
        SearchPrev => "Previous search match",
        Refresh => "Refresh repository",
        ToggleStage => {
            if label == "Unstage" {
                "Unstage file"
            } else if label == "ToggleStage" {
                "Stage / unstage selected files"
            } else {
                "Stage file"
            }
        }
        StageAll => "Stage all / unstage all",
        Amend => "Amend last commit",
        EditFile => "Edit file",
        CopyPath => "Copy relative path",
        IgnoreFile => "Add to .gitignore",
        Discard => "Discard changes",
        StashPush => "Stash selected files",
        Visual => "Toggle visual selection",
        ToggleDir => "Expand / collapse directory",
        Collapse => "Collapse all directories",
        Expand => "Expand all directories",
        Checkout => "Checkout branch",
        CreateBranch => "Create branch",
        DeleteBranch => "Delete branch",
        Fetch => "Fetch remote",
        OpenRevision => match label {
            "Commits" => "Show branch commits",
            "Refresh" => "Refresh branch commits",
            "ToggleDir" => "Expand / collapse directory",
            _ => "Show revision files",
        },
        StashApply => "Apply stash",
        StashPop => "Pop stash",
        StashDrop => "Drop stash",
    };
    text.into()
}
