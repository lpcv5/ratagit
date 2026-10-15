# UI component library

Reusable terminal components live here. Components accept data and `UiTheme`
tokens; they do not read application state, dispatch actions, or perform Git I/O.
Feature panels in `organisms/` receive controlled display props from
`organisms/props.rs`. Overlay, log and footer components receive narrow display
props from `ui/panels/props.rs`.
Only the layout container receives an application snapshot; the adapter in
`flux/snapshot/presentation.rs` projects it into component inputs.

Command filtering, diff availability, read-only validation and selection progress
belong to the application adapter and selectors. Components format those inputs,
arrange child widgets and retain viewport state. The container selects the active
overlay. Animation frames are explicit footer inputs so component rendering does
not read a clock.

`flux/snapshot/presentation/side_panels.rs` chooses branch history, commit/stash
lists and file trees. It supplies titles, focus, scope-specific queries, selected
indices and opaque viewport identities. Graph ownership matching is resolved to
per-cell dimming flags, and commit sync state is projected to a semantic tone.
Side-panel components do not import application feature state or selectors.
The shared revision container mounts the supplied list/tree display variant and
keeps independent viewport histories. Icon/color mapping, author initials,
search span styling, responsive geometry and scrolling remain UI responsibilities.

Keyboard events are bound by `flux/input_mapper.rs` through `app/commands.rs`;
the registry supplies configured keys, availability and returned actions. Diff
behavior ports bind semantic intents in `app/commands/diff_bindings.rs`. The
runtime sends actions to the dispatcher, stores and effects. New event handlers
must follow this path instead of invoking Git operations during rendering.

| Layer | Component | Usage |
| --- | --- | --- |
| Atoms | `badge(label, Tone, &theme)` | Neutral counts and semantic status labels |
| Atoms | `keycap(label, &theme)` | A key label with a raised background |
| Molecules | `card(&theme, title, focused)` | Shared rounded panel and input chrome; returns an owned `Block` |
| Molecules | `shortcut_hint(keys, label, &theme)` | Keycap and description spans for footers and modal actions |
| Molecules | `EmptyState::new(message, &theme)` | Centered message in spacious cards, compact text in small panels |
| Molecules | `render_modal(frame, area, title, &theme)` | Clears covered content, draws a card, returns its inner rectangle |
| Feature containers | `DiffView::new(DiffViewModel, &theme)` | Controlled hunk/diff widget with a separate `DiffViewState` per instance |

```rust,ignore
let theme = UiTheme::default();
let block = card(&theme, "Changes", focused);
let inner = block.inner(area);
frame.render_widget(block, area);
frame.render_widget(EmptyState::new("No changes", &theme), inner);
```

Use `Block::inner` for content geometry. List highlights come from
`UiTheme::highlight_for`; file trees, lists, inputs and diff rows share the same
surface and selection tokens. Read shortcut keys from the configured keymap.
The workspace adds card gutters at 100 columns and 28 rows; smaller windows
retain the compact layout. There is no global header or status bar.

`molecules/tests.rs` renders a component gallery to
`src/ui/goldens/component_library.golden`. Review changes with `cargo test ui::`.
To intentionally refresh the visual baselines, set `RATAGIT_UPDATE_GOLDEN=1`
for that invocation, then run the tests again without it.

The [hunk/diff component](diff/README.md) exposes read-only data entry and
semantic behavior bindings. Rendering, row styles, chrome, ANSI parsing and
viewport history are separated from application navigation and Git operations.
