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
            move_index(
                &mut state.branches.selected,
                state.branches.items.len(),
                move_up,
            );
            Vec::new()
        }
        PanelFocus::Commits => commit_workflow::move_commit_selection(state, move_up),
        PanelFocus::Stash => {
            move_index(&mut state.stash.selected, state.stash.items.len(), move_up);
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

fn move_index(selected: &mut usize, len: usize, move_up: bool) {
    if len == 0 {
        *selected = 0;
        return;
    }
    if move_up {
        *selected = selected.saturating_sub(1);
    } else {
        *selected = (*selected + 1).min(len - 1);
    }
}
