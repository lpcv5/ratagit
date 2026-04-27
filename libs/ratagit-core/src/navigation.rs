use crate::scroll::{move_selected_index, move_selected_index_with_scroll_offset};
use crate::{
    AppState, Command, PanelFocus, commit_workflow, move_selected, push_notice,
    toggle_selected_directory,
};

pub(crate) fn move_selection(state: &mut AppState, move_up: bool) -> Vec<Command> {
    match state.focus {
        PanelFocus::Files => {
            move_selected(&mut state.files, move_up);
            Vec::new()
        }
        PanelFocus::Branches => {
            move_selected_index(
                &mut state.branches.selected,
                state.branches.items.len(),
                move_up,
            );
            Vec::new()
        }
        PanelFocus::Commits => commit_workflow::move_commit_selection(state, move_up),
        PanelFocus::Stash => {
            move_selected_index(&mut state.stash.selected, state.stash.items.len(), move_up);
            Vec::new()
        }
        PanelFocus::Details | PanelFocus::Log => Vec::new(),
    }
}

pub(crate) fn move_selection_in_viewport(
    state: &mut AppState,
    move_up: bool,
    visible_lines: usize,
) -> Vec<Command> {
    match state.focus {
        PanelFocus::Files => {
            crate::move_selected_in_viewport(&mut state.files, move_up, visible_lines);
            Vec::new()
        }
        PanelFocus::Branches => {
            move_selected_index_with_scroll_offset(
                &mut state.branches.selected,
                &mut state.branches.scroll_offset,
                state.branches.items.len(),
                move_up,
                visible_lines,
            );
            Vec::new()
        }
        PanelFocus::Commits => {
            commit_workflow::move_commit_selection_in_viewport(state, move_up, visible_lines)
        }
        PanelFocus::Stash => {
            move_selected_index_with_scroll_offset(
                &mut state.stash.selected,
                &mut state.stash.scroll_offset,
                state.stash.items.len(),
                move_up,
                visible_lines,
            );
            Vec::new()
        }
        PanelFocus::Details | PanelFocus::Log => Vec::new(),
    }
}

pub(crate) fn toggle_selected_directory_or_notice(state: &mut AppState) -> bool {
    if toggle_selected_directory(&mut state.files) {
        true
    } else {
        push_notice(state, "Selected file is not a directory");
        false
    }
}
