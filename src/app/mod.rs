#![allow(clippy::module_inception)]

mod app;
mod app_effects;
mod app_effects_impl;
mod background_poll;
mod background_task_runner;
pub(crate) mod branch_panel_adapter;
mod command;
mod diff_cache;
mod diff_cache_manager;
mod diff_loader;
mod dirty_flags;
pub(crate) mod files_panel_adapter;
pub mod graph_highlight;
mod hints;
mod input_mode;
mod panel_nav;
mod refresh_scheduler;
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
pub use app_effects::AppEffects;
pub use command::Command;
pub use states::{
    BranchesPanelState, CommitsPanelState, FilesPanelState, RenderCache, SidePanel, StashPanelState,
};
#[cfg(test)]
pub use test_dispatch::{dispatch_test_action, dispatch_test_key, map_test_key};

#[cfg(test)]
#[path = "update_tests.rs"]
mod update_tests;
