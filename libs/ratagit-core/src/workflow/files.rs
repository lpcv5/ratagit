use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileFilter {
    #[default]
    All,
    Staged,
    Unstaged,
    Untracked,
    Conflicted,
}

impl FileFilter {
    pub fn matches(self, file: &crate::FileEntry) -> bool {
        match self {
            Self::All => true,
            Self::Staged => file.staged,
            Self::Unstaged => file.unstaged,
            Self::Untracked => file.untracked,
            Self::Conflicted => file.conflicted,
        }
    }
}

pub(super) fn filter(state: &mut AppContext, filter: FileFilter) -> Vec<Command> {
    state.ui.files.filter = filter;
    state.ui.files.selected = 0;
    state.ui.files.scroll_offset = 0;
    state.ui.files.selected_rows.clear();
    state.ui.files.selection_anchor = None;
    state.ui.files.mode = crate::FileInputMode::Normal;
    state.ui.search.clear();
    crate::refresh_tree_projection(&state.repo.files.items, &mut state.ui.files);
    crate::details::refresh_on_navigation(state)
}

pub(super) fn view(state: &mut AppContext, feature: Feature) -> Vec<Command> {
    match feature {
        Feature::ToggleTree => state.ui.files.flat = !state.ui.files.flat,
        Feature::ExpandAll => {
            state.ui.files.expanded_dirs = crate::collect_directories(&state.repo.files.items)
        }
        Feature::CollapseAll => state.ui.files.expanded_dirs.clear(),
        _ => return Vec::new(),
    }
    state.ui.files.selected = 0;
    state.ui.files.scroll_offset = 0;
    state.ui.files.selected_rows.clear();
    state.ui.files.selection_anchor = None;
    state.ui.files.mode = crate::FileInputMode::Normal;
    crate::refresh_tree_projection(&state.repo.files.items, &mut state.ui.files);
    crate::details::refresh_on_navigation(state)
}

pub(super) fn open(state: &mut AppContext, feature: Feature) -> Vec<Command> {
    match feature {
        Feature::Filter => menu(
            state,
            "Filter files by status",
            [
                ("All", FileFilter::All),
                ("Staged", FileFilter::Staged),
                ("Unstaged", FileFilter::Unstaged),
                ("Untracked", FileFilter::Untracked),
                ("Conflicted", FileFilter::Conflicted),
            ]
            .into_iter()
            .map(|(label, filter)| Choice {
                label: label.into(),
                action: ChoiceAction::Filter(filter),
            })
            .collect(),
        ),
        Feature::Stash => menu(
            state,
            "Stash options",
            [
                ("All changes (including untracked)", StashMode::All),
                ("Staged changes only", StashMode::Staged),
                ("Unstaged changes (keep index)", StashMode::Unstaged),
            ]
            .into_iter()
            .map(|(label, mode)| run_choice(label, Operation::Stash(mode)))
            .collect(),
        ),
        Feature::Conflict => {
            let targets = crate::selected_target_paths(&state.repo.files.items, &state.ui.files);
            let paths: Vec<_> = state
                .repo
                .files
                .items
                .iter()
                .filter(|file| file.conflicted && targets.contains(&file.path))
                .map(|file| file.path.clone())
                .collect();
            if paths.is_empty() {
                crate::push_notice(
                    state,
                    "Select conflicted files; e opens the external editor",
                );
                return Vec::new();
            }
            menu(
                state,
                "Resolve selected conflicts",
                [
                    ("Use ours (replace selected files)", ConflictSide::Ours),
                    ("Use theirs (replace selected files)", ConflictSide::Theirs),
                    ("Mark resolved after editing", ConflictSide::Resolved),
                ]
                .into_iter()
                .map(|(label, side)| {
                    run_choice(
                        label,
                        Operation::Resolve {
                            paths: paths.clone(),
                            side,
                        },
                    )
                })
                .collect(),
            );
        }
        Feature::UpstreamReset => super::open::reset_menu(state, "@{upstream}".into()),
        Feature::CommitNoVerify => prompt::open(state, PromptKind::CommitNoVerify, String::new()),
        Feature::ToggleTree | Feature::CollapseAll | Feature::ExpandAll => {
            return view(state, feature);
        }
        _ => {}
    }
    Vec::new()
}
