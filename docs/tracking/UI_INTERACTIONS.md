# UI Interactions Guide (Lazygit Keymap Parity)

This document tracks ratagit keybindings against the local lazygit repo at `D:\prj\lazygit`
(checked at lazygit commit `d167063b4`).
The operation groups and default keys are aligned with lazygit `docs/keybindings/Keybindings_en.md`
and the default keybinding config in `pkg/config/user_config.go`.

## Legend

- Implemented?: `✅` yes, `❌` no
- Have test?: `✅` yes, `❌` no
- Lazygit key notation: `<c-b>` means Ctrl+B, `<s-down>` means Shift+Down, and `B` means Shift+B.

---

## Lazygit UI operation levels

- Global: actions available across the app unless an overlay/input context captures the key.
- Shared list navigation: list cursor movement, paging, range select, search/filter, tabs, and horizontal scroll.
- Resource panels: status, files, branches, commits, stash, remotes, tags, worktrees, submodules, reflog, sub-commits, and commit files.
- Main panel modes: normal, staging, merging, and custom patch building.
- Overlay/input contexts: menu, confirmation, input prompt, commit summary, and secondary view.

---

## Global keybindings

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<c-r>` | Switch to a recent repo | ❌ | ❌ |
| `<pgup>` (`fn+up` / `shift+k`) | Scroll up main window | ❌ | ❌ |
| `<pgdown>` (`fn+down` / `shift+j`) | Scroll down main window | ❌ | ❌ |
| `@` | View command log options | ❌ | ❌ |
| `P` | Push | ❌ | ❌ |
| `p` | Pull | ❌ | ❌ |
| `)` | Increase rename similarity threshold | ❌ | ❌ |
| `(` | Decrease rename similarity threshold | ❌ | ❌ |
| `}` | Increase diff context size | ❌ | ❌ |
| `{` | Decrease diff context size | ❌ | ❌ |
| `:` | Execute shell command | ❌ | ❌ |
| `<c-p>` | View custom patch options | ❌ | ❌ |
| `m` | View merge/rebase options | ❌ | ❌ |
| `R` | Refresh | ❌ | ❌ |
| `+` | Next screen mode (normal/half/fullscreen) | ❌ | ❌ |
| `_` | Previous screen mode | ❌ | ❌ |
| `\|` | Cycle pagers | ❌ | ❌ |
| `<esc>` | Cancel | ✅ | ✅ |
| `?` | Open keybindings menu | ✅ | ❌ |
| `<c-s>` | View filter options | ❌ | ❌ |
| `W` | View diffing options | ❌ | ❌ |
| `<c-e>` | View diffing options | ❌ | ❌ |
| `q` | Quit | ✅ | ❌ |
| `<c-z>` | Suspend the application | ❌ | ❌ |
| `<c-w>` | Toggle whitespace | ❌ | ❌ |
| `z` | Undo | ❌ | ❌ |
| `Z` | Redo | ❌ | ❌ |

---

## Shared list navigation

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `j` / `<down>` | Next item | ✅ | ✅ |
| `k` / `<up>` | Previous item | ✅ | ✅ |
| `,` | Previous page | ❌ | ❌ |
| `.` | Next page | ❌ | ❌ |
| `<` / `<home>` | Scroll to top | ❌ | ❌ |
| `>` / `<end>` | Scroll to bottom | ❌ | ❌ |
| `v` | Toggle range select | ✅ | ❌ |
| `<s-down>` | Range select down | ❌ | ❌ |
| `<s-up>` | Range select up | ❌ | ❌ |
| `/` | Search or filter the current view by text | ❌ | ❌ |
| `H` | Scroll left | ❌ | ❌ |
| `L` | Scroll right | ❌ | ❌ |
| `]` | Next tab | ❌ | ❌ |
| `[` | Previous tab | ❌ | ❌ |
| `1`-`5` | Jump to block/panel | ❌ | ❌ |
| `0` | Focus main view | ❌ | ❌ |

---

## Status panel

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `o` | Open config file | ❌ | ❌ |
| `e` | Edit config file | ❌ | ❌ |
| `u` | Check for update | ❌ | ❌ |
| `<enter>` | Switch to a recent repo | ❌ | ❌ |
| `a` | Show/cycle all branch logs | ❌ | ❌ |
| `A` | Show/cycle all branch logs (reverse) | ❌ | ❌ |
| `0` | Focus main view | ❌ | ❌ |

---

## Files panel

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<c-o>` | Copy path to clipboard | ❌ | ❌ |
| `<space>` | Stage | ✅ | ✅ |
| `<c-b>` | Filter files by status | ❌ | ❌ |
| `y` | Copy to clipboard | ❌ | ❌ |
| `c` | Commit | ✅ | ✅ |
| `w` | Commit changes without pre-commit hook | ❌ | ❌ |
| `A` | Amend last commit | ✅ | ✅ |
| `C` | Commit changes using git editor | ❌ | ❌ |
| `<c-f>` | Find base commit for fixup | ❌ | ❌ |
| `e` | Edit file | ❌ | ❌ |
| `o` | Open file | ❌ | ❌ |
| `i` | Ignore or exclude file | ✅ | ✅ |
| `r` | Refresh files | ❌ | ❌ |
| `s` | Stash | ✅ | ✅ |
| `S` | View stash options | ❌ | ❌ |
| `a` | Stage all | ✅ | ✅ |
| `<enter>` | Stage lines / collapse directory | ✅ | ✅ |
| `d` | Discard | ✅ | ✅ |
| `g` | View upstream reset options | ❌ | ❌ |
| `D` | Reset | ✅ | ✅ |
| `` ` `` | Toggle file tree view | ✅ | ✅ |
| `<c-t>` | Open external diff tool (git difftool) | ❌ | ❌ |
| `M` | View merge conflict options | ❌ | ❌ |
| `f` | Fetch | ❌ | ❌ |
| `-` | Collapse all files | ✅ | ✅ |
| `=` | Expand all files | ✅ | ✅ |
| `0` | Focus main view | ❌ | ❌ |
| `/` | Filter the current view by text | ❌ | ❌ |

---

## Local branches panel

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<c-o>` | Copy branch name to clipboard | ❌ | ❌ |
| `i` | Show git-flow options | ❌ | ❌ |
| `<space>` | Checkout | ✅ | ✅ |
| `n` | New branch | ✅ | ✅ |
| `N` | Move commits to new branch | ❌ | ❌ |
| `o` | Create pull request | ❌ | ❌ |
| `O` | View create pull request options | ❌ | ❌ |
| `G` | Open pull request in browser | ❌ | ❌ |
| `<c-y>` | Copy pull request URL to clipboard | ❌ | ❌ |
| `c` | Checkout by name | ❌ | ❌ |
| `-` | Checkout previous branch | ❌ | ❌ |
| `F` | Force checkout | ❌ | ❌ |
| `d` | View delete options (local / remote / local+remote) | ✅ | ✅ |
| `r` | Rebase | ❌ | ❌ |
| `M` | Merge | ❌ | ❌ |
| `f` | Fast-forward | ❌ | ❌ |
| `T` | New tag | ❌ | ❌ |
| `s` | Sort order | ❌ | ❌ |
| `g` | Reset | ❌ | ❌ |
| `R` | Rename branch | ❌ | ❌ |
| `u` | View upstream options | ❌ | ❌ |
| `<c-t>` | Open external diff tool (git difftool) | ❌ | ❌ |
| `0` | Focus main view | ❌ | ❌ |
| `<enter>` | View commits | ✅ | ✅ |
| `w` | View worktree options | ❌ | ❌ |
| `/` | Filter the current view by text | ❌ | ❌ |

---

## Commits panel

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<c-o>` | Copy abbreviated commit hash to clipboard | ✅ | ✅ |
| `<c-r>` | Reset copied (cherry-picked) commits selection | ✅ | ✅ |
| `b` | View bisect options | ❌ | ❌ |
| `s` | Squash | ❌ | ❌ |
| `f` | Fixup | ❌ | ❌ |
| `c` | Set fixup message | ❌ | ❌ |
| `r` | Reword | ❌ | ❌ |
| `R` | Reword with editor | ❌ | ❌ |
| `d` | Drop | ❌ | ❌ |
| `e` | Edit (start interactive rebase) | ❌ | ❌ |
| `i` | Start interactive rebase | ❌ | ❌ |
| `p` | Pick | ❌ | ❌ |
| `F` | Create fixup commit | ❌ | ❌ |
| `S` | Apply fixup commits | ❌ | ❌ |
| `<c-j>` | Move commit down one | ❌ | ❌ |
| `<c-k>` | Move commit up one | ❌ | ❌ |
| `V` | Paste (cherry-pick) | ✅ | ✅ |
| `B` | Mark as base commit for rebase | ❌ | ❌ |
| `A` | Amend | ❌ | ❌ |
| `a` | Amend commit attribute | ❌ | ❌ |
| `t` | Revert | ✅ | ✅ |
| `T` | Tag commit | ❌ | ❌ |
| `<c-l>` | View log options | ❌ | ❌ |
| `G` | Open pull request in browser | ❌ | ❌ |
| `<space>` | Checkout | ✅ | ✅ |
| `y` | Copy commit attribute to clipboard | ❌ | ❌ |
| `o` | Open commit in browser | ❌ | ❌ |
| `n` | Create new branch off of commit | ✅ | ✅ |
| `N` | Move commits to new branch | ❌ | ❌ |
| `g` | Reset | ✅ | ✅ |
| `C` | Copy (cherry-pick) | ✅ | ✅ |
| `<c-t>` | Open external diff tool (git difftool) | ❌ | ❌ |
| `*` | Select commits of current branch | ❌ | ❌ |
| `0` | Focus main view | ❌ | ❌ |
| `<enter>` | View files | ✅ | ✅ |
| `w` | View worktree options | ❌ | ❌ |
| `/` | Search the current view by text | ❌ | ❌ |

Implementation note (core parity):
- `<enter>` in `Commits` list now triggers `GetCommitFiles`, switches the commit panel to a loading subview, and transitions to commit files tree after `CommitFilesLoaded`.
- The same `<enter>` flow is wired for branch `commits subview` (inside `Branches` panel).
- Stale `CommitFilesLoaded` responses are ignored via pending commit-id guard.

---

## Commit files panel

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<c-o>` | Copy path to clipboard | ❌ | ❌ |
| `y` | Copy to clipboard | ❌ | ❌ |
| `c` | Checkout | ❌ | ❌ |
| `d` | Discard | ❌ | ❌ |
| `o` | Open file | ❌ | ❌ |
| `e` | Edit | ❌ | ❌ |
| `<c-t>` | Open external diff tool (git difftool) | ❌ | ❌ |
| `<space>` | Toggle file included in patch | ❌ | ❌ |
| `a` | Toggle all files | ❌ | ❌ |
| `<enter>` | Enter file / toggle directory collapsed | ✅ | ✅ |
| `` ` `` | Toggle file tree view | ❌ | ❌ |
| `-` | Collapse all files | ✅ | ✅ |
| `=` | Expand all files | ✅ | ✅ |
| `0` | Focus main view | ❌ | ❌ |
| `/` | Filter the current view by text | ❌ | ❌ |

---

## Stash panel

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<space>` | Apply | ✅ | ✅ |
| `g` | Pop | ❌ | ❌ |
| `d` | Drop | ✅ | ✅ |
| `n` | New branch | ❌ | ❌ |
| `r` | Rename stash | ❌ | ❌ |
| `0` | Focus main view | ❌ | ❌ |
| `<enter>` | View files | ✅ | ✅ |
| `w` | View worktree options | ❌ | ❌ |
| `/` | Filter the current view by text | ❌ | ❌ |

---

## Main panel (normal)

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| Mouse wheel down (`fn+up`) | Scroll down | ❌ | ❌ |
| Mouse wheel up (`fn+down`) | Scroll up | ❌ | ❌ |
| `<tab>` | Switch view | ❌ | ❌ |
| `<esc>` | Exit back to side panel | ✅ | ✅ |
| `/` | Search the current view by text | ❌ | ❌ |

---

## Main panel (staging)

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<left>` | Go to previous hunk | ❌ | ❌ |
| `<right>` | Go to next hunk | ❌ | ❌ |
| `v` | Toggle range select | ❌ | ❌ |
| `a` | Toggle hunk selection | ❌ | ❌ |
| `<c-o>` | Copy selected text to clipboard | ❌ | ❌ |
| `<space>` | Stage | ❌ | ❌ |
| `d` | Discard | ❌ | ❌ |
| `o` | Open file | ❌ | ❌ |
| `e` | Edit file | ❌ | ❌ |
| `<esc>` | Return to files panel | ❌ | ❌ |
| `<tab>` | Switch view | ❌ | ❌ |
| `E` | Edit hunk | ❌ | ❌ |
| `c` | Commit | ❌ | ❌ |
| `w` | Commit changes without pre-commit hook | ❌ | ❌ |
| `C` | Commit changes using git editor | ❌ | ❌ |
| `<c-f>` | Find base commit for fixup | ❌ | ❌ |
| `/` | Search the current view by text | ❌ | ❌ |

---

## Main panel (merging)

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<space>` | Pick hunk | ❌ | ❌ |
| `b` | Pick all hunks | ❌ | ❌ |
| `<up>` | Previous hunk | ❌ | ❌ |
| `<down>` | Next hunk | ❌ | ❌ |
| `<left>` | Previous conflict | ❌ | ❌ |
| `<right>` | Next conflict | ❌ | ❌ |
| `z` | Undo | ❌ | ❌ |
| `e` | Edit file | ❌ | ❌ |
| `o` | Open file | ❌ | ❌ |
| `M` | View merge conflict options | ❌ | ❌ |
| `<esc>` | Return to files panel | ❌ | ❌ |

---

## Main panel (patch building)

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<left>` | Go to previous hunk | ❌ | ❌ |
| `<right>` | Go to next hunk | ❌ | ❌ |
| `v` | Toggle range select | ❌ | ❌ |
| `a` | Toggle hunk selection | ❌ | ❌ |
| `<c-o>` | Copy selected text to clipboard | ❌ | ❌ |
| `o` | Open file | ❌ | ❌ |
| `e` | Edit file | ❌ | ❌ |
| `<space>` | Toggle lines in patch | ❌ | ❌ |
| `d` | Remove lines from commit | ❌ | ❌ |
| `<esc>` | Exit custom patch builder | ❌ | ❌ |
| `/` | Search the current view by text | ❌ | ❌ |

---

## Reflog panel

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<c-o>` | Copy abbreviated commit hash to clipboard | ❌ | ❌ |
| `<space>` | Checkout | ❌ | ❌ |
| `y` | Copy commit attribute to clipboard | ❌ | ❌ |
| `o` | Open commit in browser | ❌ | ❌ |
| `n` | Create new branch off of commit | ❌ | ❌ |
| `N` | Move commits to new branch | ❌ | ❌ |
| `g` | Reset | ❌ | ❌ |
| `C` | Copy (cherry-pick) | ❌ | ❌ |
| `<c-r>` | Reset copied (cherry-picked) commits selection | ❌ | ❌ |
| `<c-t>` | Open external diff tool (git difftool) | ❌ | ❌ |
| `*` | Select commits of current branch | ❌ | ❌ |
| `0` | Focus main view | ❌ | ❌ |
| `<enter>` | View commits | ❌ | ❌ |
| `w` | View worktree options | ❌ | ❌ |
| `/` | Filter the current view by text | ❌ | ❌ |

---

## Remote branches panel

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<c-o>` | Copy branch name to clipboard | ❌ | ❌ |
| `<space>` | Checkout | ❌ | ❌ |
| `n` | New branch | ❌ | ❌ |
| `M` | Merge | ❌ | ❌ |
| `r` | Rebase | ❌ | ❌ |
| `d` | Delete | ❌ | ❌ |
| `u` | Set as upstream | ❌ | ❌ |
| `s` | Sort order | ❌ | ❌ |
| `g` | Reset | ❌ | ❌ |
| `<c-t>` | Open external diff tool (git difftool) | ❌ | ❌ |
| `0` | Focus main view | ❌ | ❌ |
| `<enter>` | View commits | ❌ | ❌ |
| `w` | View worktree options | ❌ | ❌ |
| `/` | Filter the current view by text | ❌ | ❌ |

---

## Remotes panel

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<enter>` | View branches | ❌ | ❌ |
| `n` | New remote | ❌ | ❌ |
| `d` | Remove | ❌ | ❌ |
| `e` | Edit | ❌ | ❌ |
| `f` | Fetch | ❌ | ❌ |
| `F` | Add fork remote | ❌ | ❌ |
| `/` | Filter the current view by text | ❌ | ❌ |

---

## Tags panel

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<c-o>` | Copy tag to clipboard | ❌ | ❌ |
| `<space>` | Checkout | ❌ | ❌ |
| `n` | New tag | ❌ | ❌ |
| `d` | Delete | ❌ | ❌ |
| `P` | Push tag | ❌ | ❌ |
| `g` | Reset | ❌ | ❌ |
| `<c-t>` | Open external diff tool (git difftool) | ❌ | ❌ |
| `0` | Focus main view | ❌ | ❌ |
| `<enter>` | View commits | ❌ | ❌ |
| `w` | View worktree options | ❌ | ❌ |
| `/` | Filter the current view by text | ❌ | ❌ |

---

## Worktrees panel

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `n` | New worktree | ❌ | ❌ |
| `<space>` | Switch | ❌ | ❌ |
| `o` | Open in editor | ❌ | ❌ |
| `d` | Remove | ❌ | ❌ |
| `/` | Filter the current view by text | ❌ | ❌ |

---

## Sub-commits panel

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<c-o>` | Copy abbreviated commit hash to clipboard | ❌ | ❌ |
| `<space>` | Checkout | ❌ | ❌ |
| `y` | Copy commit attribute to clipboard | ❌ | ❌ |
| `o` | Open commit in browser | ❌ | ❌ |
| `n` | Create new branch off of commit | ❌ | ❌ |
| `N` | Move commits to new branch | ❌ | ❌ |
| `g` | Reset | ❌ | ❌ |
| `C` | Copy (cherry-pick) | ❌ | ❌ |
| `<c-r>` | Reset copied (cherry-picked) commits selection | ❌ | ❌ |
| `<c-t>` | Open external diff tool (git difftool) | ❌ | ❌ |
| `*` | Select commits of current branch | ❌ | ❌ |
| `0` | Focus main view | ❌ | ❌ |
| `<enter>` | View files | ❌ | ❌ |
| `w` | View worktree options | ❌ | ❌ |
| `/` | Search the current view by text | ❌ | ❌ |

---

## Submodules panel

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<c-o>` | Copy submodule name to clipboard | ❌ | ❌ |
| `<enter>` | Enter | ❌ | ❌ |
| `d` | Remove | ❌ | ❌ |
| `u` | Update | ❌ | ❌ |
| `n` | New submodule | ❌ | ❌ |
| `e` | Update submodule URL | ❌ | ❌ |
| `i` | Initialize | ❌ | ❌ |
| `b` | View bulk submodule options | ❌ | ❌ |
| `/` | Filter the current view by text | ❌ | ❌ |

---

## Menu panel

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<enter>` | Execute | ✅ | ✅ |
| `<esc>` | Close/cancel | ✅ | ✅ |
| `/` | Filter the current view by text | ❌ | ❌ |

---

## Confirmation panel

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<enter>` | Confirm | ❌ | ❌ |
| `<esc>` | Close/cancel | ❌ | ❌ |
| `<c-o>` | Copy to clipboard | ❌ | ❌ |

---

## Input prompt

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<enter>` | Confirm | ❌ | ❌ |
| `<esc>` | Close/cancel | ❌ | ❌ |

---

## Commit summary / Commit description panels

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<enter>` | Confirm | ✅ | ✅ |
| `<esc>` | Close/cancel | ✅ | ✅ |

---

## Secondary view

| Key | Action | Implemented? | Have test? |
|-----|--------|--------------|------------|
| `<tab>` | Switch view | ❌ | ❌ |
| `<esc>` | Exit back to side panel | ❌ | ❌ |
| `/` | Search the current view by text | ❌ | ❌ |

---

## Coverage Summary

### Implemented Features

- Basic list navigation (`j`/`k`, arrows) in panels
- Global quit/cancel/help (`q`, `<esc>`, `?`)
- Files: stage/unstage, commit, amend, discard, ignore, stash, enter staging/file tree
- Branches: checkout, new, delete options (local/remote/both), view commits
- Commits: view files, copy/paste cherry-pick flow (`C`/`V`/`Ctrl+r`), checkout commit (`Space`), reset/new-branch/revert/copy-hash
- Stash: apply, drop, view files
- Menus and commit message panels: confirm/cancel and execute
- Multi-select toggle (`v`)

### High-Priority Missing (Lazygit Core)

- Global refresh (`R`)
- Push/pull (`P`/`p`)
- Undo/redo (`z`/`Z`)
- Search/filter (`/`) across list/main/overlay contexts
- Main view staging mode
- Merge/rebase options (`m`)
- Screen modes (`+`/`_`)
- Panel focus/jump parity (`0`, `1`-`5`)
- Status panel operation keys

### Medium-Priority Missing

- Remotes and remote branches
- Worktrees
- Reflog and sub-commits
- Tags
- Submodules
- Secondary view
- Confirmation/input prompt lazygit parity
- External diff tool

---

## Update Protocol

When implementing features:

1. Mark `Implemented?` as `✅` when feature works.
2. Mark `Have test?` as `✅` when test coverage is added.
3. Update `docs/tracking/LAZYGIT_FEATURE_PARITY.md` for feature-level tracking.
4. Commit both docs together.

This document is the source of truth for lazygit keymap parity.
