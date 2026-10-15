use super::{overlay_utils::render_confirmation, props::CommitAllConfirmProps};
use ratatui::Frame;

pub fn render_commit_all_confirm(frame: &mut Frame, props: CommitAllConfirmProps) {
    render_confirmation(frame, "Commit All Files", &format!("{} changed file(s) · Nothing staged", props.change_count),
        "Stage all changed files and open the commit editor. You can review the message before committing.", "Stage all + commit");
}
