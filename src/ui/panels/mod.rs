pub mod branch_editor;
pub mod branch_switch_confirm;
pub mod command_log;
pub mod command_palette;
pub mod commit_all_confirm;
pub mod commit_editor;
pub mod diff_panel;
pub mod keybindings;
pub mod overlay_utils;
pub mod props;
pub mod revision_tree_panel;
pub(crate) mod selection;
pub mod shortcut_bar;
pub mod stash_editor;

#[cfg(test)]
mod props_tests;

pub use branch_editor::render_branch_editor;
pub use branch_switch_confirm::render_branch_switch_confirm;
pub use command_log::render_command_log;
pub use command_palette::render_command_palette;
pub use commit_all_confirm::render_commit_all_confirm;
pub use commit_editor::render_commit_editor;
pub use diff_panel::render_diff_panel;
pub use keybindings::render_keybindings;
pub use overlay_utils::render_overlay_chrome;
pub use shortcut_bar::render_shortcut_bar;
pub use stash_editor::render_stash_editor;
