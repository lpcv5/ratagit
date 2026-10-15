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
| Molecules | `render_input(frame, area, InputField, &theme)` | Labeled input with placeholder, focus border and cell-aware scrolling/cursor for single or multiple lines |
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

## Overlay design

The terminal equivalents draw on shadcn's [Dialog](https://ui.shadcn.com/docs/components/dialog),
[Alert Dialog](https://ui.shadcn.com/docs/components/alert-dialog),
[Field](https://ui.shadcn.com/docs/components/field) and
[Command](https://ui.shadcn.com/docs/components/command) patterns: clear titles,
muted descriptions, labeled fields, separate actions and searchable results.
They use existing terminal theme tokens without adding a web UI dependency.

| Pattern | Terminal usage |
| --- | --- |
| Alert Dialog | Stage-all and branch-switch confirmations; explain effects before the primary action |
| Dialog + Input / Textarea | Branch creation, stash title and commit message/description; required-field errors retain the draft |
| Command / searchable selection | Command palette with Up/Down, Tab/Shift+Tab, PageUp/PageDown, Home/End and Enter; exact aliases rank first |
| Empty | Command search with no results; Enter keeps the search open |
| Kbd + Button | Wrapped action rows with keycaps and an accented primary action |
| Scroll Area | Selected commands and help entries remain visible; long input follows the editing cursor |

`panels/overlay_utils.rs` owns content-sized geometry, the dimmed backdrop,
padding and footer layout. Confirmations cap at 72 columns, branch input at 64,
editors/palette at 84 and help at 104. Small terminals reduce gutters; footers
reserve space before body content. The backdrop is redrawn normally on close.
Selection belongs to application input state and is published in both borrowed
and owned snapshots; renderers never execute commands.

Additional standalone Select/Combobox, Popover/Context Menu, Drawer/Sheet,
Checkbox/Radio and Toast components are deferred until a Git workflow needs
them. The palette already supplies searchable single selection, visual file
selection handles batching, and the existing command log and loading indicator
provide operation feedback. Calendar, OTP and similar form controls do not
currently have a repository workflow.
