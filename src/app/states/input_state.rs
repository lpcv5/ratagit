use crate::app::{CommitFieldFocus, InputMode, SearchScopeKey, SidePanel};
use std::collections::HashMap;
use std::path::PathBuf;

/// Input state - all user input buffers and modes
#[derive(Clone)]
pub struct InputState {
    pub mode: Option<InputMode>,
    pub buffer: String,
    pub commit_message_buffer: String,
    pub commit_description_buffer: String,
    pub commit_focus: CommitFieldFocus,
    pub stash_message_buffer: String,
    pub stash_targets: Vec<PathBuf>,
    pub branch_switch_target: Option<String>,
    pub search_query: String,
    pub(crate) search_matches: Vec<usize>,
    pub(crate) search_scope: SearchScopeKey,
    pub(crate) search_queries: HashMap<SearchScopeKey, String>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            mode: None,
            buffer: String::new(),
            commit_message_buffer: String::new(),
            commit_description_buffer: String::new(),
            commit_focus: CommitFieldFocus::Message,
            stash_message_buffer: String::new(),
            stash_targets: Vec::new(),
            branch_switch_target: None,
            search_query: String::new(),
            search_matches: Vec::new(),
            search_scope: SearchScopeKey {
                panel: SidePanel::Files,
                commit_tree_mode: false,
                stash_tree_mode: false,
            },
            search_queries: HashMap::new(),
        }
    }
}
