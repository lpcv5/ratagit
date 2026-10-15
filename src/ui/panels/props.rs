//! Controlled display inputs. Components never receive a dispatcher or mutable
//! application state; keyboard bindings are resolved by the application owner.
use crate::app::commands::CommandHelp;
use crate::app::help::HelpEntry;
use crate::app::{CommandLogEntry, CommitFieldFocus};

pub struct CommitEditorProps<'a> {
    pub error: Option<&'a str>,
    pub focus: CommitFieldFocus,
    pub message: &'a str,
    pub description: &'a str,
    pub hints: &'a [(String, String)],
}

pub struct StashEditorProps<'a> {
    pub error: Option<&'a str>,
    pub message: &'a str,
    pub targets: &'a [String],
}

pub struct BranchEditorProps<'a> {
    pub error: Option<&'a str>,
    pub name: &'a str,
}

pub struct BranchSwitchConfirmProps<'a> {
    pub target: &'a str,
    pub change_count: usize,
}

pub struct CommitAllConfirmProps {
    pub change_count: usize,
}

pub struct CommandPaletteProps<'a> {
    pub input: &'a str,
    pub selected: usize,
    /// Already filtered by the owner using command execution normalization.
    pub commands: Vec<&'a CommandHelp>,
    pub hints: &'a [(String, String)],
}

pub struct KeybindingsProps<'a> {
    pub context: &'a str,
    pub query: &'a str,
    pub filtering: bool,
    pub selected: usize,
    /// The same filtered order used by navigation and command execution.
    pub entries: Vec<&'a HelpEntry>,
}

pub struct CommandLogProps<'a> {
    pub entries: &'a [CommandLogEntry],
}

pub struct ShortcutBarProps<'a> {
    pub hints: &'a [(String, String)],
    pub search_input: Option<&'a str>,
    pub loading_frame: Option<usize>,
}
