mod branch_modal;
mod discard_modal;
mod editor_modal;
mod frame;
mod layout;
mod loading_indicator;
mod modal;
mod panels;
mod reset_modal;
mod sync_modal;
mod terminal;
mod text;
mod theme;

pub use frame::{
    RenderContext, RenderedFrame, TerminalBuffer, TerminalCursor, TerminalSize,
    buffer_contains_batch_selected_text, buffer_contains_selected_text,
    buffer_contains_text_with_style, buffer_to_text_with_selected_marker,
};
pub use layout::{
    details_content_lines_for_terminal_size, details_scroll_lines_for_terminal_size,
    focused_left_panel_content_lines_for_terminal_size,
};
pub use panels::{
    format_branch_entry, format_commit_entry, format_file_tree_row, format_stash_entry,
};
pub use terminal::{
    render_terminal, render_terminal_buffer, render_terminal_buffer_with_cursor,
    render_terminal_buffer_with_cursor_and_context, render_terminal_buffer_with_render_context,
    render_terminal_text, render_terminal_text_with_context, render_terminal_with_context,
};
pub use text::{render, render_with_context};
pub use theme::{batch_selected_row_style, focused_panel_style, selected_row_style};
