#![allow(clippy::module_inception)]

mod app;
mod command;
mod diff_cache;
mod diff_loader;
mod dirty_flags;
pub mod graph_highlight;
mod hints;
mod input_mode;
mod message;
mod panel_nav;
mod refresh;
mod revision_tree;
mod search;
mod selection;
mod selectors;
mod update;
mod update_handlers;

pub use app::App;
pub use app::BranchesPanelState;
pub use app::CommitFieldFocus;
pub use app::CommitsPanelState;
pub use app::FilesPanelState;
pub use app::InputMode;
pub use app::RefreshKind;
pub use app::SearchScopeKey;
pub use app::SidePanel;
pub use app::StashPanelState;
pub use command::Command;
pub use message::GlobalMessage;
pub use message::Message;
pub use update::update;
