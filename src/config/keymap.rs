use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Documentation comment in English.
#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalKeymap {
    #[serde(flatten)]
    pub bindings: HashMap<String, Vec<String>>,
}

/// Documentation comment in English.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PanelKeymap {
    #[serde(flatten)]
    pub bindings: HashMap<String, Vec<String>>,
}

/// Documentation comment in English.
#[derive(Debug, Serialize, Deserialize)]
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
}

impl Default for GlobalKeymap {
    fn default() -> Self {
        let mut b = HashMap::new();
        b.insert("quit".into(), vec!["q".into()]);
        b.insert("refresh".into(), vec!["r".into()]);
        b.insert("panel_next".into(), vec!["l".into(), "Right".into()]);
        b.insert("panel_prev".into(), vec!["h".into(), "Left".into()]);
        b.insert("panel_1".into(), vec!["1".into()]);
        b.insert("panel_2".into(), vec!["2".into()]);
        b.insert("panel_3".into(), vec!["3".into()]);
        b.insert("panel_4".into(), vec!["4".into()]);
        b.insert("list_up".into(), vec!["k".into(), "Up".into()]);
        b.insert("list_down".into(), vec!["j".into(), "Down".into()]);
        b.insert("diff_scroll_up".into(), vec!["C-u".into()]);
        b.insert("diff_scroll_down".into(), vec!["C-d".into()]);
        b.insert("commit".into(), vec!["c".into()]);
        b.insert("search_start".into(), vec!["/".into()]);
        b.insert("search_next".into(), vec!["n".into()]);
        b.insert("search_prev".into(), vec!["N".into()]);
        Self { bindings: b }
    }
}

impl Default for Keymap {
    fn default() -> Self {
        let mut files = HashMap::new();
        files.insert("toggle_stage".into(), vec!["Space".into()]);
        files.insert("toggle_visual_select".into(), vec!["v".into()]);
        files.insert("toggle_dir".into(), vec!["Enter".into()]);
        files.insert("collapse_all".into(), vec!["-".into()]);
        files.insert("expand_all".into(), vec!["=".into()]);
        files.insert("stash_push".into(), vec!["s".into()]);

        let mut branches = HashMap::new();
        branches.insert("checkout_branch".into(), vec!["Enter".into()]);
        branches.insert("create_branch".into(), vec!["n".into()]);
        branches.insert("delete_branch".into(), vec!["d".into()]);
        branches.insert("fetch_remote".into(), vec!["f".into()]);

        let mut stash = HashMap::new();
        stash.insert("open_tree".into(), vec!["Enter".into()]);
        stash.insert("stash_apply".into(), vec!["a".into()]);
        stash.insert("stash_pop".into(), vec!["p".into()]);
        stash.insert("stash_drop".into(), vec!["d".into()]);
        let mut commits = HashMap::new();
        commits.insert("open_tree".into(), vec!["Enter".into()]);

        Self {
            global: GlobalKeymap::default(),
            files: PanelKeymap { bindings: files },
            branches: PanelKeymap { bindings: branches },
            commits: PanelKeymap { bindings: commits },
            stash: PanelKeymap { bindings: stash },
        }
    }
}

impl Keymap {
    pub fn load() -> Self {
        let defaults = Self::default();
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            toml::from_str::<Self>(&content)
                .map(|mut loaded| {
                    loaded.merge_missing(&defaults);
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

    pub fn global_matches(&self, action: &str, key_str: &str) -> bool {
        self.global
            .bindings
            .get(action)
            .map(|keys| keys.iter().any(|k| k == key_str))
            .unwrap_or(false)
    }

    pub fn panel_matches(&self, panel: &str, action: &str, key_str: &str) -> bool {
        let map = match panel {
            "files" => &self.files,
            "branches" => &self.branches,
            "commits" => &self.commits,
            "stash" => &self.stash,
            _ => return false,
        };
        map.bindings
            .get(action)
            .map(|keys| keys.iter().any(|k| k == key_str))
            .unwrap_or(false)
    }

    pub fn first_global_key(&self, action: &str) -> Option<String> {
        self.global
            .bindings
            .get(action)
            .and_then(|keys| keys.first())
            .cloned()
    }

    pub fn first_panel_key(&self, panel: &str, action: &str) -> Option<String> {
        let map = match panel {
            "files" => &self.files.bindings,
            "branches" => &self.branches.bindings,
            "commits" => &self.commits.bindings,
            "stash" => &self.stash.bindings,
            _ => return None,
        };

        map.get(action).and_then(|keys| keys.first()).cloned()
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
