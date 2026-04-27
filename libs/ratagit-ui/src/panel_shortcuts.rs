use ratagit_core::{AppState, PanelFocus};

pub(crate) fn shortcuts_for_state(state: &AppState) -> String {
    if let Some(editor) = &state.editor.kind {
        return match editor {
            ratagit_core::EditorKind::Commit { .. } => {
                "commit editor: Tab field | arrows/Home/End cursor | Ctrl+J newline | Enter confirm | Esc cancel"
                    .to_string()
            }
            ratagit_core::EditorKind::Stash { .. } => {
                "stash editor: arrows/Home/End cursor | Enter confirm | Esc cancel".to_string()
            }
        };
    }

    if state.branches.create.active {
        return "branch name: arrows/Home/End cursor | Enter create | Esc cancel".to_string();
    }

    if state.branches.delete_menu.active {
        return "delete branch: j/k select | Enter delete | Esc cancel".to_string();
    }

    if state.branches.force_delete_confirm.active {
        return "force delete branch: Enter force delete | Esc cancel".to_string();
    }

    if state.branches.rebase_menu.active {
        return "rebase: j/k select | Enter rebase | Esc cancel".to_string();
    }

    if state.branches.auto_stash_confirm.active {
        return "auto stash: Enter confirm | Esc cancel".to_string();
    }

    if state.reset_menu.active {
        return "reset: j/k select | Enter confirm | Esc cancel".to_string();
    }

    if state.discard_confirm.active {
        return "discard: Enter confirm | Esc cancel".to_string();
    }

    if state
        .active_search_scope()
        .is_some_and(|scope| state.search.is_input_active_for(scope))
    {
        return format!("search: {}", state.search.query);
    }
    match state.focus {
        PanelFocus::Files => {
            "keys(files): space stage/unstage | d discard | c commit | s stash(all|selected) | D reset | v multi | enter expand".to_string()
        }
        PanelFocus::Branches => {
            "keys(branches): space checkout | n new | d delete | r rebase".to_string()
        }
        PanelFocus::Commits => {
            if state.commits.files.active {
                "keys(commit files): Esc back".to_string()
            } else {
                "keys(commits): enter files | s squash | f fixup | r reword | d delete | space detach | v multi"
                    .to_string()
            }
        }
        PanelFocus::Stash => "keys(stash): p stash push | O stash pop".to_string(),
        PanelFocus::Details | PanelFocus::Log => String::new(),
    }
}
