# Everyday Git workflow audit

Date: 2026-10-16. Platform: Windows 11, Git 2.55, Rust 1.99.

## Method

Primary evidence comes from launching the actual `ratagit.exe` in a native
Windows ConPTY terminal, sending real keys, reading the resulting terminal
screen, and checking Git index, worktree, commits, refs and stash objects.
Git commands prepare disposable fixtures and independently inspect outcomes;
the operations under review run through the application. Fixtures live under
`tmp/git-tests/`. No checkout history or remote network state was changed.

Captures, actual key sequences, raw terminal output and per-step Git state are
under `tmp/git-workflow-audit/`. `live-evidence.html` presents the initial
before/after captures. Character captures preserve screen text; they do not prove
font appearance or exact native terminal colors. Early WinPTY captures and
shared controller sessions affected by input collisions are excluded; native
ConPTY runs with isolated controller/fixture names supply the evidence below.

Cargo regressions supplement live interaction after fixes. They are not used as
a substitute for opening and operating the software.

## Confirmed defects and repair status

| Defect | Actual trigger and consequence | Repair / verification |
| --- | --- | --- |
| Selected rename only partly unstaged | `git mv a.txt renamed.txt`, open Files, Space on `R renamed.txt`: old `a.txt` deletion remains staged. A later commit can accidentally delete the old file. | Restore both index paths while retaining worktree rename and later edits. Actual fixed-app unstage/restage passed, including unrelated staged `b.txt`. |
| Discard expands literal filenames | Select untracked `[abc].txt`, `d`, Enter. Confirmation lists only that file, but its Git pathspec matches dirty `a.txt`; `a.txt` changes are discarded and selected file remains. | Use literal pathspecs. Actual fixed-app tracked/untracked bracket-name discard and cancellation passed; unrelated staged/unstaged and Unicode paths preserved. |
| Case-only rename can retain staged deletion | Windows `core.ignorecase=true`, rename `a.txt` to `A.txt`, Space: per-path remove/restore ordering can remove restored source. | Separate removal and restoration passes. Actual final-app replay passed with empty cached diff and intact worktree. |
| Symlink rename only partly unstaged | Real `120000` link object with `core.symlinks=false`, rename old-link to new-link, Space: old-link deletion remains staged. | Exact link-target rename matching supplements libgit2 similarity detection. Actual final-app replay passed with empty cached diff and intact target content. |
| Selected stash interprets paths as patterns | Select only `[abc].txt`, `v`, `s`, Enter: unselected dirty `a.txt` is also stashed and reverted. | Literal pathspecs. Final actual-app save/preview/pop passed with only `[abc].txt` in payload and unchanged unselected `a.txt`. |
| Selected stash captures unrelated staged content | Staged `a.txt`, select only dirty `b.txt` and untracked `new.txt`, `s`, Enter. Stash payload contains all three despite selected-files scope of two. | Save using a scoped alternate index, then install the prepared real index under its lock. Final actual-app payload is exactly `b.txt/new.txt`; unrelated staged/worktree contents preserved. |
| Stash preview omits untracked contents | Save tracked edits plus untracked `new.txt`, focus Stash. Preview shows tracked diffs only, although saved third parent contains `new.txt`. | Append bounded third-parent additions using the existing read-only patch renderer. Final actual-app mixed and untracked-only previews visibly contain saved text. |
| Conflicting stash pop leaves Files stale | Pop stash onto a conflicting committed version. Git contains `UU a.txt` plus restored files, but Files still shows an empty clean list until manual refresh. | Refresh after failed pop. Final actual-app replay automatically displays conflicted `a.txt`, preserves the error and retains the stash. |

Source changes are confined to the Git boundary and mutation-result handling;
regressions exercise index/worktree preservation, literal paths, stash payloads,
read-only bounded previews, rendered conflict state and emitted commands.
Selected stash preparation holds the real index lock, scopes a temporary index,
prepares the selected-reset replacement before mutating the worktree, and installs
it only after native stash succeeds. Failures retain the real index and any
created stash. Temporary commands disable shared-index expiration locally;
persistent repository configuration and existing stash objects are unchanged.

## Actual application scenario coverage

| Area | Cases operated through actual keys | Result so far |
| --- | --- | --- |
| Files and commit | Individual/all stage and unstage; commit editor and submit; unborn HEAD stage/unstage and stage-all confirmation for first commit | Passed |
| Partial changes | Two nonadjacent hunks, stage one hunk then immediately reverse, retain other hunk and worktree | Passed |
| Paths and deletion | Rename unstage/restage; case-only and symlink-object rename; bracket filename; Unicode/space filename preservation; discard cancel/confirm; mixed staged/unstaged discard; staged added-file discard; tracked deletion stage/unstage/restore | Confirmed defects above; all repaired cases passed actual-app replay |
| Projection | Staged filter with stage-all applying repository targets | Passed |
| Branches | Create from selected noncurrent branch; rename; create tracking local branch from remote browser; unset/set upstream | Passed |
| Synchronization | Fast-forward noncurrent branch while current worktree is dirty; push to isolated local bare origin; pull independent peer commit | Passed |
| Merge | Divergent normal merge creates two-parent commit; squash merge leaves expected changes staged and HEAD unchanged | Passed |
| Stash | All with mixed index/worktree/untracked and existing stash; selected paths; staged-only; keep-index; apply; pop; drop cancel/confirm; refusal over dirty conflicting content | Ordinary cases passed; all scope/preview/conflict defects above repaired and replayed |
| Auto stash | Dirty checkout preserves exact index/worktree split and existing stash; failed index restoration retains temporary stash and original stash | Passed data preservation |
| History and conflicts | Hook refusal with no-verify recovery; non-HEAD reword; selected amend with stage-all cancel/retry; fixup and autosquash; cherry-pick conflict abort/resolve/continue; revert conflict recovery | Seven actual-app cases passed |

## Evidence map

- Files: `files-live-results.json`, `fixed-live-results.json`,
  `extra-files-results.json`, `screens/files-*`, `screens/fixed-*`.
- Independent path data-loss reproduction:
  `literal-independent-result.json`, `screens/files-literal-independent/`.
- Rename edges: `case-rename-results.json`, `rename-edge-before-results.json`,
  `screens/case-rename-0/`, `screens/rename-symlink-before/`.
- Independent stash reproduction/replay:
  `live_stash_parent.py`, `parent-stash-before-results.json`,
  `parent-stash-after-results.json`, `stash-literal-final-result.json`,
  and corresponding `screens/parent-stash-*` / `screens/stash-literal-final/`.
- Final rename-edge replay: `rename-edge-after-results.json` and
  `screens/rename-case-final-after/`, `screens/rename-symlink-after/`.
- Exact final executable after the shared-index safeguard:
  `parent-stash-final-results.json`, `screens/parent-stash-*-final/`, and
  `final-build.log`. Executable: `fixed-target/debug/ratagit.exe`.
- Isolated stash discovery: `stash-mythos-live-report.json`,
  `screens/mythos-stash-*`, `mythos-stash-control/`.
- Branch/remote native sessions: `screens/remote-mythos-*` and matching
  controller/fixture records; early non-native sessions are excluded.
- History native sessions: `screens/history-mythos-*` and matching isolated
  controller/fixture records.

Each scenario folder includes `keys.json`, actual screen `.txt` captures,
matching `-git.txt` state, and `terminal.raw` where supported by its driver.

## Validation and limits

All eight repaired defects passed actual-app replay. The final source includes
additional command-scoped shared-index expiration suppression for temporary stash
indexes, with aggressive-expiration success/error regression coverage. Its
ordinary selected-stash live replay passed on the exact final executable:
preview includes untracked content, payload contains only selected paths, and
conflicting pop automatically displays Files while retaining the stash.
Final build, formatting, strict production workspace Clippy and whitespace checks
passed. The full final workspace gate passed **664 tests**, zero failed/ignored,
including the expiration regression; see `final-workspace-complete.log`.
The test command used `--target-dir tmp/git-workflow-audit/fixed-target` to avoid
Windows executable locks from early sessions. No snapshots changed.

An additional actual-app defect remains: a split-index repository reports
`Failed to refresh: unsupported mandatory extension: 'link'`, leaving Files empty.
This comes from libgit2 index access before status fallback, and affects more
than the selected-stash path. Evidence:
`screens/split-index-before/01-selected-stash.txt` and `02-stashed.txt`.
Because Files did not load, `v` selected nothing and `s` opened **all changes**;
this attempted scenario is explicitly excluded from selected-stash success
claims. The original index was still readable through native Git. A separate
follow-up task is offered for complete split-index backend/UI compatibility.
The temporary-index expiration safeguard is backend-tested, not live UI proof
of split-index support.

Coverage is bounded. This Windows pass does not establish Linux/macOS terminal
behavior, SSH/HTTPS authentication, network transport failures, large-repository
performance, every rebase/merge topology, external editor suspension, clipboard
integration, mouse behavior, or every terminal size. Symlink edge evidence uses
Git's Windows link emulation, not privileged native Windows symlink creation.

The early legacy driver session did not quit normally. Automatic approval review
denied a requested process cleanup as potential workload interference. That
cleanup was not retried; it is excluded from product findings and does not block
independently built fixed-executable replay.
