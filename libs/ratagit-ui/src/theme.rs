use ratagit_core::PanelFocus;
use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiTheme {
    pub accent: Color,
    pub selection_text: Color,
    pub selection: Color,
    pub batch_selection_text: Color,
    pub batch_selection: Color,
    pub muted: Color,
    pub danger: Color,
    pub warning: Color,
    pub success: Color,
    pub info: Color,
    pub foreground: Color,
    pub background: Color,
    pub badge_text: Color,
    pub badge_background: Color,
    pub commit: CommitTheme,
    pub files: FileTheme,
    pub diff: DiffTheme,
    pub modal: ModalTheme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModalTheme {
    pub active: Color,
    pub text: Option<Color>,
    pub dim: Color,
    pub background: Option<Color>,
    pub surface_text: Color,
    pub surface: Color,
    pub border: Color,
    pub danger: Color,
    pub warning: Color,
    pub scrim: Option<Color>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitTheme {
    pub graph: Color,
    pub hash_merged: Color,
    pub hash_pushed: Color,
    pub hash_unpushed: Color,
    pub authors: [Color; 8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileTheme {
    pub added: Color,
    pub modified: Color,
    pub deleted: Color,
    pub renamed: Color,
    pub copied: Color,
    pub type_changed: Color,
    pub unknown: Color,
    pub conflict: Color,
    pub staged: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffTheme {
    pub added_text: Color,
    pub removed_text: Color,
    pub context_text: Color,
    pub gutter: Color,
    pub added: [[Color; 3]; 2],
    pub removed: [[Color; 3]; 2],
    pub context_cursor: Color,
    pub context_marked: Color,
}

const DEFAULT_COMMIT: CommitTheme = CommitTheme {
    graph: Color::DarkGray,
    hash_merged: Color::Green,
    hash_pushed: Color::Yellow,
    hash_unpushed: Color::Red,
    authors: [
        Color::Cyan,
        Color::Magenta,
        Color::Rgb(138, 180, 248),
        Color::LightGreen,
        Color::LightBlue,
        Color::LightMagenta,
        Color::LightCyan,
        Color::White,
    ],
};
const DEFAULT_FILES: FileTheme = FileTheme {
    added: Color::Green,
    modified: Color::Yellow,
    deleted: Color::Red,
    renamed: Color::Cyan,
    copied: Color::Cyan,
    type_changed: Color::Yellow,
    unknown: Color::Cyan,
    conflict: Color::Red,
    staged: Color::Green,
};
const DEFAULT_DIFF: DiffTheme = DiffTheme {
    added_text: Color::Rgb(174, 239, 190),
    removed_text: Color::Rgb(255, 190, 198),
    context_text: Color::Rgb(224, 230, 240),
    gutter: Color::Yellow,
    added: [
        [
            Color::Rgb(24, 46, 36),
            Color::Rgb(30, 65, 47),
            Color::Rgb(40, 80, 58),
        ],
        [
            Color::Rgb(33, 67, 48),
            Color::Rgb(44, 85, 60),
            Color::Rgb(52, 100, 70),
        ],
    ],
    removed: [
        [
            Color::Rgb(51, 30, 37),
            Color::Rgb(72, 38, 46),
            Color::Rgb(88, 44, 54),
        ],
        [
            Color::Rgb(73, 36, 45),
            Color::Rgb(94, 44, 55),
            Color::Rgb(111, 52, 64),
        ],
    ],
    context_cursor: Color::Rgb(37, 57, 82),
    context_marked: Color::Rgb(46, 58, 86),
};

pub const MAIN_UI_THEME: UiTheme = UiTheme {
    accent: Color::Yellow,
    selection_text: Color::Black,
    selection: Color::Yellow,
    batch_selection_text: Color::White,
    batch_selection: Color::Blue,
    muted: Color::DarkGray,
    danger: Color::Red,
    warning: Color::Yellow,
    success: Color::Green,
    info: Color::Cyan,
    foreground: Color::Reset,
    background: Color::Reset,
    badge_text: Color::Black,
    badge_background: Color::DarkGray,
    commit: DEFAULT_COMMIT,
    files: DEFAULT_FILES,
    diff: DEFAULT_DIFF,
    modal: ModalTheme {
        active: Color::Cyan,
        text: None,
        dim: Color::DarkGray,
        background: None,
        surface_text: Color::Rgb(20, 34, 56),
        surface: Color::Rgb(180, 215, 255),
        border: Color::DarkGray,
        danger: Color::Red,
        warning: Color::Yellow,
        scrim: None,
    },
};

pub const TOKYO_NIGHT_THEME: UiTheme = UiTheme {
    accent: Color::Rgb(0xe0, 0xaf, 0x68),
    selection_text: Color::Rgb(0x1a, 0x1b, 0x26),
    selection: Color::Rgb(0xe0, 0xaf, 0x68),
    batch_selection_text: Color::Rgb(0x1a, 0x1b, 0x26),
    batch_selection: Color::Rgb(0x7a, 0xa2, 0xf7),
    muted: Color::Rgb(0x56, 0x5f, 0x89),
    danger: Color::Rgb(0xf7, 0x76, 0x8e),
    warning: Color::Rgb(0xe0, 0xaf, 0x68),
    success: Color::Rgb(0x9e, 0xce, 0x6a),
    info: Color::Rgb(0x7a, 0xa2, 0xf7),
    foreground: Color::Rgb(0xc0, 0xca, 0xf5),
    background: Color::Rgb(0x1a, 0x1b, 0x26),
    badge_text: Color::Rgb(0xc0, 0xca, 0xf5),
    badge_background: Color::Rgb(0x24, 0x28, 0x3b),
    commit: DEFAULT_COMMIT,
    files: DEFAULT_FILES,
    diff: DEFAULT_DIFF,
    modal: ModalTheme {
        active: Color::Rgb(0x7a, 0xa2, 0xf7),
        text: Some(Color::Rgb(0xc0, 0xca, 0xf5)),
        dim: Color::Rgb(0x56, 0x5f, 0x89),
        background: Some(Color::Rgb(0x1a, 0x1b, 0x26)),
        surface_text: Color::Rgb(0xc0, 0xca, 0xf5),
        surface: Color::Rgb(0x24, 0x28, 0x3b),
        border: Color::Rgb(0x3b, 0x42, 0x61),
        danger: Color::Rgb(0xf7, 0x76, 0x8e),
        warning: Color::Rgb(0xe0, 0xaf, 0x68),
        scrim: Some(Color::Rgb(0x16, 0x1b, 0x2d)),
    },
};

pub const BUILT_IN_THEMES: [UiTheme; 2] = [MAIN_UI_THEME, TOKYO_NIGHT_THEME];

impl Default for UiTheme {
    fn default() -> Self {
        MAIN_UI_THEME
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowRole {
    Normal,
    Muted,
    BatchSelected,
    #[cfg(test)]
    FileStaged,
    #[cfg(test)]
    FileUntracked,
    SearchMatch,
    CurrentBranch,
    Error,
    Notice,
    #[cfg(test)]
    DiffMeta,
    DiffAdd,
    DiffRemove,
}

pub(crate) const ICON_BATCH_SELECTED: &str = "✓";
pub(crate) const ICON_BRANCH: &str = "";
pub(crate) const ICON_DIRECTORY_CLOSED: &str = "";
pub(crate) const ICON_DIRECTORY_OPEN: &str = "";
pub(crate) const ICON_SEARCH_MATCH: &str = "";
pub(crate) const ICON_STASH: &str = "";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoadingSpotlightTone {
    Dim,
    Mid,
    Bright,
}

pub fn focused_panel_style() -> Style {
    MAIN_UI_THEME.focused_panel_style()
}
pub fn selected_row_style() -> Style {
    MAIN_UI_THEME.selected_row_style()
}
pub fn batch_selected_row_style() -> Style {
    MAIN_UI_THEME.batch_selected_row_style()
}

impl UiTheme {
    pub fn base_style(&self) -> Style {
        Style::default().fg(self.foreground).bg(self.background)
    }
    pub fn focused_panel_style(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected_row_style(&self) -> Style {
        Style::default()
            .fg(self.selection_text)
            .bg(self.selection)
            .add_modifier(Modifier::BOLD)
    }

    pub fn batch_selected_row_style(&self) -> Style {
        Style::default()
            .fg(self.batch_selection_text)
            .bg(self.batch_selection)
            .add_modifier(Modifier::BOLD)
    }

    pub(crate) fn modal_info_style(&self) -> Style {
        Style::default()
            .fg(self.modal.active)
            .add_modifier(Modifier::BOLD)
    }

    pub(crate) fn modal_warning_style(&self) -> Style {
        Style::default()
            .fg(self.modal.warning)
            .add_modifier(Modifier::BOLD)
    }

    pub(crate) fn modal_danger_style(&self) -> Style {
        Style::default()
            .fg(self.modal.danger)
            .add_modifier(Modifier::BOLD)
    }

    pub(crate) fn modal_text_style(&self) -> Style {
        Style::default().fg(self.modal.text.unwrap_or(self.foreground))
    }

    pub(crate) fn modal_background_style(&self) -> Style {
        self.modal_text_style()
            .bg(self.modal.background.unwrap_or(self.background))
    }

    pub(crate) fn modal_scrim_style(&self) -> Style {
        Style::default()
            .fg(self.modal.dim)
            // A missing background would preserve batch and diff highlights underneath.
            .bg(self.modal.scrim.unwrap_or(self.background))
    }

    pub(crate) fn modal_border_style(&self) -> Style {
        Style::default().fg(self.modal.border)
    }

    pub(crate) fn modal_selected_row_style(&self) -> Style {
        Style::default()
            .fg(self.modal.surface_text)
            .bg(self.modal.surface)
            .add_modifier(Modifier::BOLD)
    }

    pub(crate) fn modal_muted_style(&self) -> Style {
        Style::default().fg(self.modal.dim)
    }

    pub(crate) fn modal_footer_style(&self) -> Style {
        Style::default().fg(self.modal.dim)
    }

    pub(crate) fn inactive_panel_style(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub(crate) fn loading_spinner_style(&self) -> Style {
        Style::default()
            .fg(self.warning)
            .add_modifier(Modifier::BOLD)
    }

    pub(crate) fn loading_text_style(&self, tone: LoadingSpotlightTone) -> Style {
        match tone {
            LoadingSpotlightTone::Dim => Style::default().fg(self.muted),
            LoadingSpotlightTone::Mid => Style::default().fg(self.info),
            LoadingSpotlightTone::Bright => Style::default()
                .fg(self.warning)
                .add_modifier(Modifier::BOLD),
        }
    }

    pub(crate) fn title_badge_style(&self, focused: bool) -> Style {
        let background = if focused {
            self.selection
        } else {
            self.badge_background
        };
        Style::default()
            .fg(if focused {
                self.selection_text
            } else {
                self.badge_text
            })
            .bg(background)
            .add_modifier(Modifier::BOLD)
    }

    pub(crate) fn row_style(&self, role: RowRole) -> Style {
        match role {
            RowRole::Normal => Style::default(),
            RowRole::Muted => Style::default().fg(self.muted),
            RowRole::BatchSelected => self.batch_selected_row_style(),
            #[cfg(test)]
            RowRole::FileStaged => Style::default().fg(self.files.staged),
            #[cfg(test)]
            RowRole::FileUntracked => Style::default().fg(self.files.unknown),
            RowRole::SearchMatch => Style::default()
                .fg(self.warning)
                .add_modifier(Modifier::BOLD),
            RowRole::CurrentBranch => Style::default()
                .fg(self.success)
                .add_modifier(Modifier::BOLD),
            RowRole::Error => Style::default().fg(self.danger),
            RowRole::Notice => Style::default().fg(self.info),
            #[cfg(test)]
            RowRole::DiffMeta => Style::default().fg(self.info),
            RowRole::DiffAdd => Style::default().fg(self.success),
            RowRole::DiffRemove => Style::default().fg(self.danger),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PanelLabel {
    pub badge: &'static str,
    pub body: &'static str,
}

pub(crate) fn panel_label(panel: PanelFocus) -> PanelLabel {
    match panel {
        PanelFocus::Files => PanelLabel {
            badge: "1",
            body: "󰈙 Files",
        },
        PanelFocus::Branches => PanelLabel {
            badge: "2",
            body: " Branches",
        },
        PanelFocus::Commits => PanelLabel {
            badge: "3",
            body: " Commits",
        },
        PanelFocus::Stash => PanelLabel {
            badge: "4",
            body: " Stash",
        },
        PanelFocus::Details => PanelLabel {
            badge: "5",
            body: " Details",
        },
        PanelFocus::Log => PanelLabel {
            badge: "6",
            body: "󰌱 Log",
        },
    }
}

impl UiTheme {
    pub fn from_preset(name: &str) -> Result<Self, String> {
        match name {
            "default" => Ok(MAIN_UI_THEME),
            "tokyo-night" => Ok(TOKYO_NIGHT_THEME),
            _ => Err(format!(
                "unknown theme preset {name:?}; expected default or tokyo-night"
            )),
        }
    }

    /// Override a semantic color. Paired surfaces require RGB colors for validation.
    pub fn set_color(&mut self, role: &str, color: Color) -> Result<(), String> {
        if matches!(
            role,
            "selection.foreground"
                | "selection.background"
                | "batch.foreground"
                | "batch.background"
                | "badge.foreground"
                | "badge.background"
                | "modal.selection.foreground"
                | "modal.selection.background"
                | "diff.added.foreground"
                | "diff.removed.foreground"
                | "diff.context.foreground"
                | "diff.context.cursor"
                | "diff.context.marked"
                | "diff.added.unstaged.normal"
                | "diff.added.unstaged.marked"
                | "diff.added.unstaged.cursor"
                | "diff.added.staged.normal"
                | "diff.added.staged.marked"
                | "diff.added.staged.cursor"
                | "diff.removed.unstaged.normal"
                | "diff.removed.unstaged.marked"
                | "diff.removed.unstaged.cursor"
                | "diff.removed.staged.normal"
                | "diff.removed.staged.marked"
                | "diff.removed.staged.cursor"
                | "diff.gutter"
        ) && !matches!(color, Color::Rgb(..))
        {
            return Err(format!(
                "{role}: use #RRGGBB for contrast-validated color pairs"
            ));
        }
        match role {
            "foreground" => self.foreground = color,
            "background" => self.background = color,
            "accent" => self.accent = color,
            "muted" => self.muted = color,
            "danger" => self.danger = color,
            "warning" => self.warning = color,
            "success" => self.success = color,
            "info" => self.info = color,
            "selection.foreground" => self.selection_text = color,
            "selection.background" => self.selection = color,
            "batch.foreground" => self.batch_selection_text = color,
            "batch.background" => self.batch_selection = color,
            "badge.foreground" => self.badge_text = color,
            "badge.background" => self.badge_background = color,
            "modal.active" => self.modal.active = color,
            "modal.foreground" => self.modal.text = Some(color),
            "modal.background" => self.modal.background = Some(color),
            "modal.muted" => self.modal.dim = color,
            "modal.border" => self.modal.border = color,
            "modal.danger" => self.modal.danger = color,
            "modal.warning" => self.modal.warning = color,
            "modal.scrim" => self.modal.scrim = Some(color),
            "modal.selection.foreground" => self.modal.surface_text = color,
            "modal.selection.background" => self.modal.surface = color,
            "commit.graph" => self.commit.graph = color,
            "commit.hash.merged" => self.commit.hash_merged = color,
            "commit.hash.pushed" => self.commit.hash_pushed = color,
            "commit.hash.unpushed" => self.commit.hash_unpushed = color,
            "diff.added.foreground" => self.diff.added_text = color,
            "diff.removed.foreground" => self.diff.removed_text = color,
            "diff.context.foreground" => self.diff.context_text = color,
            "diff.gutter" => self.diff.gutter = color,
            "diff.context.cursor" => self.diff.context_cursor = color,
            "diff.context.marked" => self.diff.context_marked = color,
            "files.added" => self.files.added = color,
            "files.modified" => self.files.modified = color,
            "files.deleted" => self.files.deleted = color,
            "files.renamed" => self.files.renamed = color,
            "files.copied" => self.files.copied = color,
            "files.type_changed" => self.files.type_changed = color,
            "files.unknown" => self.files.unknown = color,
            "files.conflict" => self.files.conflict = color,
            "files.staged" => self.files.staged = color,
            "commit.author.0" => self.commit.authors[0] = color,
            "commit.author.1" => self.commit.authors[1] = color,
            "commit.author.2" => self.commit.authors[2] = color,
            "commit.author.3" => self.commit.authors[3] = color,
            "commit.author.4" => self.commit.authors[4] = color,
            "commit.author.5" => self.commit.authors[5] = color,
            "commit.author.6" => self.commit.authors[6] = color,
            "commit.author.7" => self.commit.authors[7] = color,
            "diff.added.unstaged.normal" => self.diff.added[0][0] = color,
            "diff.added.unstaged.marked" => self.diff.added[0][1] = color,
            "diff.added.unstaged.cursor" => self.diff.added[0][2] = color,
            "diff.added.staged.normal" => self.diff.added[1][0] = color,
            "diff.added.staged.marked" => self.diff.added[1][1] = color,
            "diff.added.staged.cursor" => self.diff.added[1][2] = color,
            "diff.removed.unstaged.normal" => self.diff.removed[0][0] = color,
            "diff.removed.unstaged.marked" => self.diff.removed[0][1] = color,
            "diff.removed.unstaged.cursor" => self.diff.removed[0][2] = color,
            "diff.removed.staged.normal" => self.diff.removed[1][0] = color,
            "diff.removed.staged.marked" => self.diff.removed[1][1] = color,
            "diff.removed.staged.cursor" => self.diff.removed[1][2] = color,
            _ => return Err(format!("unknown theme color role {role:?}")),
        }
        Ok(())
    }

    /// Validate the final merged palette; terminal-defined legacy defaults are exempt.
    pub fn validate(&self) -> Result<(), String> {
        for (role, foreground, background, minimum) in [
            ("selection", self.selection_text, self.selection, 4.5),
            (
                "batch",
                self.batch_selection_text,
                self.batch_selection,
                4.5,
            ),
            ("badge", self.badge_text, self.badge_background, 4.5),
            (
                "modal.selection",
                self.modal.surface_text,
                self.modal.surface,
                7.0,
            ),
        ] {
            let legacy = match role {
                "selection" => (MAIN_UI_THEME.selection_text, MAIN_UI_THEME.selection),
                "batch" => (
                    MAIN_UI_THEME.batch_selection_text,
                    MAIN_UI_THEME.batch_selection,
                ),
                "badge" => (MAIN_UI_THEME.badge_text, MAIN_UI_THEME.badge_background),
                _ => (Color::Reset, Color::Reset),
            };
            if (foreground, background) != legacy {
                check_contrast(role, foreground, background, minimum)?;
            }
        }
        for (role, text, shades) in [
            ("diff.added", self.diff.added_text, self.diff.added),
            ("diff.removed", self.diff.removed_text, self.diff.removed),
        ] {
            for (stage, backgrounds) in shades.iter().enumerate() {
                for (state, background) in backgrounds.iter().enumerate() {
                    let label = format!(
                        "{role}.{}.{}",
                        ["unstaged", "staged"][stage],
                        ["normal", "marked", "cursor"][state]
                    );
                    check_contrast(&label, text, *background, 4.5)?;
                    check_contrast(
                        &format!("{label} gutter"),
                        self.diff.context_text,
                        *background,
                        4.5,
                    )?;
                }
            }
        }
        for (role, background) in [
            ("diff.context.cursor", self.diff.context_cursor),
            ("diff.context.marked", self.diff.context_marked),
        ] {
            check_contrast(role, self.diff.context_text, background, 4.5)?;
        }
        let modal_text = self.modal.text.unwrap_or(self.foreground);
        let modal_background = self.modal.background.unwrap_or(self.background);
        for (role, text, background) in [
            ("base", self.foreground, self.background),
            ("modal", modal_text, modal_background),
        ] {
            check_known_surface(role, text, background)?;
        }
        for (role, text) in [
            ("accent", self.accent),
            ("danger", self.danger),
            ("warning", self.warning),
            ("success", self.success),
            ("info", self.info),
            ("files.added", self.files.added),
            ("files.modified", self.files.modified),
            ("files.deleted", self.files.deleted),
            ("files.renamed", self.files.renamed),
            ("files.copied", self.files.copied),
            ("files.type_changed", self.files.type_changed),
            ("files.unknown", self.files.unknown),
            ("files.conflict", self.files.conflict),
            ("files.staged", self.files.staged),
            ("commit.hash.merged", self.commit.hash_merged),
            ("commit.hash.pushed", self.commit.hash_pushed),
            ("commit.hash.unpushed", self.commit.hash_unpushed),
            ("diff.gutter", self.diff.gutter),
        ] {
            check_known_surface(role, text, self.background)?;
        }
        for (index, text) in self.commit.authors.iter().enumerate() {
            check_known_surface(&format!("commit.author.{index}"), *text, self.background)?;
        }
        for (role, text) in [
            ("modal.active", self.modal.active),
            ("modal.warning", self.modal.warning),
            ("modal.danger", self.modal.danger),
        ] {
            check_known_surface(role, text, modal_background)?;
        }
        for (role, text, background) in [
            ("muted", self.muted, self.background),
            ("commit.graph", self.commit.graph, self.background),
            ("modal.muted", self.modal.dim, modal_background),
            ("modal.border", self.modal.border, modal_background),
            (
                "modal.scrim",
                self.modal.dim,
                self.modal.scrim.unwrap_or(self.background),
            ),
        ] {
            check_distinct(role, text, background)?;
        }
        Ok(())
    }
}

fn check_distinct(role: &str, foreground: Color, background: Color) -> Result<(), String> {
    if foreground == background && foreground != Color::Reset {
        Err(format!(
            "{role}: foreground and background are identical; contrast is 1:1"
        ))
    } else {
        Ok(())
    }
}

fn check_known_surface(role: &str, foreground: Color, background: Color) -> Result<(), String> {
    check_distinct(role, foreground, background)?;
    if matches!((foreground, background), (Color::Rgb(..), Color::Rgb(..))) {
        check_contrast(role, foreground, background, 4.5)?;
    }
    Ok(())
}

fn check_contrast(
    role: &str,
    foreground: Color,
    background: Color,
    minimum: f64,
) -> Result<(), String> {
    let (Color::Rgb(fr, fg, fb), Color::Rgb(br, bg, bb)) = (foreground, background) else {
        return Err(format!(
            "{role}: both foreground and background must be #RRGGBB to validate contrast"
        ));
    };
    fn luminance(channels: [u8; 3]) -> f64 {
        let linear = channels.map(|v| {
            let c = f64::from(v) / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        });
        linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722
    }
    let a = luminance([fr, fg, fb]);
    let b = luminance([br, bg, bb]);
    let ratio = (a.max(b) + 0.05) / (a.min(b) + 0.05);
    if ratio < minimum {
        Err(format!(
            "{role}: contrast {ratio:.2}:1 is below {minimum}:1; choose a lighter foreground or darker background (or reverse)"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod theme_tests {
    use super::*;
    #[test]
    fn builtin_palettes_validate() {
        MAIN_UI_THEME.validate().unwrap();
        TOKYO_NIGHT_THEME.validate().unwrap();
    }
    #[test]
    fn semantic_and_named_same_color_surfaces_are_rejected() {
        let mut theme = MAIN_UI_THEME;
        theme.foreground = Color::Black;
        theme.background = Color::Black;
        assert!(theme.validate().unwrap_err().contains("base"));
        let mut theme = TOKYO_NIGHT_THEME;
        theme.modal.danger = theme.modal.background.unwrap();
        assert!(theme.validate().unwrap_err().contains("modal.danger"));
        let mut theme = TOKYO_NIGHT_THEME;
        theme.files.modified = theme.background;
        assert!(theme.validate().unwrap_err().contains("files.modified"));
    }
    #[test]
    fn invalid_pairs_and_unknown_roles_are_rejected() {
        let mut theme = MAIN_UI_THEME;
        theme
            .set_color("modal.selection.foreground", theme.modal.surface)
            .unwrap();
        assert!(theme.validate().unwrap_err().contains("modal.selection"));
        assert!(
            theme
                .set_color("selection.background", Color::Blue)
                .is_err()
        );
        assert!(theme.set_color("typo", Color::Red).is_err());
    }
}
