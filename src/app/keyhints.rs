use crate::app::Intent;

use super::Panel;

/// A single shortcut hint shown in the shortcut bar or help panel.
pub struct KeyHint {
    pub key: &'static str,
    pub description: &'static str,
    pub intent: Intent,
}

/// Returns the panel-specific action hints for the shortcut bar.
/// Navigation keys (j/k/h/l/q/1-4) are excluded — they are always available.
pub fn keyhints_for_panel(panel: Panel) -> Vec<KeyHint> {
    let mut hints: Vec<KeyHint> = match panel {
        Panel::Files => vec![
            KeyHint {
                key: "Space",
                description: "stage/unstage",
                intent: Intent::ToggleStageFile,
            },
            KeyHint {
                key: "a",
                description: "stage all",
                intent: Intent::StageAll,
            },
            KeyHint {
                key: "A",
                description: "amend commit",
                intent: Intent::AmendCommit,
            },
            KeyHint {
                key: "d",
                description: "discard",
                intent: Intent::DiscardSelected,
            },
            KeyHint {
                key: "D",
                description: "reset menu",
                intent: Intent::ShowResetMenu,
            },
            KeyHint {
                key: "s",
                description: "stash",
                intent: Intent::StashSelected,
            },
            KeyHint {
                key: "i",
                description: "ignore",
                intent: Intent::IgnoreSelected,
            },
            KeyHint {
                key: "v",
                description: "multi-select",
                intent: Intent::None,
            },
        ],
        Panel::Branches => vec![KeyHint {
            key: "Enter",
            description: "checkout / view commits",
            intent: Intent::ActivatePanel,
        }],
        Panel::Commits => vec![
            KeyHint {
                key: "Enter",
                description: "view files",
                intent: Intent::ActivatePanel,
            },
            KeyHint {
                key: "v",
                description: "multi-select",
                intent: Intent::None,
            },
        ],
        Panel::Stash => vec![KeyHint {
            key: "Enter",
            description: "view stash",
            intent: Intent::ActivatePanel,
        }],
        Panel::MainView => vec![
            KeyHint {
                key: "Ctrl+u",
                description: "page up",
                intent: Intent::ScrollMainView(-12),
            },
            KeyHint {
                key: "Ctrl+d",
                description: "page down",
                intent: Intent::ScrollMainView(12),
            },
        ],
        Panel::Log => vec![
            KeyHint {
                key: "Ctrl+u",
                description: "page up",
                intent: Intent::ScrollLog(-5),
            },
            KeyHint {
                key: "Ctrl+d",
                description: "page down",
                intent: Intent::ScrollLog(5),
            },
        ],
    };

    // Always append the help hint last
    hints.push(KeyHint {
        key: "?",
        description: "help",
        intent: Intent::ShowHelp,
    });

    hints
}
