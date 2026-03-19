pub mod files_panel;
pub mod branches_panel;
pub mod commits_panel;
pub mod stash_panel;
pub mod diff_panel;
pub mod command_log;
pub mod shortcut_bar;

pub use files_panel::render_files_panel;
pub use branches_panel::render_branches_panel;
pub use commits_panel::render_commits_panel;
pub use stash_panel::render_stash_panel;
pub use diff_panel::render_diff_panel;
pub use command_log::render_command_log;
pub use shortcut_bar::render_shortcut_bar;
