#![allow(clippy::module_inception)]

mod app;
mod command;
mod diff_cache;
mod diff_loader;
mod dirty_flags;
pub mod graph_highlight;
mod hints;
mod input_mode;
mod panel_nav;
mod refresh;
mod revision_tree;
mod search;
mod selection;
mod selectors;
mod state_access_impl;
mod states;
#[cfg(test)]
mod test_dispatch;
#[cfg(test)]
mod trace;

pub use app::App;
pub use app::CommitFieldFocus;
pub use app::InputMode;
pub use app::RefreshKind;
pub use app::SearchScopeKey;
pub use command::Command;
pub use states::{
    BranchesPanelState, CommandLogEntry, CommitsPanelState, FilesPanelState, GitState, InputState,
    PanelState, RenderCache, SidePanel, StashPanelState, TreeModeState, UiState,
};
#[cfg(test)]
pub use test_dispatch::{dispatch_test_action, dispatch_test_key, map_test_key};

#[cfg(test)]
#[path = "update_tests.rs"]
mod update_tests;
