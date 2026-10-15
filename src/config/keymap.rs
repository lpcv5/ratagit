use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Documentation comment in English.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalKeymap {
    #[serde(flatten)]
    pub bindings: HashMap<String, Vec<String>>,
}

/// Documentation comment in English.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PanelKeymap {
    #[serde(flatten)]
    pub bindings: HashMap<String, Vec<String>>,
}

/// Documentation comment in English.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keymap {
    #[serde(default)]
    schema_version: u32,
    pub global: GlobalKeymap,
    #[serde(default)]
    pub files: PanelKeymap,
    #[serde(default)]
    pub branches: PanelKeymap,
    #[serde(default)]
    pub commits: PanelKeymap,
    #[serde(default)]
    pub stash: PanelKeymap,
    #[serde(skip)]
    global_key_index: HashMap<String, Vec<String>>,
    #[serde(skip)]
    files_key_index: HashMap<String, Vec<String>>,
    #[serde(skip)]
    branches_key_index: HashMap<String, Vec<String>>,
    #[serde(skip)]
    commits_key_index: HashMap<String, Vec<String>>,
    #[serde(skip)]
    stash_key_index: HashMap<String, Vec<String>>,
}

impl Default for GlobalKeymap {
    fn default() -> Self {
        Self {
            bindings: crate::app::commands::default_bindings(None),
        }
    }
}

impl Default for Keymap {
    fn default() -> Self {
        use crate::app::SidePanel;
        let panel = |panel| PanelKeymap {
            bindings: crate::app::commands::default_bindings(Some(panel)),
        };
        let mut keymap = Self {
            schema_version: 1,
            global: GlobalKeymap::default(),
            files: panel(SidePanel::Files),
            branches: panel(SidePanel::LocalBranches),
            commits: panel(SidePanel::Commits),
            stash: panel(SidePanel::Stash),
            global_key_index: HashMap::new(),
            files_key_index: HashMap::new(),
            branches_key_index: HashMap::new(),
            commits_key_index: HashMap::new(),
            stash_key_index: HashMap::new(),
        };
        keymap.rebuild_indexes();
        keymap
    }
}

impl Keymap {
    pub fn load() -> Self {
        Self::load_from_path(&Self::config_path())
    }

    /// Explicit paths let isolated callers avoid platform-specific user folders.
    pub(crate) fn load_from_path(path: &Path) -> Self {
        let mut defaults = Self::default();
        defaults.rebuild_indexes();
        if path.exists() {
            let content = std::fs::read_to_string(path).unwrap_or_default();
            toml::from_str::<Self>(&content)
                .map(|mut loaded| {
                    loaded.migrate_defaults();
                    loaded.merge_missing(&defaults);
                    loaded.rebuild_indexes();
                    loaded
                })
                .unwrap_or_else(|_| {
                    // Avoid printing to stderr during TUI startup; it corrupts the UI buffer.
                    defaults.save_to(path);
                    defaults
                })
        } else {
            let default = defaults;
            default.save_to(path);
            default
        }
    }

    fn merge_missing(&mut self, defaults: &Self) {
        merge_bindings(&mut self.global.bindings, &defaults.global.bindings);
        merge_bindings(&mut self.files.bindings, &defaults.files.bindings);
        merge_bindings(&mut self.branches.bindings, &defaults.branches.bindings);
        merge_bindings(&mut self.commits.bindings, &defaults.commits.bindings);
        merge_bindings(&mut self.stash.bindings, &defaults.stash.bindings);
    }

    /// Upgrade only exact historical defaults; customized bindings stay intact.
    /// Migration is in memory so loading never rewrites a user's comments.
    fn migrate_defaults(&mut self) {
        if self.schema_version >= 1 {
            return;
        }
        for (name, old, new) in [
            ("focus_diff", &["Tab"][..], &["Enter", "0"][..]),
            ("diff_side", &["Enter"][..], &["Tab"][..]),
            ("hunk_next", &["j", "Down", "]"][..], &["l", "Right"][..]),
            ("hunk_previous", &["k", "Up", "["][..], &["h", "Left"][..]),
            ("hunk_select", &["Space"][..], &[][..]),
            ("hunk_stage", &["s"][..], &[][..]),
            ("hunk_unstage", &["u"][..], &[][..]),
            (
                "panel_next",
                &["l", "Right"][..],
                &["l", "Right", "Tab"][..],
            ),
            (
                "panel_prev",
                &["h", "Left"][..],
                &["h", "Left", "BackTab"][..],
            ),
        ] {
            if self
                .global
                .bindings
                .get(name)
                .is_some_and(|keys| keys.iter().map(String::as_str).eq(old.iter().copied()))
            {
                self.global
                    .bindings
                    .insert(name.into(), new.iter().map(|key| (*key).into()).collect());
            }
        }
        self.schema_version = 1;
    }

    fn save_to(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = toml::to_string_pretty(self) {
            let _ = std::fs::write(path, content);
        }
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ratagit")
            .join("keymap.toml")
    }

    pub fn global_keys(&self, action: &str) -> Vec<&str> {
        self.global
            .bindings
            .get(action)
            .into_iter()
            .flatten()
            .map(String::as_str)
            .collect()
    }

    pub fn panel_keys(&self, panel: &str, action: &str) -> Vec<&str> {
        let map = match panel {
            "files" => &self.files.bindings,
            "branches" => &self.branches.bindings,
            "commits" => &self.commits.bindings,
            "stash" => &self.stash.bindings,
            _ => return Vec::new(),
        };
        map.get(action)
            .into_iter()
            .flatten()
            .map(String::as_str)
            .collect()
    }

    pub fn global_actions(&self, key_str: &str) -> &[String] {
        self.global_key_index
            .get(key_str)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn panel_actions(&self, panel: &str, key_str: &str) -> &[String] {
        let index = match panel {
            "files" => &self.files_key_index,
            "branches" => &self.branches_key_index,
            "commits" => &self.commits_key_index,
            "stash" => &self.stash_key_index,
            _ => return &[],
        };

        index.get(key_str).map(Vec::as_slice).unwrap_or(&[])
    }

    pub(crate) fn rebuild_indexes(&mut self) {
        self.global_key_index = build_key_index(&self.global.bindings);
        self.files_key_index = build_key_index(&self.files.bindings);
        self.branches_key_index = build_key_index(&self.branches.bindings);
        self.commits_key_index = build_key_index(&self.commits.bindings);
        self.stash_key_index = build_key_index(&self.stash.bindings);
    }
}

fn merge_bindings(
    target: &mut HashMap<String, Vec<String>>,
    defaults: &HashMap<String, Vec<String>>,
) {
    for (action, keys) in defaults {
        target.entry(action.clone()).or_insert_with(|| keys.clone());
    }
}

fn build_key_index(bindings: &HashMap<String, Vec<String>>) -> HashMap<String, Vec<String>> {
    let mut index = HashMap::new();
    for (action, keys) in bindings {
        for key in keys {
            index
                .entry(key.clone())
                .or_insert_with(Vec::new)
                .push(action.clone());
        }
    }
    index
}

/// Documentation comment in English.
pub fn key_to_string(key: &crossterm::event::KeyEvent) -> String {
    use crossterm::event::{KeyCode, KeyModifiers};

    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    let base = match key.code {
        KeyCode::Char(' ') => "Space".into(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "Enter".into(),
        KeyCode::Tab => "Tab".into(),
        KeyCode::BackTab => "BackTab".into(),
        KeyCode::Up => "Up".into(),
        KeyCode::Down => "Down".into(),
        KeyCode::Left => "Left".into(),
        KeyCode::Right => "Right".into(),
        KeyCode::Esc => "Esc".into(),
        KeyCode::Backspace => "Backspace".into(),
        KeyCode::Delete => "Delete".into(),
        KeyCode::PageUp => "PageUp".into(),
        KeyCode::PageDown => "PageDown".into(),
        KeyCode::Home => "Home".into(),
        KeyCode::End => "End".into(),
        KeyCode::F(number) => format!("F{number}"),
        _ => return String::new(),
    };

    if ctrl {
        format!("C-{}", base)
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_binding_merges_into_existing_config_and_preserves_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keymap.toml");
        let text = "# preserve comments\nschema_version = 1\n[global]\nquit = ['q']\n";
        std::fs::write(&path, text).unwrap();
        assert_eq!(Keymap::load_from_path(&path).global_keys("help"), ["?"]);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        for keys in ["['F1']", "[]"] {
            let text = format!("{text}help = {keys}\n");
            std::fs::write(&path, &text).unwrap();
            let loaded = Keymap::load_from_path(&path);
            assert_eq!(
                loaded.global_keys("help"),
                if keys == "[]" { vec![] } else { vec!["F1"] }
            );
            assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        }
    }

    #[test]
    fn test_legacy_default_diff_bindings_migrate_without_rewriting_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keymap.toml");
        let text = "# keep this comment\n[global]\nfocus_diff = ['Tab']\ndiff_side = ['Enter']\nhunk_next = ['j', 'Down', ']']\nhunk_previous = ['k', 'Up', '[']\nhunk_select = ['Space']\nhunk_stage = ['s']\nhunk_unstage = ['u']\npanel_next = ['l', 'Right']\npanel_prev = ['h', 'Left']\n";
        std::fs::write(&path, text).unwrap();
        for _ in 0..2 {
            let loaded = Keymap::load_from_path(&path);
            assert_eq!(loaded.global_keys("focus_diff"), ["Enter", "0"]);
            assert_eq!(loaded.global_keys("diff_side"), ["Tab"]);
            assert_eq!(loaded.global_keys("diff_apply"), ["Space"]);
            assert_eq!(loaded.global_keys("hunk_next"), ["l", "Right"]);
            assert_eq!(loaded.global_keys("hunk_previous"), ["h", "Left"]);
            assert_eq!(loaded.global_keys("panel_next"), ["l", "Right", "Tab"]);
            assert_eq!(loaded.global_keys("panel_prev"), ["h", "Left", "BackTab"]);
            for action in ["hunk_select", "hunk_stage", "hunk_unstage"] {
                assert!(loaded.global_keys(action).is_empty());
            }
            assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        }
    }

    #[test]
    fn test_keymap_migration_preserves_custom_and_disabled_bindings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keymap.toml");
        std::fs::write(&path, "[global]\nfocus_diff = ['f']\ndiff_side = ['C-t']\nhunk_select = ['x']\nhunk_stage = []\nhunk_next = ['n']\n").unwrap();
        let loaded = Keymap::load_from_path(&path);
        assert_eq!(loaded.global_keys("focus_diff"), ["f"]);
        assert_eq!(loaded.global_keys("diff_side"), ["C-t"]);
        assert_eq!(loaded.global_keys("hunk_select"), ["x"]);
        assert!(loaded.global_keys("hunk_stage").is_empty());
        assert_eq!(loaded.global_keys("hunk_next"), ["n"]);
        assert_eq!(loaded.global_keys("diff_apply"), ["Space"]);
        std::fs::write(
            &path,
            "schema_version = 1\n[global]\nfocus_diff = ['Tab']\n",
        )
        .unwrap();
        assert_eq!(
            Keymap::load_from_path(&path).global_keys("focus_diff"),
            ["Tab"]
        );
    }
}
