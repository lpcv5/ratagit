//! Feature behavior runs without AppState, Flux, a repository or terminal widgets.
use super::*;
use crate::app::ViewContext;

fn query(
    state: &mut SearchState,
    scope: &ViewContext,
    query: &str,
    items: &[&str],
) -> SearchUpdate {
    let items: Vec<_> = items.iter().map(|item| (*item).to_owned()).collect();
    state.update(
        SearchAction::SetQuery {
            query: query.into(),
            items: &items,
        },
        SearchContext {
            scope,
            selected: None,
        },
    )
}

fn restore(state: &mut SearchState, scope: &ViewContext, items: &[&str]) -> SearchUpdate {
    let items: Vec<_> = items.iter().map(|item| (*item).to_owned()).collect();
    state.update(
        SearchAction::Restore { items: &items },
        SearchContext {
            scope,
            selected: None,
        },
    )
}

#[test]
fn test_matching_is_case_insensitive_unicode_substring_search_without_trimming() {
    let mut state = SearchState::default();
    let scope = ViewContext::Files;
    assert_eq!(
        query(
            &mut state,
            &scope,
            "UI",
            &["UI change", "other", "build ui"]
        )
        .match_count,
        2
    );
    assert_eq!(state.view().matches, &[0, 2]);
    assert_eq!(
        query(
            &mut state,
            &scope,
            "界面",
            &["修复界面", "other", "界面.rs"]
        )
        .match_count,
        2
    );
    assert_eq!(
        query(&mut state, &scope, " UI ", &["UI", "before ui after"]).match_count,
        1
    );
    assert_eq!(state.view().matches, &[1]);
    assert_eq!(
        query(&mut state, &scope, "ß", &["SS", "Straße"]).match_count,
        1
    );
    assert_eq!(state.view().matches, &[1]);
}

#[test]
fn test_every_view_and_revision_source_has_an_independent_saved_query() {
    let scopes = [
        ViewContext::Files,
        ViewContext::Branches,
        ViewContext::Commits,
        ViewContext::Stashes,
        ViewContext::BranchCommits { branch: "a".into() },
        ViewContext::BranchCommits { branch: "b".into() },
        ViewContext::CommitFiles { oid: "a".into() },
        ViewContext::CommitFiles { oid: "b".into() },
        ViewContext::StashFiles { index: None },
        ViewContext::StashFiles { index: Some(1) },
        ViewContext::StashFiles { index: Some(12) },
    ];
    let mut state = SearchState::default();
    for (index, scope) in scopes.iter().enumerate() {
        query(&mut state, scope, &format!("query {index}"), &[]);
    }
    for (index, scope) in scopes.iter().enumerate() {
        let expected = format!("query {index}");
        restore(&mut state, scope, &["unmatched"]);
        assert_eq!(state.view().query, expected);
        assert_eq!(state.view().saved_query(scope), Some(expected.as_str()));
        assert_eq!(state.view().query_for_scope(scope), Some(expected.as_str()));
    }
}

#[test]
fn test_restore_recomputes_matches_after_data_replacement_without_navigation() {
    let mut state = SearchState::default();
    let scope = ViewContext::Commits;
    query(&mut state, &scope, "target", &["target", "other"]);
    assert_eq!(state.view().matches, &[0]);
    let update = restore(&mut state, &scope, &["other", "target again", "TARGET"]);
    assert_eq!(
        update,
        SearchUpdate {
            match_count: 2,
            navigate_to: None
        }
    );
    assert_eq!(state.view().matches, &[1, 2]);
}

#[test]
fn test_restore_without_saved_query_clears_stale_current_matches() {
    let mut state = SearchState::default();
    state.set_test_current("stale".into(), vec![3, 5]);
    restore(&mut state, &ViewContext::Commits, &["stale"]);
    assert!(state.view().query.is_empty());
    assert!(state.view().matches.is_empty());
}

#[test]
fn test_capture_scope_retains_current_query_and_matches_without_saving_or_restoring() {
    let mut state = SearchState::default();
    query(&mut state, &ViewContext::Files, "old", &["old"]);
    let update = state.update(
        SearchAction::CaptureScope,
        SearchContext {
            scope: &ViewContext::Branches,
            selected: None,
        },
    );
    assert_eq!(update.match_count, 1);
    assert_eq!(state.view().query, "old");
    assert_eq!(state.view().matches, &[0]);
    assert_eq!(state.view().scope, &ViewContext::Branches);
    assert!(state.view().saved_query(&ViewContext::Branches).is_none());
}

#[test]
fn test_clear_removes_only_the_captured_scope_and_keeps_other_saved_queries() {
    let mut state = SearchState::default();
    query(&mut state, &ViewContext::Files, "file", &["file"]);
    query(&mut state, &ViewContext::Commits, "commit", &["commit"]);
    state.update(
        SearchAction::Clear,
        SearchContext {
            scope: &ViewContext::Files,
            selected: None,
        },
    );
    assert!(state.view().query.is_empty());
    assert!(state.view().matches.is_empty());
    assert!(state.view().saved_query(&ViewContext::Files).is_none());
    restore(&mut state, &ViewContext::Commits, &["commit"]);
    assert_eq!(state.view().query, "commit");
}

#[test]
fn test_empty_query_removes_scope_query_and_does_not_match_every_item() {
    let mut state = SearchState::default();
    query(&mut state, &ViewContext::Files, "a", &["a"]);
    assert_eq!(
        query(&mut state, &ViewContext::Files, "", &["a", "b"]).match_count,
        0
    );
    assert!(!state.view().has_query(&ViewContext::Files));
    assert!(state.view().saved_query(&ViewContext::Files).is_none());
}

#[test]
fn test_initial_match_is_inclusive_and_subsequent_jumps_are_strict_and_wrap() {
    let mut state = SearchState::default();
    let scope = ViewContext::Files;
    query(
        &mut state,
        &scope,
        "hit",
        &["x", "hit", "x", "hit", "x", "hit"],
    );
    for (action, selected, expected) in [
        (SearchAction::SelectInitial, Some(3), 3),
        (SearchAction::Next, Some(3), 5),
        (SearchAction::Previous, Some(3), 1),
        (SearchAction::Next, Some(5), 1),
        (SearchAction::Previous, Some(1), 5),
        (SearchAction::SelectInitial, Some(99), 1),
        (SearchAction::SelectInitial, None, 1),
        (SearchAction::Next, None, 1),
        (SearchAction::Previous, None, 5),
    ] {
        assert_eq!(
            state
                .update(
                    action,
                    SearchContext {
                        scope: &scope,
                        selected
                    }
                )
                .navigate_to,
            Some(expected)
        );
    }
}

#[test]
fn test_one_match_still_returns_navigation_intent_when_cursor_stays_on_it() {
    let mut state = SearchState::default();
    query(&mut state, &ViewContext::Files, "hit", &["hit"]);
    for action in [SearchAction::Next, SearchAction::Previous] {
        assert_eq!(
            state
                .update(
                    action,
                    SearchContext {
                        scope: &ViewContext::Files,
                        selected: Some(0)
                    }
                )
                .navigate_to,
            Some(0)
        );
    }
}

#[test]
fn test_navigation_is_rejected_for_inactive_scope_or_empty_matches() {
    let mut state = SearchState::default();
    query(&mut state, &ViewContext::Files, "hit", &["hit"]);
    for action in [
        SearchAction::SelectInitial,
        SearchAction::Next,
        SearchAction::Previous,
    ] {
        assert!(state
            .update(
                action,
                SearchContext {
                    scope: &ViewContext::Branches,
                    selected: None
                }
            )
            .navigate_to
            .is_none());
    }
    query(&mut state, &ViewContext::Files, "absent", &["hit"]);
    assert!(state
        .update(
            SearchAction::Next,
            SearchContext {
                scope: &ViewContext::Files,
                selected: None
            }
        )
        .navigate_to
        .is_none());
}

#[test]
fn test_projection_distinguishes_query_without_matches_and_hides_inactive_scope() {
    let mut state = SearchState::default();
    query(&mut state, &ViewContext::Files, "missing", &["other"]);
    assert!(state.view().has_query(&ViewContext::Files));
    assert!(!state.view().has_search(&ViewContext::Files));
    assert_eq!(
        state.view().summary(&ViewContext::Files, None).as_deref(),
        Some("/missing 0/0")
    );
    assert!(state
        .view()
        .query_for_scope(&ViewContext::Branches)
        .is_none());
    assert!(state.view().summary(&ViewContext::Branches, None).is_none());
}

#[test]
fn test_summary_keeps_current_match_position_and_first_match_fallback() {
    let mut state = SearchState::default();
    query(&mut state, &ViewContext::Files, "HIT", &["hit", "x", "hit"]);
    assert_eq!(
        state
            .view()
            .summary(&ViewContext::Files, Some(2))
            .as_deref(),
        Some("/HIT 2/2")
    );
    assert_eq!(
        state
            .view()
            .summary(&ViewContext::Files, Some(1))
            .as_deref(),
        Some("/HIT 1/2")
    );
    assert_eq!(
        state.view().summary(&ViewContext::Files, None).as_deref(),
        Some("/HIT 1/2")
    );
}
