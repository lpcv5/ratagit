# Diff cache budget and freshness

Ratagit retains at most 50 diff entries and approximately 32 MiB of shared diff
payloads in each application's in-memory LRU cache. An entry larger than the byte
budget is displayed normally but is not cached. Entry count and byte limits are
enforced together, including replacements.

## Retained memory

The estimate includes display-line vector capacity, string capacity, the shared
allocation headers, and lossless patch data. Patch accounting includes request
and side paths, OIDs, file/hunk/line vector capacities, hunk headers, exact line
bytes, and encoded patch bytes. Reserved capacity counts even when unused.

Shared line collections and patch objects are identified separately. Multiple
cache entries referencing one allocation pay for it once; different display text
can share the same patch. Removal releases a charge only after the last cache
entry using that allocation is removed. Accounting is updated incrementally;
cache hits do not rescan payload contents.

Insertion replaces any existing value, checks whether the incoming document can
fit by itself, then evicts the least recently used entries until both limits are
satisfied. An oversized replacement removes the previous cached value without
evicting unrelated entries. Invalidation releases accounting while preserving
the relative LRU order of surviving entries.

This is a payload-retention budget. Cache keys, map bookkeeping, allocator
metadata, active previews, in-flight work, and retained UI snapshots are outside
the estimate. A snapshot can keep an evicted allocation alive, so eviction does
not promise an immediate RSS decrease. Shared ownership and copy-on-write keep
retained snapshots and their original lossless patches unchanged.

## Identity and freshness

Each background diff carries one `DiffCacheIdentity` through registration,
execution, completion, lookup, and insertion. It combines a target key with a
revision scoped to the mutable inputs:

| Target | Key inputs | Revision |
| --- | --- | --- |
| Tracked unstaged file | Path and worktree source | Worktree |
| Staged file | Path and index source | Worktree |
| Untracked file | Path and untracked source | Worktree |
| Directory | Path and structured visible path/status/directory fingerprint | Worktree |
| Branch log | Branch name and the same 20-commit limit used for execution | Branches |
| Stash | Index and optional path scope | Stashes |
| Commit | Backend-provided commit OID and optional path scope | Immutable |

Worktree, branch, and stash revisions are independent. Refresh requests advance
the affected scopes and remove their cache entries. Accepted status, branch,
and stash observations also advance their respective scopes, even when file
status labels, branch names, or stash indices are unchanged. Immutable commit
entries survive mutable refreshes. Runtime diff options are currently fixed;
introducing configurable options requires including them in the identity.

Directory fingerprints hash native paths and typed statuses directly, preserving
non-UTF-8 path distinctions on Unix and field boundaries. Their allocation cost
does not grow with the number of visible nodes. The fingerprint describes
membership and status; content freshness comes from the worktree revision.
The existing refresh loop discovers external edits and reference changes. Between
observations, reuse follows the last observed revision without a filesystem scan
on navigation. Each application owns one repository's cache; persistence or
cross-repository sharing would require additional repository identity.

Task-generation acceptance happens first. A current generation must still match
the feature's in-flight identity, the current scoped revision, and the selected
target before publication or caching. The payload identity must also match its
registered request. An obsolete success cannot repopulate the cache. An obsolete
failure stays silent and cannot clear a newer load. Freshness and selection
rejections schedule a new load through the existing coordinator and Flux path.

## Diagnostics and regression coverage

The per-second `ratagit-debug.log` reporter emits a separate `diff_cache` line
with `entries`, `retained_bytes`, `budget_bytes`, `hits`, `misses`, `evictions`,
`oversized`, `invalidations`, and `rejected`. Counters are cumulative for the
application session. `rejected` counts identities rejected at the diff feature
boundary; cancelled generations can be discarded earlier by the task registry.
The reporter is available in debug builds and with the explicit
[`--profile-latency` flag](profiling.md).

Regression tests cover byte-driven LRU pressure, replacement growth/shrinkage,
oversized documents, reserved capacities, partial sharing, empty collections,
zero/exact limits, source/path identity, stale completion races, and immutable
commit reuse. Real temporary repositories exercise external file edits, branch
movement under the same name, stash index reuse, tracked/untracked staging
roundtrips, malformed payload identity, and retained snapshots after eviction.

```powershell
cargo test app::diff
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```
