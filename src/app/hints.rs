use crate::app::{App, InputMode, SidePanel};

impl App {
    pub fn shortcut_hints(&self) -> Vec<(String, String)> {
        if let Some(mode) = self.input_mode {
            return match mode {
                InputMode::CommitEditor => vec![
                    ("Tab".to_string(), "SwitchField".to_string()),
                    ("Enter".to_string(), "Confirm/DescNewline".to_string()),
                    ("Esc".to_string(), "Cancel".to_string()),
                    ("Backspace".to_string(), "Delete".to_string()),
                ],
                InputMode::CreateBranch => vec![
                    ("Enter".to_string(), "Confirm".to_string()),
                    ("Esc".to_string(), "Cancel".to_string()),
                    ("Backspace".to_string(), "Delete".to_string()),
                ],
                InputMode::StashEditor => vec![
                    ("Enter".to_string(), "Confirm".to_string()),
                    ("Esc".to_string(), "Cancel".to_string()),
                    ("Backspace".to_string(), "Delete".to_string()),
                ],
            };
        }

        let mut hints = vec![
            (
                format!(
                    "{}/{}",
                    self.global_key_or("panel_prev", "h"),
                    self.global_key_or("panel_next", "l")
                ),
                "Panel".to_string(),
            ),
            (
                format!(
                    "{}/{}",
                    self.global_key_or("list_up", "k"),
                    self.global_key_or("list_down", "j")
                ),
                "Move".to_string(),
            ),
            (self.global_key_or("refresh", "r"), "Refresh".to_string()),
            (self.global_key_or("commit", "c"), "Commit".to_string()),
            (
                format!(
                    "{}/{}",
                    self.global_key_or("diff_scroll_up", "C-u"),
                    self.global_key_or("diff_scroll_down", "C-d")
                ),
                "DiffScroll".to_string(),
            ),
            (self.global_key_or("quit", "q"), "Quit".to_string()),
        ];

        let panel = self.active_panel_name();
        match self.active_panel {
            SidePanel::Files => {
                hints.push((
                    self.panel_key_or(panel, "toggle_visual_select", "v"),
                    if self.files_visual_mode {
                        "VisualOn".to_string()
                    } else {
                        "Visual".to_string()
                    },
                ));
                hints.push((
                    self.panel_key_or(panel, "toggle_stage", "Space"),
                    if self.files_visual_mode {
                        "BatchToggle".to_string()
                    } else {
                        "Stage".to_string()
                    },
                ));
                hints.push((
                    self.panel_key_or(panel, "toggle_dir", "Enter"),
                    "ToggleDir".to_string(),
                ));
                hints.push((
                    self.panel_key_or(panel, "collapse_all", "-"),
                    "Collapse".to_string(),
                ));
                hints.push((
                    self.panel_key_or(panel, "expand_all", "="),
                    "Expand".to_string(),
                ));
                hints.push((
                    self.panel_key_or(panel, "stash_push", "s"),
                    "Stash".to_string(),
                ));
            }
            SidePanel::LocalBranches => {
                hints.push((
                    self.panel_key_or(panel, "checkout_branch", "Enter"),
                    "Checkout".to_string(),
                ));
                hints.push((
                    self.panel_key_or(panel, "create_branch", "n"),
                    "NewBranch".to_string(),
                ));
                hints.push((
                    self.panel_key_or(panel, "delete_branch", "d"),
                    "Delete".to_string(),
                ));
                hints.push((
                    self.panel_key_or(panel, "fetch_remote", "f"),
                    "Fetch".to_string(),
                ));
            }
            SidePanel::Stash => {
                hints.push((
                    self.panel_key_or(panel, "open_tree", "Enter"),
                    if self.stash_tree_mode {
                        "ToggleDir".to_string()
                    } else {
                        "Files".to_string()
                    },
                ));
                hints.push((
                    self.panel_key_or(panel, "stash_apply", "a"),
                    "Apply".to_string(),
                ));
                hints.push((
                    self.panel_key_or(panel, "stash_pop", "p"),
                    "Pop".to_string(),
                ));
                hints.push((
                    self.panel_key_or(panel, "stash_drop", "d"),
                    "Drop".to_string(),
                ));
            }
            SidePanel::Commits => {
                hints.push((
                    self.panel_key_or(panel, "open_tree", "Enter"),
                    if self.commit_tree_mode {
                        "ToggleDir".to_string()
                    } else {
                        "Files".to_string()
                    },
                ));
            }
        }

        hints
    }
}
