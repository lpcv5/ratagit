# Themes

ratagit loads a theme once at startup, before enabling raw mode or entering the
alternate screen. The executable reads the configuration and passes the resolved
palette to the renderer; rendering does not read files or environment variables.

## Configuration location

`RATAGIT_CONFIG` selects an explicit configuration file and takes precedence on
all platforms. Relative paths are resolved from the process working directory.
The selected file must exist and be readable; even an empty `RATAGIT_CONFIG`
value is treated as an explicit path and fails rather than selecting a default.

Without `RATAGIT_CONFIG`, ratagit looks for:

| Platform | Default file |
| --- | --- |
| Windows | `%APPDATA%\ratagit\config.json` |
| Unix, with nonempty `XDG_CONFIG_HOME` | `$XDG_CONFIG_HOME/ratagit/config.json` |
| Unix, otherwise with nonempty `HOME` | `$HOME/.config/ratagit/config.json` |

If no applicable base directory is set, or the implicit file does not exist,
ratagit uses the built-in `default` theme. Other read errors are reported rather
than ignored. Windows uses `APPDATA` only, with no `HOME` fallback.

## JSON format and presets

The configuration is a UTF-8 JSON object; a leading UTF-8 BOM is accepted. Its
shape is:

```json
{
  "theme": {
    "preset": "default",
    "colors": {}
  }
}
```

`theme`, `preset`, and `colors` are optional. An omitted `theme` or `preset` uses
`default`, and an omitted `colors` uses no overrides. `{}` is therefore valid.
`theme` must be an object, `preset` a string, and `colors` an object mapping exact
role names to color strings. Unknown fields at the root or inside `theme` are
errors, as are unknown color roles. JSON comments, trailing commas, and `null`
values are not supported.

Choose `default` or `tokyo-night` as the base preset, then override individual
roles in `colors`. Preset and role names are case-sensitive. Omitted roles retain
their preset values.

| Preset | Appearance |
| --- | --- |
| `default` | Terminal-default foreground/background, yellow focus and cursor selection, blue batch selection, ANSI semantic colors, and fixed RGB diff and modal-selection surfaces. |
| `tokyo-night` | A dark RGB base with blue and amber accents, explicit modal colors and scrim, and RGB cursor, batch, and badge pairs. Commit, file-status, and diff roles start with the same values as `default`. |

The default modal selection uses dark `#142238` text on pale blue `#B4D7FF`.
Tokyo Night uses `#C0CAF5` text on `#24283B`. Both pairs meet the modal selection
contrast check.

## Color strings

Use `#RRGGBB` for fixed RGB colors, with case-insensitive hex digits. Short hex
forms and alpha channels are not supported. Surrounding whitespace is ignored.
For roles outside the fixed RGB restrictions below, these strings are also
accepted:

- Named ANSI colors: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`,
  `gray`/`grey`, `darkgray`/`darkgrey`, `lightred`, `lightgreen`, `lightyellow`,
  `lightblue`, `lightmagenta`, `lightcyan`, and `white`.
- `default` or `reset` to use the terminal's default color for that channel.
- `ansi:0` through `ansi:255` for indexed ANSI colors.

Names are case-insensitive and also accept hyphenated or underscored spellings,
such as `LightBlue`, `light-blue`, and `light_blue`. Named and indexed colors are
resolved by the terminal palette, so their actual RGB values are not known to
ratagit.

## Color roles

Every entry below is a literal key in the flat `theme.colors` object. Dots are
part of the key; do not turn them into nested JSON objects.

| Group | Accepted keys | Purpose |
| --- | --- | --- |
| Base | `foreground`, `background` | Main text and background; also fallbacks for modal text/background and scrim background. |
| Chrome and feedback | `accent`, `muted`, `danger`, `warning`, `success`, `info` | Focused chrome, subdued text, semantic feedback, search and loading accents. |
| Cursor selection | `selection.foreground`, `selection.background` | Focused list cursor, focused title badges, and shortcut badges. |
| Batch selection | `batch.foreground`, `batch.background` | Rows selected for batch operations. |
| Inactive badge | `badge.foreground`, `badge.background` | Unfocused panel title badges. |
| Modal body | `modal.foreground`, `modal.background`, `modal.muted`, `modal.border`, `modal.scrim` | Body text/surface, subdued text, border, and overlay scrim background. |
| Modal tones | `modal.active`, `modal.danger`, `modal.warning` | Active/info, danger, and warning accents. |
| Modal selection | `modal.selection.foreground`, `modal.selection.background` | Selected choices in all modal lists, including the command palette. |
| Commit graph and hash | `commit.graph`, `commit.hash.merged`, `commit.hash.pushed`, `commit.hash.unpushed` | Graph placeholder and main/upstream reachability colors. |
| Commit authors | `commit.author.0`, `commit.author.1`, `commit.author.2`, `commit.author.3`, `commit.author.4`, `commit.author.5`, `commit.author.6`, `commit.author.7` | Eight colors assigned deterministically to author initials. |
| File status | `files.added`, `files.modified`, `files.deleted`, `files.renamed`, `files.copied`, `files.type_changed`, `files.unknown`, `files.conflict`, `files.staged` | Status markers and staged workspace filenames. `type_changed` uses an underscore. |
| Diff text and gutter | `diff.added.foreground`, `diff.removed.foreground`, `diff.context.foreground`, `diff.gutter` | Add/remove text, highlighted context and highlighted gutters, and gutters without a highlighted background. |
| Added, unstaged backgrounds | `diff.added.unstaged.normal`, `diff.added.unstaged.marked`, `diff.added.unstaged.cursor` | Added worktree rows by interaction state. |
| Added, staged backgrounds | `diff.added.staged.normal`, `diff.added.staged.marked`, `diff.added.staged.cursor` | Added index rows by interaction state. |
| Removed, unstaged backgrounds | `diff.removed.unstaged.normal`, `diff.removed.unstaged.marked`, `diff.removed.unstaged.cursor` | Removed worktree rows by interaction state. |
| Removed, staged backgrounds | `diff.removed.staged.normal`, `diff.removed.staged.marked`, `diff.removed.staged.cursor` | Removed index rows by interaction state. |
| Context backgrounds | `diff.context.cursor`, `diff.context.marked` | Highlighted context rows. |

Cursor styling takes precedence over marked styling in diff rows. In list rows,
cursor and batch selection replace every span's foreground/background, including
hashes, authors, file statuses, and search matches. Search matches retain their
underline. Modal list selection likewise replaces semantic label colors,
indicators, and trailing padding with one full-width selection pair.

Theme roles apply to ratagit's chrome, list rows, modals, and shared Files diff
renderer. Native Git ANSI colors in history patches and log/graph output are
preserved, not recolored by the theme.

## Contrast constraints

Overrides are applied to the preset first, then the final palette is validated
using the relative luminance of fixed RGB colors:

| Surface | Requirements |
| --- | --- |
| Cursor, batch, and badge pairs | Overrides to either channel must be `#RRGGBB`. The resulting foreground and background must both be RGB and have at least **4.5:1** contrast. With the `default` preset, override both channels together because the inherited partner is ANSI. With an RGB preset partner, one channel may be sufficient. |
| Modal selection | Both resulting channels must be RGB with at least **7:1** contrast. Each overridden channel must be `#RRGGBB`; the preset already supplies an RGB partner. |
| Diff | Every diff override, including `diff.gutter`, must be `#RRGGBB`. Added and removed text must each have at least **4.5:1** contrast against all six corresponding staged/unstaged and normal/marked/cursor backgrounds. `diff.context.foreground` must also meet **4.5:1** against all twelve add/remove backgrounds and both context highlight backgrounds, since it colors highlighted gutters as well as context text. |
| Base and modal body | The base foreground/background and the effective modal foreground/background are checked for at least **4.5:1** when both channels are RGB. Identical non-reset color values are rejected even for named/indexed colors. Omitted modal body channels inherit their base counterparts. |
| Semantic text | `accent`, `danger`, `warning`, `success`, `info`, all `files.*`, all `commit.hash.*`, all `commit.author.*`, and `diff.gutter` are checked against the base background. `modal.active`, `modal.warning`, and `modal.danger` are checked against the effective modal background. These pairs require **4.5:1** when both channels are RGB, and reject identical non-reset color values in any format. |
| Muted chrome | `muted` and `commit.graph` are checked against the base background; `modal.muted` and `modal.border` against the effective modal background; the modal muted text against the effective scrim background. These checks reject identical non-reset values, without enforcing a contrast ratio. |

The unchanged legacy ANSI cursor, batch, and badge pairs in `default` are
preserved as exceptions. They cannot be contrast-verified because the terminal
controls their values. Explicit overrides to those roles still require RGB.
Named and indexed overrides are accepted for other roles, except the diff roles
listed above. `diff.gutter` requires RGB on override; its contrast is checked
against an RGB base background. Highlighted gutters use
`diff.context.foreground` instead. An identical named or indexed value can be
rejected without knowing the terminal palette, but distinct values do not imply
adequate contrast. Two terminal-default (`reset`) channels are allowed.

Validation does not guarantee contrast for terminal-default colors, named/indexed
ANSI colors, or preserved native Git ANSI spans. Muted chrome has no minimum
ratio, and semantic text checks cover the surfaces listed above rather than
every possible combination. Check the result in your terminal, particularly
when using an ANSI palette or terminal-default base colors.

## Minimal custom theme

Save the following as `theme.json` in your working directory. It changes the
focus accent while inheriting the remaining Tokyo Night palette:

```json
{
  "theme": {
    "preset": "tokyo-night",
    "colors": {
      "accent": "#ABCDEF"
    }
  }
}
```

From the repository root, launch it in PowerShell:

```powershell
$env:RATAGIT_CONFIG = Join-Path (Get-Location) 'theme.json'
cargo run --locked --bin ratagit
```

Or from a Unix shell:

```sh
RATAGIT_CONFIG="$PWD/theme.json" cargo run --locked --bin ratagit
```

For an installed binary, replace `cargo run --locked --bin ratagit` with
`ratagit`. To use automatic discovery instead, save the JSON at your platform's
default configuration location and unset `RATAGIT_CONFIG`.

## Configuration errors

A bad configuration stops startup before terminal setup. File-related errors
include the selected path. Parse errors identify invalid JSON or field types;
theme errors identify the unknown preset/role, invalid color value, or failing
contrast pair. For example:

- `"files.type-changed"` is unknown; use `"files.type_changed"`.
- `"selection.background": "blue"` is rejected because paired overrides require
  fixed RGB, even though `blue` is accepted for an unpaired role such as `accent`.
- Overriding only `selection.background` on `default` fails because its inherited
  foreground is ANSI; supply an RGB foreground too.
- Equal RGB foreground/background values fail the relevant contrast threshold;
  identical non-reset named/indexed values on checked surfaces also fail. Low
  contrast RGB semantic text, such as a file-status color on the RGB base
  background, is rejected too.
- An unreadable file or a missing explicit `RATAGIT_CONFIG` file is an error;
  only a missing implicit file falls back to the default theme.

Correct the reported file and start ratagit again.
