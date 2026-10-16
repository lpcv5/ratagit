# Git execution efficiency audit

Date: 2026-10-16. Platform: Windows, Git 2.55.0.windows.5, Cargo 1.99.0.
All timings below are local release-build measurements in milliseconds.

The user's tolerance is to ignore differences below approximately 150 ms. On the
current workspace, no measured backend read justifies optimization by that rule.
Two synthetic large-repository cases do exceed that tolerance. This delivery
records the findings; production code and behavior are unchanged.

## Current workspace

Read-only measurements on this checkout (182 tracked paths, 270 commits), with
one warmup and seven recorded samples per operation:

| Backend operation | Median ms | Min-max ms |
| --- | ---: | ---: |
| Files/status | 43.4 | 41.6-46.2 |
| First 100 commits | 43.1 | 42.4-45.0 |
| Branches | 3.0 | 2.5-4.5 |
| Stashes, currently empty | 0.024 | 0.021-0.025 |
| HEAD commit files | 40.8 | 40.5-42.0 |
| HEAD commit preview | 44.4 | 43.1-45.8 |
| Files text preview, changed execution plan | 79.5 | 78.6-80.7 |

This is backend execution time. UI debounce, input polling, worker queue delay,
and rendering are separate and are not included in these numbers.

## Standard synthetic suite

One warmup and five samples for each operation/runner/scale combination. The
suite compares raw CLI output, CLI plus parsing, and the real backend. All 315
measurement records and 450 profile records succeeded.

| Operation | 20 files backend | 1,000 files backend | 10,000 files backend | 10,000 files raw CLI |
| --- | ---: | ---: | ---: | ---: |
| Status | 39 | 56 | 160 | 160 |
| Commits | 39 | 41 | 53 | 56 |
| Load more commits | 39 | 40 | 48 | 48 |
| Commit files | 38 | 41 | 46 | 45 |
| Commit preview | 39 | 46 | 60 | 60 |
| Commit file preview | 37 | 38 | 40 | 40 |
| Files text preview | 84 | 81 | 92 | 90 |

The 10,000-file status result is near the tolerance boundary, but adds essentially
no backend overhead over Git itself. Small timing differences in the table are
not evidence of an optimization opportunity. The Files text preview runs staged
and unstaged Git diffs; its roughly doubled process cost remains below tolerance.

## Material finding: commit classification walks full history

[Commit status classification](../libs/ratagit-git/src/hybrid.rs) calls
`mark_reachable_commits` from the main tip, and then from the upstream tip for
remaining unclassified entries. The walk stops early only when all requested
unclassified commits have been found. If even one is unreachable, it exhausts
that tip's history. A 100-row page is therefore not a 100-commit work bound.

The focused fixture uses packed linear main histories, 120 additional feature
commits, no upstream, and a 100-commit first page. Each row has one warmup and
seven recorded samples; commit counts and classifications are asserted.

| Main history | Main page backend | Feature page backend | Feature raw log | Feature overhead |
| --- | ---: | ---: | ---: | ---: |
| 1,000 commits | 41.0 | 41.4 | 39.6 | 1.9 |
| 10,000 commits | 40.2 | 45.9 | 39.2 | 6.7 |
| 50,000 commits | 39.7 | 71.4 | 40.0 | 31.4 |
| 250,000 commits | 40.4 | 223.7 | 40.4 | 183.3 |

At 250,000 commits the backend range was 215.6-229.8 ms; raw log was 39.5-45.0 ms.
This reproducible gap exceeds the user's tolerance. Source inspection identifies
the full-history classification walk as the extra work; the comparison does not
provide a separate CPU profile of every internal substep.

Recommended priority: address this first when supporting long-history repos.
Evaluate a batched reachability strategy that handles negative membership queries
efficiently. Preserve `MergedToMain` precedence over `Pushed`, then `Unpushed`.
Validate divergent branches, merges, unrelated histories, detached HEAD, upstream
changes, and deep pagination before selecting an implementation. The git2
fallback also makes per-commit graph queries and needs the same evaluation.

## Material finding: historical native patches scale with the index

[Native patch loading](../libs/ratagit-git/src/patch_backend.rs) explicitly opens
and force-refreshes the index for every request, including Commit and Stash.
Measuring the same one-file root-commit patch against different synthetic index
sizes gives:

| Index entries | Native Commit patch median | CLI commit-file preview median |
| --- | ---: | ---: |
| 1,000 | 1.0 | 39.9 |
| 10,000 | 3.8 | 41.0 |
| 100,000 | 39.5 | 39.6 |
| 500,000 | 260.1 | 39.8 |

At 500,000 entries, native patch loading ranged from 247.1 to 292.2 ms. Its
increase from the small-index case is about 259 ms. Both APIs read the same
commit/path, but return different representations; the CLI comparison is a
control for commit content, not a replacement with equivalent partial-edit data.

The fixture stores repeated small-blob entries in an on-disk index; it does not
create 500,000 physical files. The commit remains one file, deliberately isolating
index-size dependence from patch complexity. Native patch file count is asserted.

Recommended priority: optimize when large-index native previews are a target.
Keep worktree/index requests fresh after external writes. Merely skipping the
explicit refresh is insufficient to guarantee index independence: an exploratory
invalid-index test still failed through the historical patch path after that
change, indicating another index dependency in libgit2's patch processing. That
probe and the small initial change were withdrawn; no speculative fix remains.

## Scheduling and measurement limits

- Read execution already uses four workers and split panel refreshes. Slow status
  need not prevent branches/commits from loading on other workers. Details have
  an 80 ms debounce and reducer-owned caches/request validation.
- [Worker dispatch](../libs/ratagit-harness/src/async_runtime.rs) assigns commands
  round-robin to individual FIFO channels. Already-dispatched work is not replaced
  by newer Details requests. A Details request can wait behind a slow read on its
  chosen worker. This is a source-level risk, not a measured queue-delay finding;
  use existing `queue_delay_ms` and execution traces to establish its frequency.
- The standard suite does not exercise native `load_patch`, long histories,
  branch/stash timing, or complete async UI latency. Its backend profile stage
  often wraps the entire call and cannot by itself identify an internal cause.
- These are warm-cache local samples. No network push/pull, hooks, remote drives,
  cold-cache runs, million-file worktrees, or terminal interaction was measured.
  Empty-page load-more results on short histories are not deep-pagination evidence.
- No production code, dependency, runtime ordering, or Git mutation changed.

## Reproduction and local artifacts

Run the checked-in suite from the repository root:

```powershell
cargo run --release --locked --bin perf-suite -- --scales smoke,small,medium --operations status,commits,load-more-commits,commit-files,commit-details-diff,commit-file-diff,files-details-diff --iterations 5 --warmup 1 --profile --output tmp/perf/results/git-audit-before
```

Its report ID is `perf-1792121459-518808000`; JSON and Markdown reports are under
`tmp/perf/results/git-audit-before/`. The suite creates or reuses its marked
synthetic repositories under `tmp/perf/suite/` and resets those fixtures only.

The supplemental disposable Cargo program is retained locally at
`tmp/git-read-audit/src/main.rs`, with its manifest and lockfile beside it.
It generates packed histories with `git fast-import`, and synthetic index entries
with git2. It asserts first-page counts/classifications and native file count.
The program uses git2 0.20.4 and the local backend; its offline lockfile is separate
from the workspace lockfile. All outputs and fixture mutations are under `tmp/`.

```powershell
cargo run --offline --release --manifest-path tmp/git-read-audit/Cargo.toml --target-dir target -- tmp/perf/git-read-audit-baseline
cargo run --offline --release --manifest-path tmp/git-read-audit/Cargo.toml --target-dir target -- --workspace-read
```

Recorded output is in `tmp/perf/git-read-audit-threshold.txt` and
`tmp/perf/git-read-audit-workspace.txt`. These supplemental artifacts are ignored
local files, not shipped benchmark tooling. Fresh historical fixtures should use
a fresh output root; the retained fixture root can also be reused by this probe.
