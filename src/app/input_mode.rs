use crate::app::{App, CommitFieldFocus, InputMode, RefreshKind};
use std::path::PathBuf;

impl App {
    pub fn start_commit_editor(&mut self) {
        self.input_mode = Some(InputMode::CommitEditor);
        self.commit_message_buffer.clear();
        self.commit_description_buffer.clear();
        self.commit_focus = CommitFieldFocus::Message;
    }

    pub fn start_commit_editor_guarded(&mut self) -> bool {
        if self.status.staged.is_empty() {
            if self.pending_refresh_kind().is_some() {
                self.request_refresh(RefreshKind::StatusOnly);
                if let Err(e) = self.flush_pending_refresh() {
                    self.push_log(format!("refresh failed: {}", e), false);
                    return false;
                }
            }
            if self.status.staged.is_empty() {
                self.push_log("nothing staged to commit", false);
                return false;
            }
        }
        self.start_commit_editor();
        true
    }

    pub fn start_branch_create_input(&mut self) {
        self.input_mode = Some(InputMode::CreateBranch);
        self.input_buffer.clear();
    }

    pub fn start_command_palette(&mut self) {
        self.input_mode = Some(InputMode::CommandPalette);
        self.input_buffer.clear();
    }

    pub fn start_stash_editor(&mut self, targets: Vec<PathBuf>) {
        self.input_mode = Some(InputMode::StashEditor);
        self.stash_targets = targets;
        self.stash_message_buffer.clear();
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = None;
        self.input_buffer.clear();
        self.commit_message_buffer.clear();
        self.commit_description_buffer.clear();
        self.commit_focus = CommitFieldFocus::Message;
        self.stash_message_buffer.clear();
        self.stash_targets.clear();
        self.branch_switch_target = None;
    }

    pub(crate) fn resolve_command_palette_command(
        &self,
        input: &str,
    ) -> Option<crate::flux::action::DomainAction> {
        use crate::flux::action::DomainAction;
        let normalized = input.trim().to_lowercase();
        match normalized.as_str() {
            "q" | "quit" | "exit" => Some(DomainAction::Quit),
            "r" | "refresh" => Some(DomainAction::RefreshStatus),
            "c" | "commit" => Some(DomainAction::StartCommitInput),
            "/" | "search" => Some(DomainAction::StartSearchInput),
            "stash" | "stash push" => Some(DomainAction::StartStashInput),
            "branch" | "branch create" | "branch new" => Some(DomainAction::StartBranchCreateInput),
            "fetch" | "fetch remote" => Some(DomainAction::FetchRemote),
            "panel 1" | "panel files" | "files" => Some(DomainAction::PanelGoto(1)),
            "panel 2" | "panel branches" | "branches" => Some(DomainAction::PanelGoto(2)),
            "panel 3" | "panel commits" | "commits" => Some(DomainAction::PanelGoto(3)),
            "panel 4" | "panel stash" | "stash panel" => Some(DomainAction::PanelGoto(4)),
            _ => None,
        }
    }
}
