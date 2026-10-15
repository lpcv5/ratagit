# Hunk/diff component

`DiffView` is a controlled Ratatui `StatefulWidget`. It renders a supplied model
and theme into a rectangle. It has no App, dispatcher, repository handle, keymap,
or Git I/O dependency. Selection and repository state belong to the caller.

## Data entry

`DiffViewModel` is the data boundary:

- `lines` and `patch` describe the same display document. During loading, retain
  both from the previous document until the replacement is ready.
  Git supplies the native combined presentation and maps displayed rows to
  reviewed source IDs. Use `row_metadata` and `line_row` for display coordinates;
  raw source-hunk spans remain application data. The view performs no merging.
- `selection` supplies focus, line/hunk mode, cursor IDs, marked IDs, range state,
  and one-based navigation progress. Rendering never edits these values.
  Hunk mode is the default. Its focus backgrounds and selection rail cover only
  added/removed rows of one contiguous edit. `hunk_cursor` and `marked_hunks`
  contain `LineId`s for block starts; the cursor arrow marks the first changed
  line. Context separates blocks even within the same native Git hunk.
  Staged status changes only the green/red edit shading. Headers and context
  have no source badge or source-specific background.
- `scroll` requests page scrolling; cursor movement resumes cursor following.
- `viewport_anchor` optionally maps a row in the displayed document to its row
  in a replacement. Give each request a new `revision`. The view consumes it
  once, preserves that row's screen position through loading, and keeps manual
  scrolling inside large hunks. This also works when the source/request changes.
- `status` supplies loading, applying and read-only state. The owner determines
  command availability; the widget only presents it.
- `empty_message` lets files, commits, stashes and other hosts choose their hint.

Keep one `DiffViewState` per mounted container. It owns only viewport history;
preserve it across snapshot publications and loading frames. Two containers use
two states, even when they render the same document.

Hunk navigation follows the entire change block range. Context padding shrinks
when needed to fit it; an oversized block starts at the top of the content area.
Anchor restoration takes priority over fitting, including on a shorter final
page, so replacement does not pull content upward merely to fill empty space.
Subsequent navigation and resizing resume normal range following.

```rust,ignore
let data = DiffViewModel {
    lines: &document.lines,
    patch: document.patch.as_deref(),
    selection,
    status,
    ..Default::default()
};
frame.render_stateful_widget(DiffView::new(data, &theme), area, &mut view_state);
```

## Behavior binding

`DiffIntent` describes semantic behavior: navigation, selection, mode changes,
staging, unstaging, discarding, file changes and focus. It contains no keycodes
or application action types. `DiffBehaviorBindings` binds an intent to the
owner's action; a closure can also serve as a binding. Returning `None` disables
an unavailable behavior. Resolving a binding has no side effects.

```rust,ignore
// A host can bind to its own action type and dispatcher.
let bindings = |intent| match intent {
    DiffIntent::NextHunk => Some(MyAction::NextHunk),
    DiffIntent::StageSelection if can_stage => Some(MyAction::StageReviewedSelection),
    _ => None,
};
if let Some(action) = bindings.resolve_diff_intent(intent) {
    dispatch(action);
}
```

Ratagit's actual bindings live in `app/commands/diff_bindings.rs`. `CommandEnv`
implements the binding contract using the existing domain selectors. The command
registry converts configured keys and palette commands to the same intents and
uses the same bindings for execution and availability. Stores, reducers and
effects continue to execute returned `DomainAction`s. Rendering never runs a
handler, changes a selection, or applies a patch.

The host adapter `ui/panels/diff_panel.rs` projects snapshots into the model. It
keeps panel-specific hints outside the reusable component. Read-only validation
and cursor progress are supplied by `app/diff/selectors.rs`.
The host also projects successful write relocations into `viewport_anchor`.
Matching old/new Git changes belongs to `app/diff/position.rs`; the component
only applies the supplied row displacement and never inspects Git write state.

## Extension points

| Responsibility | Location |
| --- | --- |
| Public display data and controlled selection | `model.rs` |
| Semantic intents and external behavior bindings | `bindings.rs` |
| Widget/container composition | `mod.rs` |
| Line numbers, selection and staged change colors | `rows.rs` |
| File title, change counts, navigation and operation status | `chrome.rs` |
| Cursor following, page scroll and retained loading position | `viewport.rs` |
| ANSI branch/log text | `ansi.rs` |
| Git navigation rules, patch targets, freshness and writes | `app/diff/` and Git effects |

For a new interaction, add an intent and bind it in the host; keep its reducer
and effects outside the widget. For annotations or new display affordances,
extend the view model and row rendering using the existing `HunkId`/`LineId`
identities. Do not create a second mutable copy of domain selection in the UI.

Component tests render without an App or repository. Existing UI golden tests
and Git interaction regressions verify that the host adapter preserves display,
loading stability, staged grouping, keymaps and write behavior. Intentional
presentation changes update the reviewed golden snapshots.
