use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ratagit_ui::UiTheme;
use ratatui::style::Color;
use serde::Deserialize;

/// Load the executable's theme configuration before initializing the terminal.
///
/// `RATAGIT_CONFIG` takes precedence over the platform's conventional config
/// directory. An absent implicit file uses the built-in default theme; an
/// explicit path must exist. Every file-related error includes its path.
pub fn load_theme() -> io::Result<UiTheme> {
    match discover_config_path(cfg!(windows), |key| std::env::var_os(key)) {
        Some(config) => load_theme_from_path(&config),
        None => Ok(UiTheme::default()),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ConfigPath {
    path: PathBuf,
    explicit: bool,
}

fn discover_config_path(
    windows: bool,
    mut env: impl FnMut(&str) -> Option<OsString>,
) -> Option<ConfigPath> {
    if let Some(path) = env("RATAGIT_CONFIG") {
        return Some(ConfigPath {
            path: path.into(),
            explicit: true,
        });
    }

    let base = if windows {
        env("APPDATA")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    } else {
        env("XDG_CONFIG_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                env("HOME")
                    .filter(|value| !value.is_empty())
                    .map(|home| PathBuf::from(home).join(".config"))
            })
    }?;
    Some(ConfigPath {
        path: base.join("ratagit").join("config.json"),
        explicit: false,
    })
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Config {
    theme: ThemeConfig,
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ThemeConfig {
    preset: String,
    colors: BTreeMap<String, String>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            preset: "default".to_owned(),
            colors: BTreeMap::new(),
        }
    }
}

fn load_theme_from_path(config: &ConfigPath) -> io::Result<UiTheme> {
    let text = match fs::read_to_string(&config.path) {
        Ok(text) => text,
        Err(error) if !config.explicit && error.kind() == io::ErrorKind::NotFound => {
            return Ok(UiTheme::default());
        }
        Err(error) => {
            let source = if config.explicit {
                "configuration selected by RATAGIT_CONFIG"
            } else {
                "configuration"
            };
            return Err(io::Error::new(
                error.kind(),
                format!("cannot read {source} '{}': {error}", config.path.display()),
            ));
        }
    };

    // Windows editors can emit UTF-8 with a BOM; it is not part of the JSON.
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let parsed: Config = serde_json::from_str(text)
        .map_err(|error| invalid_config(&config.path, format!("invalid JSON: {error}")))?;
    let mut theme = UiTheme::from_preset(&parsed.theme.preset).map_err(|error| {
        invalid_config(
            &config.path,
            format!("theme.preset {:?}: {error}", parsed.theme.preset),
        )
    })?;
    for (role, value) in parsed.theme.colors {
        let color = parse_color(&value).map_err(|error| {
            invalid_config(
                &config.path,
                format!("theme.colors[{role:?}] = {value:?}: {error}"),
            )
        })?;
        theme.set_color(&role, color).map_err(|error| {
            invalid_config(&config.path, format!("theme.colors[{role:?}]: {error}"))
        })?;
    }
    theme
        .validate()
        .map_err(|error| invalid_config(&config.path, format!("invalid theme: {error}")))?;
    Ok(theme)
}

fn invalid_config(path: &Path, message: impl std::fmt::Display) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("invalid configuration '{}': {message}", path.display()),
    )
}

fn parse_color(value: &str) -> Result<Color, String> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('#') {
        if hex.len() == 6 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            let rgb = u32::from_str_radix(hex, 16).map_err(|error| error.to_string())?;
            return Ok(Color::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8));
        }
        return Err("expected an RGB color in #RRGGBB format".to_owned());
    }

    let normalized = value.to_ascii_lowercase();
    if let Some(index) = normalized.strip_prefix("ansi:") {
        if !index.is_empty()
            && index.bytes().all(|byte| byte.is_ascii_digit())
            && let Ok(index) = index.parse::<u8>()
        {
            return Ok(Color::Indexed(index));
        }
        return Err("expected an indexed ANSI color from ansi:0 through ansi:255".to_owned());
    }

    // Support ratatui's color names, plus conventional kebab/snake spellings.
    match normalized.replace(['-', '_'], "").as_str() {
        "default" | "reset" => Ok(Color::Reset),
        "black" => Ok(Color::Black),
        "red" => Ok(Color::Red),
        "green" => Ok(Color::Green),
        "yellow" => Ok(Color::Yellow),
        "blue" => Ok(Color::Blue),
        "magenta" => Ok(Color::Magenta),
        "cyan" => Ok(Color::Cyan),
        "gray" | "grey" => Ok(Color::Gray),
        "darkgray" | "darkgrey" => Ok(Color::DarkGray),
        "lightred" => Ok(Color::LightRed),
        "lightgreen" => Ok(Color::LightGreen),
        "lightyellow" => Ok(Color::LightYellow),
        "lightblue" => Ok(Color::LightBlue),
        "lightmagenta" => Ok(Color::LightMagenta),
        "lightcyan" => Ok(Color::LightCyan),
        "white" => Ok(Color::White),
        _ => Err(
            "expected #RRGGBB, a named ANSI color (such as red, light-blue, or default), or ansi:0 through ansi:255"
                .to_owned(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn discover(windows: bool, values: &[(&str, &str)]) -> Option<ConfigPath> {
        discover_config_path(windows, |key| {
            values
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| OsString::from(value))
        })
    }

    fn load_json(text: &str) -> io::Result<UiTheme> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, text).unwrap();
        load_theme_from_path(&ConfigPath {
            path,
            explicit: true,
        })
    }

    #[test]
    fn explicit_path_takes_precedence_on_both_platforms() {
        let values = [
            ("RATAGIT_CONFIG", "chosen.json"),
            ("APPDATA", "roaming"),
            ("XDG_CONFIG_HOME", "xdg"),
            ("HOME", "home"),
        ];
        for windows in [false, true] {
            assert_eq!(
                discover(windows, &values),
                Some(ConfigPath {
                    path: PathBuf::from("chosen.json"),
                    explicit: true,
                })
            );
        }
    }

    #[test]
    fn windows_uses_appdata_and_unix_prefers_xdg_over_home() {
        let values = [
            ("APPDATA", "roaming"),
            ("XDG_CONFIG_HOME", "xdg"),
            ("HOME", "home"),
        ];
        assert_eq!(
            discover(true, &values).unwrap().path,
            Path::new("roaming").join("ratagit/config.json")
        );
        assert_eq!(
            discover(false, &values).unwrap().path,
            Path::new("xdg").join("ratagit/config.json")
        );
        assert!(!discover(false, &values).unwrap().explicit);
    }

    #[test]
    fn unix_falls_back_to_home_when_xdg_is_missing_or_empty() {
        for values in [
            vec![("HOME", "home")],
            vec![("XDG_CONFIG_HOME", ""), ("HOME", "home")],
        ] {
            assert_eq!(
                discover(false, &values).unwrap().path,
                Path::new("home").join(".config/ratagit/config.json")
            );
        }
        assert_eq!(discover(true, &[("HOME", "home")]), None);
        assert_eq!(discover(false, &[]), None);
        assert_eq!(discover(false, &[("HOME", "")]), None);
    }

    #[test]
    fn empty_explicit_path_remains_explicit_and_fails() {
        let config = discover(false, &[("RATAGIT_CONFIG", ""), ("HOME", "home")]).unwrap();
        assert!(config.explicit);
        assert!(load_theme_from_path(&config).is_err());
    }

    #[test]
    fn missing_implicit_file_uses_defaults_but_explicit_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.json");
        assert_eq!(
            load_theme_from_path(&ConfigPath {
                path: path.clone(),
                explicit: false,
            })
            .unwrap(),
            UiTheme::default()
        );
        let error = load_theme_from_path(&ConfigPath {
            path: path.clone(),
            explicit: true,
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert!(error.to_string().contains(&path.display().to_string()));
        assert!(error.to_string().contains("RATAGIT_CONFIG"));
    }

    #[test]
    fn unreadable_implicit_path_does_not_silently_use_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let error = load_theme_from_path(&ConfigPath {
            path: dir.path().to_path_buf(),
            explicit: false,
        })
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains(&dir.path().display().to_string())
        );
        assert!(error.to_string().contains("cannot read"));
    }

    #[test]
    fn omitted_theme_or_preset_uses_defaults() {
        for text in ["{}", r#"{"theme":{}}"#, r#"{"theme":{"preset":"default"}}"#] {
            assert_eq!(load_json(text).unwrap(), UiTheme::default());
        }
    }

    #[test]
    fn preset_supports_partial_overrides() {
        let mut expected = UiTheme::from_preset("tokyo-night").unwrap();
        expected
            .set_color("accent", Color::Rgb(171, 205, 239))
            .unwrap();
        assert_eq!(
            load_json(r##"{"theme":{"preset":"tokyo-night","colors":{"accent":"#aBcDeF"}}}"##)
                .unwrap(),
            expected
        );
        let mut expected = UiTheme::default();
        expected.set_color("accent", Color::Indexed(255)).unwrap();
        assert_eq!(
            load_json(r#"{"theme":{"colors":{"accent":"ansi:255"}}}"#).unwrap(),
            expected
        );
    }

    #[test]
    fn rgb_paired_overrides_are_validated_after_all_colors_are_applied() {
        let loaded = load_json(
            r##"{"theme":{"colors":{"modal.selection.foreground":"#ffffff","modal.selection.background":"#000000"}}}"##,
        )
        .unwrap();
        let mut expected = UiTheme::default();
        expected
            .set_color("modal.selection.foreground", Color::Rgb(255, 255, 255))
            .unwrap();
        expected
            .set_color("modal.selection.background", Color::Rgb(0, 0, 0))
            .unwrap();
        assert_eq!(loaded, expected);
    }

    #[test]
    fn low_contrast_and_unverifiable_paired_overrides_are_rejected() {
        for text in [
            r##"{"theme":{"colors":{"modal.selection.foreground":"#ffffff","modal.selection.background":"#ffffff"}}}"##,
            r#"{"theme":{"colors":{"modal.selection.foreground":"default"}}}"#,
            r#"{"theme":{"colors":{"modal.selection.foreground":"ansi:0"}}}"#,
            r#"{"theme":{"colors":{"foreground":"black","background":"black"}}}"#,
            r#"{"theme":{"colors":{"modal.foreground":"ansi:1","modal.background":"ansi:1"}}}"#,
            r##"{"theme":{"preset":"tokyo-night","colors":{"modal.danger":"#1A1B26"}}}"##,
        ] {
            let error = load_json(text).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            assert!(error.to_string().contains("config.json"));
        }
    }

    #[test]
    fn malformed_json_unknown_fields_presets_and_roles_include_the_path() {
        for (text, expected) in [
            ("{", "invalid JSON"),
            (r#"{"typo":{}}"#, "typo"),
            (r#"{"theme":{"typo":{}}}"#, "typo"),
            (r#"{"theme":{"preset":"unknown"}}"#, "unknown"),
            (
                r#"{"theme":{"colors":{"unknown.role":"red"}}}"#,
                "unknown.role",
            ),
            (r#"{"theme":{"colors":{"accent":42}}}"#, "invalid JSON"),
            (r#"{"theme":null}"#, "invalid JSON"),
        ] {
            let error = load_json(text).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            let message = error.to_string();
            assert!(message.contains("config.json"), "{message}");
            assert!(message.contains(expected), "{message}");
        }
    }

    #[test]
    fn invalid_color_error_identifies_role_value_and_path() {
        let error = load_json(r##"{"theme":{"colors":{"accent":"#oops"}}}"##).unwrap_err();
        let message = error.to_string();
        for expected in ["config.json", "accent", "#oops", "#RRGGBB"] {
            assert!(message.contains(expected), "{message}");
        }
    }

    #[test]
    fn parses_rgb_named_default_and_indexed_colors() {
        for (text, expected) in [
            ("#142238", Color::Rgb(20, 34, 56)),
            ("#aAbBcC", Color::Rgb(170, 187, 204)),
            ("red", Color::Red),
            ("LightBlue", Color::LightBlue),
            ("light-blue", Color::LightBlue),
            ("dark_gray", Color::DarkGray),
            ("default", Color::Reset),
            ("reset", Color::Reset),
            ("ansi:0", Color::Indexed(0)),
            ("ansi:255", Color::Indexed(255)),
        ] {
            assert_eq!(parse_color(text).unwrap(), expected, "{text}");
        }
        for text in [
            "",
            "#123",
            "#1234567",
            "#gggggg",
            "#éabcd",
            "purple-ish",
            "ansi:",
            "ansi:256",
            "ansi:-1",
            "ansi:+1",
            "ansi:1.5",
        ] {
            assert!(parse_color(text).is_err(), "{text}");
        }
    }

    #[test]
    fn utf8_bom_is_supported_and_invalid_utf8_reports_the_path() {
        assert_eq!(load_json("\u{feff}{}").unwrap(), UiTheme::default());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid-utf8.json");
        fs::write(&path, [0xff, 0xfe]).unwrap();
        let error = load_theme_from_path(&ConfigPath {
            path: path.clone(),
            explicit: true,
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains(&path.display().to_string()));
    }
}
