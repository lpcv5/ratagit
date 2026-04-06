use crate::app::{App, InputMode, SidePanel};

impl App {
    pub fn shortcut_hints(&self) -> Vec<(String, String)> {
        if let Some(mode) = self.input.mode {
            return match mode {
                InputMode::CommitEditor => vec![
                    ("Tab".to_string(), "SwitchField".to_string()),
                    ("Enter".to_string(), "Confirm".to_string()),
                    ("Esc".to_string(), "Cancel".to_string()),
                ],
                InputMode::CreateBranch | InputMode::StashEditor | InputMode::Search => vec![
                    ("Enter".to_string(), "Confirm".to_string()),
                    ("Esc".to_string(), "Cancel".to_string()),
                ],
                InputMode::CommandPalette => vec![
                    ("Enter".to_string(), "Run".to_string()),
                    ("Esc".to_string(), "Cancel".to_string()),
                ],
                InputMode::BranchSwitchConfirm => vec![
                    ("Y".to_string(), "AutoStash+Switch".to_string()),
                    ("N".to_string(), "Cancel".to_string()),
                ],
                InputMode::CommitAllConfirm => vec![
                    ("Y".to_string(), "StageAll+Commit".to_string()),
                    ("N".to_string(), "Cancel".to_string()),
                ],
            };
        }

        let mut hints = self.panel_primary_hints();
        hints.push((
            self.global_key_or("command_palette", ":"),
            "Command".to_string(),
        ));
        hints.push((self.global_key_or("quit", "q"), "Quit".to_string()));
        hints
    }

    fn panel_primary_hints(&self) -> Vec<(String, String)> {
        let panel = self.active_panel_name();
        match self.ui.active_panel {
            SidePanel::Files => vec![
                (
                    self.panel_key_or(panel, "toggle_stage", "Space"),
                    "Stage".to_string(),
                ),
                (
                    self.panel_key_or(panel, "toggle_visual_select", "v"),
                    "Visual".to_string(),
                ),
                (self.global_key_or("commit", "c"), "Commit".to_string()),
            ],
            SidePanel::LocalBranches => {
                if self.ui.branches.commits_subview_active {
                    vec![
                        (
                            self.panel_key_or(panel, "open_tree", "Enter"),
                            "Refresh".to_string(),
                        ),
                        ("Esc".to_string(), "Back".to_string()),
                    ]
                } else {
                    vec![
                        (
                            self.panel_key_or(panel, "checkout_branch", "Space"),
                            "Checkout".to_string(),
                        ),
                        (
                            self.panel_key_or(panel, "open_tree", "Enter"),
                            "Commits".to_string(),
                        ),
                        (
                            self.panel_key_or(panel, "create_branch", "n"),
                            "New".to_string(),
                        ),
                        (
                            self.panel_key_or(panel, "fetch_remote", "f"),
                            "Fetch".to_string(),
                        ),
                    ]
                }
            }
            SidePanel::Commits => vec![(
                self.panel_key_or(panel, "open_tree", "Enter"),
                "Files".to_string(),
            )],
            SidePanel::Stash => vec![
                (
                    self.panel_key_or(panel, "stash_apply", "a"),
                    "Apply".to_string(),
                ),
                (
                    self.panel_key_or(panel, "stash_pop", "p"),
                    "Pop".to_string(),
                ),
            ],
        }
    }
}
