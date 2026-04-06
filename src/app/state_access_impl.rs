use crate::app::{App, CommitFieldFocus, InputMode, RefreshKind, SearchScopeKey, SidePanel};
use crate::flux::action::DomainAction;
use crate::flux::stores::{DirtyHint, StateAccess};
use crate::git::{CommitInfo, FileEntry, StashInfo};
use std::path::PathBuf;
use std::time::Duration;

impl StateAccess for App {
    fn push_log(&mut self, msg: String, success: bool) {
        App::push_log(self, msg, success);
    }

    fn cancel_input(&mut self) {
        App::cancel_input(self);
    }

    fn clear_search(&mut self) {
        App::clear_search(self);
    }

    fn confirm_search_input(&mut self) {
        App::confirm_search_input(self);
    }

    fn request_refresh(&mut self, kind: RefreshKind) {
        App::request_refresh(self, kind);
    }

    fn has_pending_diff_reload(&self) -> bool {
        App::has_pending_diff_reload(self)
    }

    fn diff_reload_debounce_elapsed(&self, debounce: Duration) -> bool {
        App::diff_reload_debounce_elapsed(self, debounce)
    }

    fn schedule_diff_reload(&mut self) {
        App::schedule_diff_reload(self);
    }

    fn take_branch_switch_target(&mut self) -> Option<String> {
        App::take_branch_switch_target(self)
    }

    fn selected_branch_name(&self) -> Option<String> {
        App::selected_branch_name(self)
    }

    fn selected_file_entry(&self) -> Option<&FileEntry> {
        if self.ui.active_panel != SidePanel::Files {
            return None;
        }
        let idx = self.ui.files.panel.list_state.selected()?;
        let node = self.ui.files.tree_nodes.get(idx)?;
        if node.is_dir {
            return None;
        }
        self.git
            .status
            .staged
            .iter()
            .chain(self.git.status.unstaged.iter())
            .chain(self.git.status.untracked.iter())
            .find(|e| e.path == node.path)
    }

    fn selected_commit(&self) -> Option<&CommitInfo> {
        if self.ui.active_panel != SidePanel::Commits {
            return None;
        }
        let idx = self.ui.commits.panel.list_state.selected()?;
        self.ui.commits.items.get(idx)
    }

    fn selected_stash(&self) -> Option<&StashInfo> {
        if self.ui.active_panel != SidePanel::Stash {
            return None;
        }
        let idx = self.ui.stash.panel.list_state.selected()?;
        self.ui.stash.items.get(idx)
    }

    fn selected_stash_index(&self) -> Option<usize> {
        App::selected_stash_index(self)
    }

    fn commit_focus(&self) -> CommitFieldFocus {
        self.input.commit_focus
    }

    fn set_commit_focus(&mut self, focus: CommitFieldFocus) {
        self.input.commit_focus = focus;
    }

    fn commit_message_buffer(&self) -> &str {
        &self.input.commit_message_buffer
    }

    fn commit_description_buffer(&self) -> &str {
        &self.input.commit_description_buffer
    }

    fn clear_commit_buffers(&mut self) {
        self.input.commit_message_buffer.clear();
        self.input.commit_description_buffer.clear();
        self.input.commit_focus = CommitFieldFocus::Message;
    }

    fn input_buffer(&self) -> &str {
        &self.input.buffer
    }

    fn clear_input_buffer(&mut self) {
        self.input.buffer.clear();
    }

    fn stash_message_buffer(&self) -> &str {
        &self.input.stash_message_buffer
    }

    fn stash_targets(&self) -> &[PathBuf] {
        &self.input.stash_targets
    }

    fn clear_stash_buffers(&mut self) {
        self.input.stash_message_buffer.clear();
        self.input.stash_targets.clear();
    }

    fn set_input_mode(&mut self, mode: Option<InputMode>) {
        self.input.mode = mode;
    }

    fn input_mode(&self) -> Option<InputMode> {
        self.input.mode
    }

    fn resolve_command_palette_command(&self, cmd: &str) -> Option<DomainAction> {
        App::resolve_command_palette_command(self, cmd)
    }

    fn active_panel(&self) -> SidePanel {
        self.ui.active_panel
    }

    fn set_active_panel(&mut self, panel: SidePanel) {
        self.ui.active_panel = panel;
    }

    fn is_fetching_remote(&self) -> bool {
        self.ui.branches.is_fetching_remote
    }

    fn set_fetching_remote(&mut self, fetching: bool) {
        self.ui.branches.is_fetching_remote = fetching;
    }

    fn search_scope(&self) -> SearchScopeKey {
        self.input.search_scope
    }

    fn search_query(&self) -> &str {
        &self.input.search_query
    }

    fn apply_search_query(&mut self, query: String) -> usize {
        App::apply_search_query(self, query)
    }

    fn search_select_initial_match(&mut self) -> bool {
        App::search_select_initial_match(self)
    }

    fn search_jump_next(&mut self) -> bool {
        App::search_jump_next(self)
    }

    fn search_jump_prev(&mut self) -> bool {
        App::search_jump_prev(self)
    }

    fn restore_search_for_active_scope(&mut self) {
        App::restore_search_for_active_scope(self);
    }

    fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    fn mark_all_dirty(&mut self) {
        self.ui.dirty.mark_all();
    }

    fn mark_dirty(&mut self, hint: DirtyHint) {
        let bits = hint.0;
        if bits == 0 {
            return;
        }
        if bits & DirtyHint::MAIN_CONTENT != 0 {
            self.ui.dirty.left_panels = true;
        }
        if bits & DirtyHint::DIFF != 0 {
            self.ui.dirty.diff = true;
        }
        if bits & DirtyHint::COMMAND_LOG != 0 {
            self.ui.dirty.command_log = true;
        }
        if bits & DirtyHint::SHORTCUT_BAR != 0 {
            self.ui.dirty.shortcut_bar = true;
        }
        if bits & DirtyHint::OVERLAY != 0 {
            self.ui.dirty.overlay = true;
        }
    }

    fn list_down(&mut self) {
        App::list_down(self);
    }

    fn list_up(&mut self) {
        App::list_up(self);
    }

    fn toggle_selected_dir(&mut self) {
        App::toggle_selected_dir(self);
    }

    fn collapse_all(&mut self) {
        App::collapse_all(self);
    }

    fn expand_all(&mut self) {
        App::expand_all(self);
    }

    fn diff_scroll_up(&mut self) {
        App::diff_scroll_up(self);
    }

    fn diff_scroll_down(&mut self) {
        App::diff_scroll_down(self);
    }

    fn recompute_commit_highlight(&mut self) {
        App::recompute_commit_highlight(self);
    }

    fn start_command_palette(&mut self) {
        App::start_command_palette(self);
    }

    fn start_branch_create_input(&mut self) {
        App::start_branch_create_input(self);
    }

    fn start_search_input(&mut self) {
        App::start_search_input(self);
    }

    fn start_stash_editor(&mut self, targets: Vec<PathBuf>) {
        App::start_stash_editor(self, targets);
    }

    fn toggle_visual_select_mode(&mut self) {
        App::toggle_visual_select_mode(self);
    }

    fn prepare_stash_targets_from_selection(&self) -> Vec<PathBuf> {
        App::prepare_stash_targets_from_selection(self)
    }

    fn prepare_discard_targets_from_selection(&self) -> Vec<PathBuf> {
        App::prepare_discard_targets_from_selection(self)
    }

    fn clear_files_visual_selection(&mut self) {
        self.ui.files.visual_mode = false;
        self.ui.files.visual_anchor = None;
    }

    fn stash_close_tree(&mut self) {
        App::stash_close_tree(self);
    }

    fn commit_close_tree(&mut self) {
        App::commit_close_tree(self);
    }

    fn close_branch_commits_subview(&mut self) {
        App::close_branch_commits_subview(self);
    }

    fn push_newline_to_commit_description(&mut self) {
        self.input.commit_description_buffer.push('\n');
    }

    fn pop_commit_message_char(&mut self) {
        self.input.commit_message_buffer.pop();
    }

    fn pop_commit_description_char(&mut self) {
        self.input.commit_description_buffer.pop();
    }

    fn pop_input_buffer_char(&mut self) {
        self.input.buffer.pop();
    }

    fn pop_stash_message_char(&mut self) {
        self.input.stash_message_buffer.pop();
    }

    fn push_commit_message_char(&mut self, c: char) {
        self.input.commit_message_buffer.push(c);
    }

    fn push_commit_description_char(&mut self, c: char) {
        self.input.commit_description_buffer.push(c);
    }

    fn push_input_buffer_char(&mut self, c: char) {
        self.input.buffer.push(c);
    }

    fn push_stash_message_char(&mut self, c: char) {
        self.input.stash_message_buffer.push(c);
    }

    fn input_buffer_clone(&self) -> String {
        self.input.buffer.clone()
    }

    fn search_query_clone(&self) -> String {
        self.input.search_query.clone()
    }
}
