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
pub use app::BranchesPanelState;
pub use app::CommitFieldFocus;
pub use app::CommitsPanelState;
pub use app::FilesPanelState;
pub use app::InputMode;
pub use app::RefreshKind;
pub use app::RenderCache;
pub use app::SearchScopeKey;
pub use app::SidePanel;
pub use app::StashPanelState;
pub use command::Command;
pub use states::{GitState, InputState, UiState};
#[cfg(test)]
pub use test_dispatch::{dispatch_test_action, dispatch_test_key, map_test_key};

#[cfg(test)]
#[path = "update_tests.rs"]
mod update_tests;
