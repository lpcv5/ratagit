//! Application-to-UI adapter. Derive display data here; components only format
//! supplied props. Input continues through input_mapper and the command registry.
use super::AppStateSnapshot;
use crate::app::{DiffSide, HunkView, InputMode, SidePanel};
use crate::git::patch::LineId;
use crate::ui::components::diff::{
    DiffPosition, DiffSelection, DiffSelectionMode, DiffStatus, DiffViewModel, DiffViewportAnchor,
};
use crate::ui::panels::props::*;

mod side_panels;

#[cfg(test)]
mod tests;

/// Own computed marks for the duration of a render without retaining domain state.
pub struct DiffPresentation<'a> {
    model: DiffViewModel<'a>,
    marked_lines: Vec<LineId>,
    marked_hunks: Vec<LineId>,
}

impl DiffPresentation<'_> {
    pub fn view(&self) -> DiffViewModel<'_> {
        DiffViewModel {
            selection: DiffSelection {
                marked_lines: &self.marked_lines,
                marked_hunks: &self.marked_hunks,
                ..self.model.selection
            },
            ..self.model
        }
    }
}

impl AppStateSnapshot<'_> {
    pub fn commit_editor_props(&self) -> CommitEditorProps<'_> {
        CommitEditorProps {
            focus: self.commit_focus,
            message: self.commit_message_buffer,
            description: self.commit_description_buffer,
            hints: &self.shortcut_hints,
        }
    }

    pub fn stash_editor_props(&self) -> StashEditorProps<'_> {
        StashEditorProps {
            message: self.stash_message_buffer,
            targets: &self.stash_targets,
        }
    }

    pub fn branch_switch_confirm_props(&self) -> BranchSwitchConfirmProps<'_> {
        BranchSwitchConfirmProps {
            target: self.branch_switch_target.unwrap_or("<unknown>"),
            change_count: self.uncommitted_change_count,
        }
    }

    pub fn commit_all_confirm_props(&self) -> CommitAllConfirmProps {
        CommitAllConfirmProps {
            change_count: self.uncommitted_change_count,
        }
    }

    pub fn command_palette_props(&self) -> CommandPaletteProps<'_> {
        CommandPaletteProps {
            input: self.input_buffer,
            commands: crate::app::commands::filter_commands(
                self.input_buffer,
                &self.available_commands,
            ),
            hints: &self.shortcut_hints,
        }
    }

    pub fn keybindings_props(&self) -> KeybindingsProps<'_> {
        KeybindingsProps {
            context: &self.help.context,
            query: &self.help.query,
            filtering: self.help.filtering,
            selected: self.help.selected,
            entries: self.help.filtered_entries(),
        }
    }

    pub fn command_log_props(&self) -> CommandLogProps<'_> {
        CommandLogProps {
            entries: self.command_log,
            branch_input: (self.input_mode == Some(InputMode::CreateBranch))
                .then_some(self.input_buffer),
        }
    }

    pub fn shortcut_bar_props(&self, animation_frame: usize) -> ShortcutBarProps<'_> {
        ShortcutBarProps {
            hints: &self.shortcut_hints,
            search_input: (self.input_mode == Some(InputMode::Search)).then_some(self.input_buffer),
            loading_frame: self.has_background_tasks.then_some(animation_frame % 10),
        }
    }

    pub fn diff_presentation(&self) -> DiffPresentation<'_> {
        let hunks = HunkView {
            patch: self.current_patch,
            interaction: self.diff_interaction,
            ready: self.diff_ready,
            files_context: self.active_panel == SidePanel::Files,
        };
        let interaction = hunks.interaction;
        let (current, total) = if interaction.focused && hunks.ready {
            hunks.cursor_position()
        } else {
            (0, 0)
        };
        let status = if self.diff_loading || !hunks.ready {
            DiffStatus::Loading
        } else if interaction.pending_write.is_some() {
            DiffStatus::Applying
        } else if hunks.cursor_is_read_only() {
            DiffStatus::ReadOnly
        } else {
            DiffStatus::Idle
        };
        let empty_message = match interaction.side {
            Some(DiffSide::Unstaged) => "No unstaged changes",
            Some(DiffSide::Staged) => "No staged changes",
            None => match self.active_panel {
                SidePanel::Files => "Select a file to view diff",
                SidePanel::LocalBranches => "Select a branch to view details",
                SidePanel::Commits => "Select a commit/file to view diff",
                SidePanel::Stash => "Select a stash entry/file to view diff",
            },
        };
        DiffPresentation {
            marked_lines: hunks.marked_lines(),
            marked_hunks: hunks.marked_hunks(),
            model: DiffViewModel {
                lines: self.current_diff,
                patch: self.current_patch.or(self.retained_diff_patch),
                scroll: self.diff_scroll,
                viewport_anchor: interaction
                    .cursor_relocation
                    .map(|anchor| DiffViewportAnchor {
                        revision: anchor.revision,
                        previous_row: anchor.previous_row,
                        row: anchor.row,
                    }),
                status,
                empty_message,
                selection: DiffSelection {
                    focused: interaction.focused,
                    mode: if interaction.line_mode {
                        DiffSelectionMode::Line
                    } else {
                        DiffSelectionMode::Hunk
                    },
                    hunk_cursor: interaction.cursor,
                    line_cursor: interaction.line_cursor,
                    range_active: if interaction.line_mode {
                        interaction.range_anchor.is_some()
                    } else {
                        interaction.hunk_anchor.is_some()
                    },
                    position: DiffPosition { current, total },
                    ..DiffSelection::default()
                },
            },
        }
    }
}
