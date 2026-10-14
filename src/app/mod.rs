#![allow(clippy::module_inception)]

mod app;
mod background;
mod background_poll;
mod background_task_runner;
mod command;
#[cfg(test)]
mod command_tests;
pub mod commands;
mod context;
pub(crate) mod diff;
mod diff_coordinator;
pub use diff::{DiffInteraction, DiffSide, DiffState, HunkView};
mod dirty_flags;
#[cfg(test)]
mod file_tree_tests;
pub(crate) mod files;
mod files_coordinator;
pub mod graph_highlight;
mod hints;
#[cfg(test)]
mod hunk_tests;
mod input_mode;
#[cfg(test)]
mod legacy_test_helpers;
mod panel_nav;
mod refresh_scheduler;
mod repository_operation;
mod revision_tree;
mod runtime;
mod search;
#[cfg(test)]
mod selection_tests;
mod selectors;
mod state_access_impl;
mod states;
#[cfg(test)]
mod test_dispatch;
#[cfg(test)]
mod trace;

pub use app::App;
pub use app::AppState;
pub use app::CommitFieldFocus;
pub use app::InputMode;
pub use app::RefreshKind;
pub use app::SearchScopeKey;
pub use background::{BackgroundCompleted, BackgroundLoad, BackgroundPayload};
pub use command::Command;
pub use context::{UiContext, ViewContext};
pub use files::{FileSelection, FilesPanelState, FilesState};
pub use runtime::AppRuntime;
pub use selectors::SelectionView;
pub use states::{BranchesPanelState, CommitsPanelState, RenderCache, SidePanel, StashPanelState};
#[cfg(test)]
pub use test_dispatch::{dispatch_test_action, dispatch_test_key, map_test_key};

#[cfg(test)]
#[path = "update_tests.rs"]
mod update_tests;
