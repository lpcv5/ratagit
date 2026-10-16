use crate::input::KeyEffect;
use crossterm::event::{KeyCode, KeyModifiers};
use ratagit_core::workflow::{Feature as F, WorkflowAction as A};
use ratagit_core::{ActiveLeftView, AppContext, UiAction};

pub(crate) fn action(
    state: &AppContext,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Option<UiAction> {
    let control = modifiers.contains(KeyModifiers::CONTROL);
    if state.ui.workflow.dialog.is_some() || state.ui.workflow.remote_request.is_some() {
        let prompt = matches!(
            state.ui.workflow.dialog,
            Some(ratagit_core::workflow::Dialog::Prompt { .. })
        );
        let action = match code {
            KeyCode::Esc => A::Cancel,
            KeyCode::Enter => A::Confirm,
            KeyCode::Up => A::Up,
            KeyCode::Down => A::Down,
            KeyCode::Char('k') if !prompt => A::Up,
            KeyCode::Char('j') if !prompt => A::Down,
            KeyCode::Backspace => A::Backspace,
            KeyCode::Left => A::CursorLeft,
            KeyCode::Right => A::CursorRight,
            KeyCode::Home => A::CursorHome,
            KeyCode::End => A::CursorEnd,
            KeyCode::Char(ch)
                if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                A::Input(ch)
            }
            _ => return None,
        };
        return Some(UiAction::Workflow(action));
    }
    if control
        && !matches!(
            (state.active_left_view(), code),
            (Some(ActiveLeftView::Files), KeyCode::Char('b'))
                | (
                    Some(ActiveLeftView::Commits | ActiveLeftView::BranchCommits),
                    KeyCode::Char('r' | 'j' | 'k')
                )
        )
    {
        return None;
    }
    if modifiers.contains(KeyModifiers::ALT) && !matches!(code, KeyCode::Up | KeyCode::Down) {
        return None;
    }
    let feature = if code == KeyCode::Char('m') && !control {
        F::Sequence
    } else {
        match state.active_left_view()? {
            ActiveLeftView::Files => match code {
                KeyCode::Char('b') if control => F::Filter,
                KeyCode::Char('M') => F::Conflict,
                KeyCode::Char('S') => F::Stash,
                KeyCode::Char('`') => F::ToggleTree,
                KeyCode::Char('-') => F::CollapseAll,
                KeyCode::Char('=') => F::ExpandAll,
                KeyCode::Char('g') => F::UpstreamReset,
                KeyCode::Char('w') => F::CommitNoVerify,
                _ => return None,
            },
            ActiveLeftView::BranchesList => match code {
                KeyCode::Char('M') => F::Merge,
                KeyCode::Char('R') => F::Rename,
                KeyCode::Char('u') => F::Upstream,
                KeyCode::Char('f') => F::FastForward,
                KeyCode::Char('c') => F::CheckoutByName,
                KeyCode::Char('-') => F::CheckoutPrevious,
                KeyCode::Char(']') | KeyCode::Tab => F::RemoteBranches,
                _ => return None,
            },
            ActiveLeftView::Commits | ActiveLeftView::BranchCommits => match code {
                KeyCode::Char('r') if control => F::ClearCopied,
                KeyCode::Char('k') if control => F::MoveUp,
                KeyCode::Char('j') if control => F::MoveDown,
                KeyCode::Up if modifiers.contains(KeyModifiers::ALT) => F::MoveUp,
                KeyCode::Down if modifiers.contains(KeyModifiers::ALT) => F::MoveDown,
                KeyCode::Char('C') => F::CopyCommits,
                KeyCode::Char('V') => F::PasteCommits,
                KeyCode::Char('t') => F::Revert,
                KeyCode::Char('g') => F::Reset,
                KeyCode::Char('n') if state.ui.search.query.is_empty() => F::BranchFromCommit,
                KeyCode::Char('F') => F::Fixup,
                KeyCode::Char('S') => F::Autosquash,
                KeyCode::Char('T') => F::Tag,
                KeyCode::Char('y') => F::CopyAttributes,
                _ => return None,
            },
            _ => return None,
        }
    };
    Some(UiAction::Workflow(A::Open(feature)))
}

pub(crate) fn host(
    state: &AppContext,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Option<KeyEffect> {
    if code == KeyCode::Enter
        && let Some(ratagit_core::workflow::Dialog::Menu {
            choices, selected, ..
        }) = &state.ui.workflow.dialog
        && let Some(ratagit_core::workflow::Choice {
            action: ratagit_core::workflow::ChoiceAction::Copy(text),
            ..
        }) = choices.get(*selected)
    {
        return Some(KeyEffect::Host {
            edit: false,
            path: text.clone(),
        });
    }
    if state.ui.workflow.dialog.is_some()
        || state.ui.workflow.remote_request.is_some()
        || (state.ui.command_palette.active
            && (state.ui.command_palette.search_active || code != KeyCode::Enter))
    {
        return None;
    }
    let palette = if code == KeyCode::Enter && state.ui.command_palette.active {
        match state.selected_command_palette_entry()?.command {
            ratagit_core::CommandPaletteCommand::Workflow(feature) => Some(feature),
            _ => None,
        }
    } else {
        None
    };
    let view = state.active_left_view();
    if (code == KeyCode::Char('C')
        && !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        && view == Some(ActiveLeftView::Files))
        || palette == Some(F::CommitExternal)
    {
        return state
            .work
            .mutation
            .operation_pending
            .is_none()
            .then_some(KeyEffect::CommitEditor);
    }
    if !(code == KeyCode::Char('o') && modifiers.contains(KeyModifiers::CONTROL))
        && !matches!(palette, Some(F::CopyBranch | F::CopyCommit))
    {
        return None;
    }
    let text = match view? {
        ActiveLeftView::BranchesList => state
            .repo
            .branches
            .items
            .get(state.ui.branches.selected)?
            .name
            .clone(),
        ActiveLeftView::BranchCommits => ratagit_core::commit_key(
            state
                .repo
                .branches
                .commits
                .get(state.ui.branches.commits.selected)?,
        ),
        ActiveLeftView::Commits => {
            ratagit_core::commit_key(state.repo.commits.items.get(state.ui.commits.selected)?)
        }
        _ => return None,
    };
    Some(KeyEffect::Host {
        edit: false,
        path: text,
    })
}
