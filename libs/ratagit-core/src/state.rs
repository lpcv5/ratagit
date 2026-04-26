use crate::FilesPanelState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelFocus {
    Files,
    Branches,
    Commits,
    Stash,
    Details,
    Log,
}

impl PanelFocus {
    pub fn next_left(self) -> Self {
        match self {
            Self::Files => Self::Branches,
            Self::Branches => Self::Commits,
            Self::Commits => Self::Stash,
            Self::Stash => Self::Files,
            Self::Details | Self::Log => Self::Files,
        }
    }

    pub fn prev_left(self) -> Self {
        match self {
            Self::Files => Self::Stash,
            Self::Branches => Self::Files,
            Self::Commits => Self::Branches,
            Self::Stash => Self::Commits,
            Self::Details | Self::Log => Self::Stash,
        }
    }

    pub fn is_left_panel(self) -> bool {
        matches!(
            self,
            Self::Files | Self::Branches | Self::Commits | Self::Stash
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitEntry {
    pub id: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchEntry {
    pub name: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashEntry {
    pub id: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoSnapshot {
    pub status_summary: String,
    pub current_branch: String,
    pub detached_head: bool,
    pub files: Vec<crate::FileEntry>,
    pub commits: Vec<CommitEntry>,
    pub branches: Vec<BranchEntry>,
    pub stashes: Vec<StashEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusPanelState {
    pub summary: String,
    pub current_branch: String,
    pub detached_head: bool,
    pub refresh_count: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitsPanelState {
    pub items: Vec<CommitEntry>,
    pub selected: usize,
    pub draft_message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitField {
    Message,
    Body,
}

impl CommitField {
    pub fn next(self) -> Self {
        match self {
            Self::Message => Self::Body,
            Self::Body => Self::Message,
        }
    }

    pub fn prev(self) -> Self {
        self.next()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StashScope {
    All,
    SelectedPaths(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetMode {
    Mixed,
    Soft,
    Hard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetChoice {
    Mixed,
    Soft,
    Hard,
    Nuke,
}

impl ResetChoice {
    pub const ALL: [Self; 4] = [Self::Mixed, Self::Soft, Self::Hard, Self::Nuke];

    pub fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|choice| *choice == self)
            .unwrap_or(0);
        Self::ALL[(index + 1).min(Self::ALL.len() - 1)]
    }

    pub fn prev(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|choice| *choice == self)
            .unwrap_or(0);
        Self::ALL[index.saturating_sub(1)]
    }

    pub fn reset_mode(self) -> Option<ResetMode> {
        match self {
            Self::Mixed => Some(ResetMode::Mixed),
            Self::Soft => Some(ResetMode::Soft),
            Self::Hard => Some(ResetMode::Hard),
            Self::Nuke => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResetMenuState {
    pub active: bool,
    pub selected: ResetChoice,
}

impl Default for ResetMenuState {
    fn default() -> Self {
        Self {
            active: false,
            selected: ResetChoice::Mixed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiscardConfirmState {
    pub active: bool,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorKind {
    Commit {
        message: String,
        message_cursor: usize,
        body: String,
        body_cursor: usize,
        active_field: CommitField,
    },
    Stash {
        title: String,
        title_cursor: usize,
        scope: StashScope,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditorState {
    pub kind: Option<EditorKind>,
}

impl EditorState {
    pub fn is_active(&self) -> bool {
        self.kind.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchesPanelState {
    pub items: Vec<BranchEntry>,
    pub selected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashPanelState {
    pub items: Vec<StashEntry>,
    pub selected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailsPanelState {
    pub files_diff: String,
    pub files_targets: Vec<String>,
    pub files_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    pub focus: PanelFocus,
    pub last_left_focus: PanelFocus,
    pub status: StatusPanelState,
    pub files: FilesPanelState,
    pub commits: CommitsPanelState,
    pub branches: BranchesPanelState,
    pub stash: StashPanelState,
    pub details: DetailsPanelState,
    pub editor: EditorState,
    pub reset_menu: ResetMenuState,
    pub discard_confirm: DiscardConfirmState,
    pub notices: Vec<String>,
    pub last_operation: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            focus: PanelFocus::Files,
            last_left_focus: PanelFocus::Files,
            status: StatusPanelState {
                summary: "No data yet".to_string(),
                current_branch: "unknown".to_string(),
                detached_head: false,
                refresh_count: 0,
                last_error: None,
            },
            files: FilesPanelState::default(),
            commits: CommitsPanelState {
                items: Vec::new(),
                selected: 0,
                draft_message: String::new(),
            },
            branches: BranchesPanelState {
                items: Vec::new(),
                selected: 0,
            },
            stash: StashPanelState {
                items: Vec::new(),
                selected: 0,
            },
            details: DetailsPanelState {
                files_diff: String::new(),
                files_targets: Vec::new(),
                files_error: None,
            },
            editor: EditorState::default(),
            reset_menu: ResetMenuState::default(),
            discard_confirm: DiscardConfirmState::default(),
            notices: vec!["Ready".to_string()],
            last_operation: None,
        }
    }
}
