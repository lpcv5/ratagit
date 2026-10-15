use super::props::*;
use super::*;
use crate::ui::components::atoms::Tone;
use crate::ui::panels::selection::{BranchesViewport, RevisionViewport};
use crate::ui::test_support::render;
use crate::ui::theme::UiTheme;

#[test]
fn test_commit_component_renders_supplied_graph_emphasis_without_domain_owners() {
    let mut viewport = RevisionViewport::default();
    let ui = render(80, 10, |frame| {
        draw_commits_panel(
            frame,
            frame.area(),
            CommitsPanelProps {
                header: PanelHeader {
                    title: "Owner supplied title".into(),
                    is_active: false,
                    selected: None,
                    search_query: None,
                },
                content: RevisionContent::List(vec![CommitRowProps {
                    oid: "123456789abcdef",
                    author: "Ada Lovelace",
                    message: "Owner supplied row",
                    tone: Tone::Success,
                    graph: vec![
                        GraphCellProps {
                            text: "●",
                            dimmed: false,
                        },
                        GraphCellProps {
                            text: "│",
                            dimmed: true,
                        },
                    ],
                }]),
            },
            &mut viewport,
        );
    });
    let text: String = ui.buffer.content.iter().map(|cell| cell.symbol()).collect();
    assert!(text.contains("Owner supplied title"));
    assert!(text.contains("1234567 AL Owner supplied row"));
    let dot = ui
        .buffer
        .content
        .iter()
        .find(|cell| cell.symbol() == "●")
        .unwrap();
    assert_eq!(dot.fg, UiTheme::default().success);
    assert_eq!(ui.buffer[(2, 1)].symbol(), "│");
    assert_eq!(ui.buffer[(2, 1)].fg, UiTheme::default().text_muted);
}

#[test]
fn test_branch_history_component_uses_supplied_inactive_focus() {
    let mut viewport = BranchesViewport::default();
    let ui = render(80, 10, |frame| {
        draw_branches_panel(
            frame,
            frame.area(),
            BranchesPanelProps::Commits {
                props: CommitsPanelProps {
                    header: PanelHeader {
                        title: "Supplied history".into(),
                        is_active: false,
                        selected: Some(0),
                        search_query: None,
                    },
                    content: RevisionContent::List(Vec::new()),
                },
                view_id: Some("opaque history id"),
            },
            &mut viewport,
        );
    });
    assert_eq!(ui.buffer[(0, 1)].fg, UiTheme::default().border_inactive);
    assert_eq!(viewport.commits.state(false, None).selected(), Some(0));
}
