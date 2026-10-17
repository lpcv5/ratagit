use ratagit_core::{
    Action, AppContext, Command, GitResult, PanelFocus, RepositoryItem as I, RepositoryTab as T,
    UiAction, update,
};
use ratagit_ui::{TerminalSize, render_terminal_buffer, render_terminal_text};
use ratatui::style::Modifier;

fn fixture(tab: T) -> I {
    match tab {
        T::Submodules => I::Submodule {
            path: "vendor/parser".into(),
            url: "https://example.invalid/parser.git".into(),
            head: "abcdef1234567890".into(),
            status: "uninitialized".into(),
        },
        T::Worktrees => I::Worktree {
            path: "/projects/linked".into(),
            head: "abcdef1234567890".into(),
            branch: "feature/tabs".into(),
            detached: false,
            locked: Some("in use".into()),
            prunable: None,
            main: false,
        },
        T::Remotes => I::Remote {
            name: "origin".into(),
            fetch_url: "https://example.invalid/repo.git".into(),
            push_url: "ssh://example.invalid/repo.git".into(),
        },
        T::Tags => I::Tag {
            name: "v1.0".into(),
            target: "abcdef1234567890".into(),
            summary: "Release navigation tabs".into(),
        },
        T::Reflog => I::Reflog {
            selector: "HEAD@{0}".into(),
            target: "abcdef1234567890".into(),
            message: "commit: navigation tabs".into(),
        },
    }
}
fn state(tab: T) -> (AppContext, u64) {
    let mut state = AppContext::default();
    update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(ratagit_testkit::fixture_dirty_repo())),
    );
    update(
        &mut state,
        Action::Ui(UiAction::FocusPanel { panel: tab.panel() }),
    );
    loop {
        let commands = update(
            &mut state,
            Action::Ui(UiAction::CyclePanelTab { previous: false }),
        );
        if let Some(Command::LoadRepositoryTab {
            tab: loaded,
            request_id,
        }) = commands.first()
        {
            if *loaded == tab {
                return (state, *request_id);
            }
        }
    }
}
fn complete(state: &mut AppContext, tab: T, request_id: u64, result: Result<Vec<I>, String>) {
    update(
        state,
        Action::GitResult(GitResult::RepositoryTabLoaded {
            tab,
            request_id,
            result,
        }),
    );
}

#[test]
fn repository_tabs_render_data_details_active_style_and_narrow_titles() {
    let size = TerminalSize {
        width: 150,
        height: 32,
    };
    let mut gallery = String::new();
    for tab in [T::Submodules, T::Worktrees, T::Remotes, T::Tags, T::Reflog] {
        let (mut state, request_id) = state(tab);
        let item = fixture(tab);
        let key = match &item {
            I::Remote { name, .. } => name.clone(),
            _ => item.key(),
        };
        complete(&mut state, tab, request_id, Ok(vec![item]));
        let screen = render_terminal_text(&state, size);
        assert!(screen.contains(&key), "{tab:?}: {screen}");
        assert!(screen.contains("read-only"));
        let buffer = render_terminal_buffer(&state, size);
        let active = buffer.content().iter().any(|cell| {
            cell.modifier
                .contains(Modifier::BOLD | Modifier::UNDERLINED)
        });
        assert!(active, "active tab must be distinctly styled");
        let narrow = render_terminal_text(
            &state,
            TerminalSize {
                width: 70,
                height: 26,
            },
        );
        assert!(
            narrow.lines().any(|line| line.starts_with(&format!(
                "╭ {} ",
                match tab.panel() {
                    PanelFocus::Files => "1",
                    PanelFocus::Branches => "2",
                    _ => "3",
                }
            )) && line
                .split('╮')
                .next()
                .unwrap_or("")
                .contains(tab.title())),
            "active tab survives in the panel header: {narrow}"
        );
        assert!(ratagit_ui::buffer_contains_selected_text(&buffer, &key));
        let text = ratagit_ui::render(&state, size).as_text();
        assert!(text.contains(tab.title()));
        gallery.push_str(&format!("--- {} ---\n", tab.title()));
        for line in screen.lines() {
            gallery.push_str(line.trim_end());
            gallery.push('\n');
        }
        update(
            &mut state,
            Action::Ui(UiAction::FocusPanel {
                panel: PanelFocus::Details,
            }),
        );
        assert!(render_terminal_text(&state, size).contains(&key));
    }
    insta::assert_snapshot!("repository_tabs", gallery);
}

#[test]
fn repository_tab_loading_empty_and_error_states_are_visible() {
    let size = TerminalSize {
        width: 100,
        height: 28,
    };
    let (mut state, id) = state(T::Remotes);
    assert!(render_terminal_text(&state, size).contains("Loading Remotes"));
    complete(&mut state, T::Remotes, id, Ok(Vec::new()));
    assert!(render_terminal_text(&state, size).contains("No remotes"));
    let commands = update(&mut state, Action::Ui(UiAction::RefreshAll));
    let id = commands
        .into_iter()
        .find_map(|cmd| match cmd {
            Command::LoadRepositoryTab { request_id, .. } => Some(request_id),
            _ => None,
        })
        .unwrap();
    complete(
        &mut state,
        T::Remotes,
        id,
        Err("remote configuration is unreadable".into()),
    );
    assert!(render_terminal_text(&state, size).contains("remote configuration is unreadable"));
}
