use crate::app::{App, CommitFieldFocus, InputMode, RefreshKind, SidePanel};
use crate::flux::action::DomainAction;
use crate::flux::stores::state_access::{
    CoreAccess, DirtyHint, InputAccess, NavigationAccess, OverlayAccess, RefreshAccess,
    RevisionAccess, SearchAccess, SelectionAccess,
};
use crate::git::DiffLine;
use std::path::PathBuf;
use std::time::Duration;

// ---------------------------------------------------------------------------
// CoreAccess
// ---------------------------------------------------------------------------

impl CoreAccess for App {
    fn push_log(&mut self, msg: String, success: bool) {
        App::push_log(self, msg, success);
    }

    fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    fn mark_all_dirty(&mut self) {
        self.ui.dirty.mark_all();
    }

    fn mark_dirty(&mut self, hint: DirtyHint) {
        self.ui.dirty.apply_hint(hint);
    }

    fn resolve_command_palette_command(&self, cmd: &str) -> Option<DomainAction> {
        App::resolve_command_palette_command(self, cmd)
    }

    fn selected_branch_name(&self) -> Option<String> {
        App::selected_branch_name(self)
    }

    fn selected_stash_index(&self) -> Option<usize> {
        App::selected_stash_index(self)
    }

    fn take_branch_switch_target(&mut self) -> Option<String> {
        App::take_branch_switch_target(self)
    }

    fn is_fetching_remote(&self) -> bool {
        self.ui.branches.is_fetching_remote
    }

    fn set_fetching_remote(&mut self, fetching: bool) {
        self.ui.branches.is_fetching_remote = fetching;
    }

    fn active_panel(&self) -> SidePanel {
        self.ui.active_panel
    }

    fn set_active_panel(&mut self, panel: SidePanel) {
        self.ui.active_panel = panel;
    }

    fn has_uncommitted_changes(&self) -> bool {
        App::has_uncommitted_changes(self)
    }

    fn set_current_diff(&mut self, lines: Vec<DiffLine>) {
        self.git.detail.panel.request = self.current_detail_request();
        self.git.current_diff = lines.clone();
        self.git.detail.panel.lines = lines;
        self.git.detail.panel.is_loading = false;
    }
}

// ---------------------------------------------------------------------------
// InputAccess
// ---------------------------------------------------------------------------

impl InputAccess for App {
    fn cancel_input(&mut self) {
        App::cancel_input(self);
    }

    fn input_mode(&self) -> Option<InputMode> {
        self.input.mode
    }

    fn set_input_mode(&mut self, mode: Option<InputMode>) {
        self.input.mode = mode;
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

    fn push_commit_message_char(&mut self, c: char) {
        self.input.commit_message_buffer.push(c);
    }

    fn push_commit_description_char(&mut self, c: char) {
        self.input.commit_description_buffer.push(c);
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

    fn input_buffer(&self) -> &str {
        &self.input.buffer
    }

    fn input_buffer_clone(&self) -> String {
        self.input.buffer.clone()
    }

    fn clear_input_buffer(&mut self) {
        self.input.buffer.clear();
    }

    fn push_input_buffer_char(&mut self, c: char) {
        self.input.buffer.push(c);
    }

    fn pop_input_buffer_char(&mut self) {
        self.input.buffer.pop();
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

    fn push_stash_message_char(&mut self, c: char) {
        self.input.stash_message_buffer.push(c);
    }

    fn pop_stash_message_char(&mut self) {
        self.input.stash_message_buffer.pop();
    }
}

// ---------------------------------------------------------------------------
// NavigationAccess
// ---------------------------------------------------------------------------

impl NavigationAccess for App {
    fn list_down(&mut self) {
        App::list_down(self);
    }

    fn list_up(&mut self) {
        App::list_up(self);
    }

    fn diff_scroll_up(&mut self) {
        App::diff_scroll_up(self);
    }

    fn diff_scroll_down(&mut self) {
        App::diff_scroll_down(self);
    }
}

// ---------------------------------------------------------------------------
// SearchAccess
// ---------------------------------------------------------------------------

impl SearchAccess for App {
    fn clear_search(&mut self) {
        App::clear_search(self);
    }

    fn confirm_search_input(&mut self) {
        App::confirm_search_input(self);
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

    fn search_query_clone(&self) -> String {
        self.input.search_query.clone()
    }
}

// ---------------------------------------------------------------------------
// SelectionAccess
// ---------------------------------------------------------------------------

impl SelectionAccess for App {
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

    fn all_file_paths(&self) -> Vec<PathBuf> {
        self.git
            .status
            .staged
            .iter()
            .chain(self.git.status.unstaged.iter())
            .chain(self.git.status.untracked.iter())
            .map(|e| e.path.clone())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// RefreshAccess
// ---------------------------------------------------------------------------

impl RefreshAccess for App {
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
}

// ---------------------------------------------------------------------------
// OverlayAccess
// ---------------------------------------------------------------------------

impl OverlayAccess for App {
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

    fn start_commit_editor_guarded(&mut self) -> bool {
        App::start_commit_editor_guarded(self)
    }

    fn start_branch_switch_confirm(&mut self, target: String) {
        App::start_branch_switch_confirm(self, target);
    }

    fn start_commit_editor(&mut self) {
        App::start_commit_editor(self);
    }
}

// ---------------------------------------------------------------------------
// RevisionAccess
// ---------------------------------------------------------------------------

impl RevisionAccess for App {
    fn close_branch_commits_subview(&mut self) {
        App::close_branch_commits_subview(self);
    }
}
