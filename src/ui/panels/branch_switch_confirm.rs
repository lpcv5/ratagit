use super::{overlay_utils::render_confirmation, props::BranchSwitchConfirmProps};
use ratatui::Frame;

pub fn render_branch_switch_confirm(frame: &mut Frame, props: BranchSwitchConfirmProps<'_>) {
    render_confirmation(frame, "Switch Branch", &format!("{} uncommitted file(s)", props.change_count),
        &format!("Switch to {}? Stash tracked and untracked changes, switch branches, then reapply the stash. Conflicts may need resolution.", props.target), "Stash + switch");
}
