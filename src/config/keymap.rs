use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 全局快捷键（所有面板生效）
#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalKeymap {
    #[serde(flatten)]
    pub bindings: HashMap<String, Vec<String>>,
}

/// 面板本地快捷键
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PanelKeymap {
    #[serde(flatten)]
    pub bindings: HashMap<String, Vec<String>>,
}

/// 完整 keymap 配置
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
        b.insert("quit".into(),             vec!["q".into()]);
        b.insert("refresh".into(),          vec!["r".into()]);
        b.insert("panel_next".into(),       vec!["l".into(), "Right".into()]);
        b.insert("panel_prev".into(),       vec!["h".into(), "Left".into()]);
        b.insert("panel_1".into(),          vec!["1".into()]);
        b.insert("panel_2".into(),          vec!["2".into()]);
        b.insert("panel_3".into(),          vec!["3".into()]);
        b.insert("panel_4".into(),          vec!["4".into()]);
        b.insert("list_up".into(),          vec!["k".into(), "Up".into()]);
        b.insert("list_down".into(),        vec!["j".into(), "Down".into()]);
        b.insert("diff_scroll_up".into(),   vec!["C-u".into()]);
        b.insert("diff_scroll_down".into(), vec!["C-d".into()]);
        Self { bindings: b }
    }
}

impl Default for Keymap {
    fn default() -> Self {
        let mut files = HashMap::new();
        files.insert("toggle_dir".into(),  vec!["Enter".into(), "Space".into()]);
        files.insert("collapse_all".into(), vec!["-".into()]);
        files.insert("expand_all".into(),   vec!["=".into()]);

        Self {
            global: GlobalKeymap::default(),
            files: PanelKeymap { bindings: files },
            branches: PanelKeymap::default(),
            commits: PanelKeymap::default(),
            stash: PanelKeymap::default(),
        }
    }
}

impl Keymap {
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_else(|_| {
                eprintln!("Warning: invalid keymap config, using defaults");
                Self::default()
            })
        } else {
            let default = Self::default();
            default.save();
            default
        }
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
        self.global.bindings
            .get(action)
            .map(|keys| keys.iter().any(|k| k == key_str))
            .unwrap_or(false)
    }

    pub fn panel_matches(&self, panel: &str, action: &str, key_str: &str) -> bool {
        let map = match panel {
            "files"    => &self.files,
            "branches" => &self.branches,
            "commits"  => &self.commits,
            "stash"    => &self.stash,
            _ => return false,
        };
        map.bindings
            .get(action)
            .map(|keys| keys.iter().any(|k| k == key_str))
            .unwrap_or(false)
    }
}

/// 将 crossterm KeyEvent 转换为字符串表示
pub fn key_to_string(key: &crossterm::event::KeyEvent) -> String {
    use crossterm::event::{KeyCode, KeyModifiers};

    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    let base = match key.code {
        KeyCode::Char(' ') => "Space".into(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter    => "Enter".into(),
        KeyCode::Tab      => "Tab".into(),
        KeyCode::BackTab  => "BackTab".into(),
        KeyCode::Up       => "Up".into(),
        KeyCode::Down     => "Down".into(),
        KeyCode::Left     => "Left".into(),
        KeyCode::Right    => "Right".into(),
        KeyCode::Esc      => "Esc".into(),
        KeyCode::Backspace => "Backspace".into(),
        KeyCode::Delete   => "Delete".into(),
        KeyCode::PageUp   => "PageUp".into(),
        KeyCode::PageDown => "PageDown".into(),
        KeyCode::Home     => "Home".into(),
        KeyCode::End      => "End".into(),
        _ => return String::new(),
    };

    if ctrl { format!("C-{}", base) } else { base }
}
