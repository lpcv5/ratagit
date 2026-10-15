//! Components can render owner-provided data without an App or a Flux runtime.
use super::props::*;
use super::*;
use crate::app::commands::CommandHelp;
use crate::app::CommitFieldFocus;
use crate::ui::test_support::{render, RenderedUi};

fn content(ui: &RenderedUi) -> String {
    ui.buffer.content.iter().map(|cell| cell.symbol()).collect()
}

#[test]
fn test_palette_displays_owner_supplied_commands_and_bindings() {
    let command = CommandHelp {
        name: "owner action",
        aliases: &["owner action"],
        label: "Provided by container".into(),
        keys: vec!["F12".into()],
    };
    let hints = vec![("F2".into(), "Owner confirm".into())];
    let ui = render(140, 40, |frame| {
        render_command_palette(
            frame,
            CommandPaletteProps {
                // The owner has already selected results, even if its matching
                // policy differs from Ratagit's command palette policy.
                input: "unrelated search",
                selected: 0,
                commands: vec![&command],
                hints: &hints,
            },
        );
    });
    let text = content(&ui);
    assert!(text.contains("F12  owner action  (Provided by container)"));
    assert!(text.contains("F2  Owner confirm"));
    assert!(text.contains("Commands (1)"));
}

#[test]
fn test_editor_and_shortcut_bar_render_without_application_state() {
    let hints = vec![("Ctrl+s".into(), "Save draft".into())];
    let editor = render(140, 40, |frame| {
        render_commit_editor(
            frame,
            CommitEditorProps {
                error: None,
                focus: CommitFieldFocus::Description,
                message: "Supplied title",
                description: "First line\nSecond line",
                hints: &hints,
            },
        );
    });
    assert!(content(&editor).contains("Supplied title"));
    assert!(content(&editor).contains("Description · optional"));
    assert!(content(&editor).contains("Ctrl+s  Save draft"));
    assert!(editor.cursor.is_some());

    let draw_bar = || {
        render(100, 1, |frame| {
            render_shortcut_bar(
                frame,
                frame.area(),
                ShortcutBarProps {
                    hints: &hints,
                    search_input: Some("provided query"),
                    loading_frame: Some(3),
                },
            );
        })
    };
    let first = draw_bar();
    assert_eq!(first.buffer, draw_bar().buffer);
    assert!(content(&first).contains("/provided query"));
}
