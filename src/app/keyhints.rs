use super::Panel;

/// A single shortcut hint shown in the shortcut bar or help panel.
pub struct KeyHint {
    pub key: &'static str,
    pub description: &'static str,
}

/// Returns the panel-specific action hints for the shortcut bar.
/// Navigation keys (j/k/h/l/q/1-4) are excluded — they are always available.
pub fn keyhints_for_panel(panel: Panel) -> Vec<KeyHint> {
    let mut hints: Vec<KeyHint> = match panel {
        Panel::Files => vec![
            KeyHint {
                key: "Space",
                description: "stage/unstage",
            },
            KeyHint {
                key: "a",
                description: "stage all",
            },
            KeyHint {
                key: "c",
                description: "commit",
            },
            KeyHint {
                key: "A",
                description: "amend commit",
            },
            KeyHint {
                key: "d",
                description: "discard",
            },
            KeyHint {
                key: "D",
                description: "reset menu",
            },
            KeyHint {
                key: "s",
                description: "stash",
            },
            KeyHint {
                key: "i",
                description: "ignore",
            },
            KeyHint {
                key: "`",
                description: "toggle tree",
            },
            KeyHint {
                key: "-",
                description: "collapse all",
            },
            KeyHint {
                key: "=",
                description: "expand all",
            },
            KeyHint {
                key: "v",
                description: "multi-select",
            },
        ],
        Panel::Branches => vec![
            KeyHint {
                key: "Space",
                description: "checkout",
            },
            KeyHint {
                key: "n",
                description: "new branch",
            },
            KeyHint {
                key: "d",
                description: "delete options",
            },
            KeyHint {
                key: "Enter",
                description: "view commits",
            },
        ],
        Panel::Commits => vec![
            KeyHint {
                key: "Enter",
                description: "view files",
            },
            KeyHint {
                key: "Ctrl+o",
                description: "copy hash",
            },
            KeyHint {
                key: "g",
                description: "reset",
            },
            KeyHint {
                key: "v",
                description: "multi-select",
            },
        ],
        Panel::Stash => vec![KeyHint {
            key: "Enter",
            description: "view stash",
        }],
        Panel::MainView => vec![
            KeyHint {
                key: "Ctrl+u",
                description: "page up",
            },
            KeyHint {
                key: "Ctrl+d",
                description: "page down",
            },
        ],
        Panel::Log => vec![
            KeyHint {
                key: "Ctrl+u",
                description: "page up",
            },
            KeyHint {
                key: "Ctrl+d",
                description: "page down",
            },
        ],
    };

    hints.push(KeyHint {
        key: "Esc",
        description: "cancel/back",
    });

    // Always append the help hint last
    hints.push(KeyHint {
        key: "?",
        description: "help",
    });

    hints
}
