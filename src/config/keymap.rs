use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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
        let mut defaults = Self::default();
        defaults.rebuild_indexes();
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            toml::from_str::<Self>(&content)
                .map(|mut loaded| {
                    loaded.merge_missing(&defaults);
                    loaded.rebuild_indexes();
                    loaded
                })
                .unwrap_or_else(|_| {
                    // Avoid printing to stderr during TUI startup; it corrupts the UI buffer.
                    defaults.save();
                    defaults
                })
        } else {
            let default = defaults;
            default.save();
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

    fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = toml::to_string_pretty(self) {
            let _ = std::fs::write(&path, content);
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
        _ => return String::new(),
    };

    if ctrl {
        format!("C-{}", base)
    } else {
        base
    }
}
