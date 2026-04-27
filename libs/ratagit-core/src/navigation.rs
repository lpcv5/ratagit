use crate::scroll::{move_selected_index, move_selected_index_with_scroll_offset};
use crate::{
    AppContext, Command, PanelFocus, branches, commit_workflow, move_selected, push_notice,
    toggle_selected_directory,
};

pub(crate) fn move_selection(state: &mut AppContext, move_up: bool) -> Vec<Command> {
    match state.ui.focus {
        PanelFocus::Files => {
            move_selected(&mut state.ui.files, move_up);
            Vec::new()
        }
        PanelFocus::Branches => {
            branches::move_selection(state, move_up);
            Vec::new()
        }
        PanelFocus::Commits => commit_workflow::move_commit_selection(state, move_up),
        PanelFocus::Stash => {
            move_selected_index(
                &mut state.ui.stash.selected,
                state.repo.stash.items.len(),
                move_up,
            );
            Vec::new()
        }
        PanelFocus::Details | PanelFocus::Log => Vec::new(),
    }
}

pub(crate) fn move_selection_in_viewport(
    state: &mut AppContext,
    move_up: bool,
    visible_lines: usize,
) -> Vec<Command> {
    match state.ui.focus {
        PanelFocus::Files => {
            crate::move_selected_in_viewport(&mut state.ui.files, move_up, visible_lines);
            Vec::new()
        }
        PanelFocus::Branches => {
            branches::move_selection_in_viewport(state, move_up, visible_lines);
            Vec::new()
        }
        PanelFocus::Commits => {
            commit_workflow::move_commit_selection_in_viewport(state, move_up, visible_lines)
        }
        PanelFocus::Stash => {
            move_selected_index_with_scroll_offset(
                &mut state.ui.stash.selected,
                &mut state.ui.stash.scroll_offset,
                state.repo.stash.items.len(),
                move_up,
                visible_lines,
            );
            Vec::new()
        }
        PanelFocus::Details | PanelFocus::Log => Vec::new(),
    }
}

pub(crate) fn toggle_selected_directory_or_notice(state: &mut AppContext) -> bool {
    if toggle_selected_directory(&state.repo.files.items, &mut state.ui.files) {
        true
    } else {
        push_notice(state, "Selected file is not a directory");
        false
    }
}
