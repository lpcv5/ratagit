#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Panel {
    MainView,
    #[default]
    Files,
    Branches,
    Commits,
    Stash,
    Log,
}

impl Panel {
    pub const ALL: [Panel; 6] = [
        Panel::MainView,
        Panel::Files,
        Panel::Branches,
        Panel::Commits,
        Panel::Stash,
        Panel::Log,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Panel::MainView => "Main View",
            Panel::Files => "Files",
            Panel::Branches => "Branches",
            Panel::Commits => "Commits",
            Panel::Stash => "Stash",
            Panel::Log => "Log",
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|panel| *panel == self)
            .unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|panel| *panel == self)
            .unwrap_or(0);
        if index == 0 {
            Self::ALL[Self::ALL.len() - 1]
        } else {
            Self::ALL[index - 1]
        }
    }
}

#[derive(Default)]
pub struct UiState {
    pub active_panel: Panel,
}

#[cfg(test)]
mod tests {
    use super::Panel;

    #[test]
    fn panel_cycle_excludes_commit_files_panel() {
        assert_eq!(Panel::ALL.len(), 6);
        assert_eq!(Panel::Commits.next(), Panel::Stash);
        assert_eq!(Panel::Stash.previous(), Panel::Commits);
    }
}
