# Harness Scenarios

This file is generated from `libs/ratagit-harness/tests/harness.rs`.
Detailed assertions live in the Rust scenario definitions.

Total scenarios: 80

## Global (1)

- `harness_global_pull_and_push_sync_repo`

## Files (30)

- `harness_files_stage_and_unstage`
- `harness_files_details_follow_cursor_with_combined_diff_sections`
- `harness_files_details_reuses_cached_diff_when_selection_repeats`
- `harness_files_details_show_untracked_file_diff`
- `harness_files_details_ctrl_scroll_moves_details_content_without_git_change`
- `harness_files_tree_expand_collapse`
- `harness_files_tree_compacts_single_child_directory_chain`
- `harness_files_space_toggles_directory_stage`
- `harness_files_multi_select_stashes_selected_targets`
- `harness_files_commit_editor_multiline_confirm`
- `harness_files_commit_editor_reports_terminal_cursor`
- `harness_files_stash_editor_all_mode`
- `harness_files_stash_editor_multiselect_mode`
- `harness_files_v_marks_individual_rows`
- `harness_files_visual_multiselect_escape_exits_range`
- `harness_files_search_jumps_and_clears`
- `harness_files_reset_menu_select_list_renders_all_short_choices`
- `harness_files_reset_mixed_menu`
- `harness_files_reset_hard_menu`
- `harness_files_reset_nuke_menu`
- `harness_files_reset_hard_requires_confirmation`
- `harness_files_reset_hard_confirmation_can_cancel`
- `harness_files_discard_confirmation_modal_renders`
- `harness_files_discard_confirmation_modal_renders_fullscreen`
- `harness_files_discard_current_target_with_confirmation`
- `harness_files_discard_visual_targets_with_confirmation`
- `harness_files_discard_confirmation_can_cancel`
- `harness_files_scroll_keeps_selection_visible`
- `harness_files_reversing_up_does_not_jump_to_top_reserve`
- `harness_files_reversing_down_does_not_jump_to_bottom_reserve`

## Branches (12)

- `harness_branches_reversing_inside_threshold_keeps_scroll_stable`
- `harness_branches_visual_multiselect_marks_rows`
- `harness_branches_create_and_checkout`
- `harness_branch_details_follow_cursor_with_log_graph`
- `harness_branches_create_from_selected_branch`
- `harness_branches_enter_commits_and_commit_files_subviews`
- `harness_branches_dirty_checkout_with_auto_stash_confirm`
- `harness_branches_delete_local_branch`
- `harness_branches_delete_remote_requires_confirmation`
- `harness_branches_delete_remote_after_confirmation`
- `harness_branches_delete_current_branch_is_protected`
- `harness_branches_rebase_simple_and_origin_main`

## Commits (17)

- `harness_commits_create_and_refresh`
- `harness_commits_create_without_staged_changes_prompts_stage_all`
- `harness_commits_visual_multiselect_marks_rows`
- `harness_commits_visual_multiselect_escape_exits_range`
- `harness_commits_details_follow_cursor_with_commit_diff`
- `harness_commits_details_renders_truncated_commit_diff_notice`
- `harness_commits_enter_files_subpanel_and_follow_file_cursor`
- `harness_commits_files_directory_uses_directory_pathspec`
- `harness_commits_lazy_loads_next_page_when_scrolling_past_first_hundred`
- `harness_commits_prefetches_next_page_before_tail`
- `harness_commits_squash_multiselect`
- `harness_commits_fixup_selected`
- `harness_commits_reword_selected`
- `harness_commits_amend_staged_changes_into_selected_commit`
- `harness_commits_amend_without_staged_changes_prompts_stage_all`
- `harness_commits_delete_selected`
- `harness_commits_detached_checkout_uses_auto_stash`

## Commit Files (4)

- `harness_commit_files_subpanel_keeps_commits_panel_height`
- `harness_commit_files_search_selects_file_and_refreshes_diff`
- `harness_commit_files_visual_multiselect_marks_rows_and_refreshes_diff`
- `harness_commit_files_tree_toggle_reopens_shared_tree_rows`

## Stash (2)

- `harness_stash_search_selects_match_without_git_operation`
- `harness_stash_push_and_pop`

## Large Repo (4)

- `harness_large_repo_fast_status_shows_notice_without_full_refresh`
- `harness_large_repo_fast_status_is_stable_with_tracing_enabled`
- `harness_large_repo_files_tree_expand_uses_lightweight_projection`
- `harness_huge_repo_status_skips_file_scan_without_blocking_commits`

## UI (2)

- `harness_app_context_categorizes_branch_ui_and_repo_state`
- `harness_panel_titles_are_badged_and_empty_placeholders_hidden`

## Error (1)

- `harness_error_visible_without_crash`

## Other (7)

- `harness_status_refresh`
- `harness_command_palette_executes_global_pull`
- `harness_large_directory_details_limits_diff_targets`
- `harness_details_keeps_previous_content_while_new_diff_is_pending`
- `harness_left_panel_search_selects_branch_and_commit_matches`
- `harness_untracked_directory_marker_displays_as_tree_directory`
- `harness_focus_panel_shortcuts_follow_focus`
