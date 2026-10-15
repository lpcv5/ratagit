# Rust practices from uv for Ratagit

Ratagit can borrow most from uv's explicit work identities, workload-specific
concurrency limits, reproducible profiling, and separation of domain errors from
diagnostics. Its existing persistent Git workers, immutable snapshots, and stale
completion checks already follow similar principles. The best next steps are to
measure interactive latency, bound retained diff data, preserve error categories,
and add Windows coverage before considering new runtimes or allocators.

Research date: 2026-10-15. The uv source references below are pinned to
[`9385290f1bf895cb3f587118ce92cce9a0585f77`](https://github.com/astral-sh/uv/tree/9385290f1bf895cb3f587118ce92cce9a0585f77).
The Ratagit inspection started at `893ded4e130fcb81b70cc6fff62bfd82ffaa372f`.
This concerns how uv is implemented in Rust; Ratagit should continue using Cargo
to build and test its Rust code.

## Adoption priorities

These priorities are engineering judgments for Ratagit, rather than measured
speedups or requirements imposed by uv. P1 means the next focused improvement;
P2 means implement after measurements establish a need; P3 means defer until
the relevant feature or bottleneck exists.

| Practice | Ratagit today | Decision |
| --- | --- | --- |
| Reproducible profiling and latency attribution | Opt-in causal JSONL events, allocation harness, counters, and worker timings | First implementation complete; P1: collect controlled workload baselines |
| Separate development, profiling, and distribution profiles | Development, profiling, and size-oriented release profiles | Profiling profile implemented; P1: compare size versus speed explicitly |
| Cache identities derived from inputs | Typed source/scope identities, refresh revisions, a 32 MiB payload budget, and counters | Implemented with feature and real-repository regressions |
| Structured errors with contextual hints | Owned backend diagnostics and domain categories survive workers, effects, and completion actions; UI adds contextual hints | Implemented with conversion, worker, dispatcher, real-lock, and UI regressions |
| Platform-aware integration snapshots | Ubuntu/Windows CI, shared isolated process fixtures, Unicode/CRLF regressions, and Unix PTY checks | Implemented and verified locally on Windows MSVC and WSL Ubuntu; Windows console lifecycle remains a separate harness task |
| Resource-specific concurrency | Six concurrent reads, FIFO writes, bounded native worker queues | P2: tune from latency and backlog data; preserve existing ordering |
| Shared in-flight results | Diff in-flight suppression and refresh coalescing | P2: extend only where duplicate backend work is demonstrated |
| Shared immutable identifiers | Shared collections and patches; individual IDs remain strings | P2: investigate identifier allocation after profiling |
| Workspace boundaries | One Cargo package with feature and backend modules | P3: extract a crate only for a concrete independent consumer |
| Alternative allocator and persistent cache formats | System allocator and in-memory diff cache | P3: benchmark an allocator or design persistence only when justified |

## Measure the complete interaction

uv provides a profiling build and a benchmark harness with named workloads. Its
contribution guide also describes exporting tracing durations to inspect
overlapping work and CPU-bound intervals. These are useful techniques even
though package resolution and a Git TUI have different workloads.
Sources: [profiling guide](https://github.com/astral-sh/uv/blob/9385290f1bf895cb3f587118ce92cce9a0585f77/CONTRIBUTING.md#profiling-and-benchmarking),
[benchmark harness](https://github.com/astral-sh/uv/blob/9385290f1bf895cb3f587118ce92cce9a0585f77/scripts/benchmark/README.md).

Ratagit already has several useful measurement points:

- [`runtime/telemetry.rs`](../src/runtime/telemetry.rs) records frame and dispatch
  times, effect times, action and command backlogs, and task counters.
- [`repository/worker.rs`](../src/git/repository/worker.rs) records `queue_ms`
  and `exec_ms` separately for native Git work.
- [`snapshot/measurement.rs`](../src/flux/snapshot/measurement.rs) measures
  snapshot construction, projection, and individual collection clones.
- [`app/README.md`](../src/app/README.md) documents allocation budgets and the
  existing profiling workload.

The missing connection is a trace from an input or refresh request to the first
frame showing its accepted result. Add request identifiers, task generations,
operation kind, worker lane, enqueue/start/finish events, and publication/frame
events. Keep these as structured tracing fields. Avoid recording file contents
or credentials, and keep diagnostic output in files so it cannot corrupt the TUI.

Measure cold application caches separately from repeated navigation with warm
caches. A restarted process does not imply a cold operating-system filesystem
cache. Use fixed fixtures and input sequences, including a large history, a large
diff, rapid selection changes, and a slow read overlapping a local write. Record
median and p95 input-to-frame latency, queue delay, Git execution time, peak
retained bytes, and redundant/stale work. A lower Git execution time alone does
not establish better responsiveness.

Existing commands remain useful:

```powershell
cargo test test_snapshot_profile -- --ignored --nocapture --test-threads=1
cargo run -- --debug
```

The first command is an opt-in allocation/timing experiment; the second opens
the interactive TUI and enables its diagnostic logs. Neither is an automated
end-to-end latency benchmark. The implemented `--profile-latency` flag connects
input, effects, native workers, publications, and successful frames in JSONL,
and enables counters in optimized builds. See the [profiling guide](profiling.md)
for commands, percentile summaries, and sample semantics. Ratagit sets tracing
levels in code, so uv's `RUST_LOG` examples are not Ratagit instructions.

Acceptance: a slow interaction can be attributed to waiting, backend execution,
reducer/snapshot processing, or drawing using the same request identity. Record
the fixture, compiler, platform, build profile, repetitions, and logging settings
with every performance comparison.

## Use build profiles for distinct questions

uv separates an optimized profiling build with symbols from its distribution
build. Its development profile uses `debug = "line-tables-only"`; the profiling
profile changes code generation and LTO settings to shorten iteration. Those
choices are deliberate tradeoffs, so profiling and shipping binaries are not
identical. Source:
[uv build profiles](https://github.com/astral-sh/uv/blob/9385290f1bf895cb3f587118ce92cce9a0585f77/Cargo.toml#L479-L571).

Ratagit's [`Cargo.toml`](../Cargo.toml) currently combines `opt-level = "z"`,
LTO, one codegen unit, and stripping in release. That prioritizes binary size;
it does not establish the fastest history, graph, or diff processing.

Ratagit now provides this profiling profile:

```toml
[profile.profiling]
inherits = "release"
debug = "full"
strip = false
codegen-units = 16
lto = false
panic = "unwind"
```

This retains Ratagit's release optimization level while making stack inspection
and iteration easier. Validate important conclusions again with the actual
release binary. Separately compare `opt-level = 3` against `"z"` using identical
fixtures, recording executable size, build time, memory, and interaction latency.
Cargo documents the profile settings and their interactions in its
[profile reference](https://doc.rust-lang.org/cargo/reference/profiles.html).

Do not copy uv's release `panic = "abort"` into Ratagit as a routine size
optimization. Ratagit uses unwind-dependent guards and worker panic handling.
Panic hooks can still run under abort, but stack unwinding and `Drop` cleanup do
not. Any such change would require a separate terminal and worker failure review.
The existing behavior is covered by [`terminal.rs`](../src/terminal.rs),
[`effects.rs`](../src/flux/effects.rs), and terminal lifecycle tests.

The smaller development debug-information setting is also worth timing locally.
Its cost is reduced variable/type inspection in a debugger; uv's build-time
results should not be treated as Ratagit's expected gain.

## Make cache identity and retention explicit

uv varies cache identity by source: Git dependencies use resolved commit hashes,
while mutable local inputs require freshness rules. It also versions persistent
cache buckets independently. The transferable principle is that an entry is
reusable only when all inputs that determine its result are still represented
by its identity. Source:
[uv cache rules](https://github.com/astral-sh/uv/blob/9385290f1bf895cb3f587118ce92cce9a0585f77/docs/concepts/cache.md).

Ratagit's [`DiffCacheIdentity`](../src/app/diff/cache.rs) separates file,
directory, branch, commit, and stash targets and carries scoped freshness
revisions. [`DiffState`](../src/app/diff/state.rs) validates the current request,
revision, and selection before publishing results. Commit entries survive mutable
invalidations. The backend's
[`ReachabilityCache`](../src/git/repository/history.rs) already checks reference
tips and the requested history limit before reuse.

These adaptations are now implemented in Ratagit; the [cache guide](diff-cache.md)
documents accounting, identity, diagnostics, and regression coverage. They are
Ratagit engineering choices rather than features copied from uv's cache:

1. **Bound retained data as well as entry count.** `DiffCache` retains
   at most 50 entries and approximately 32 MiB of display text and lossless patch
   payloads, with cache hit/miss/eviction counters. Oversized documents remain
   available as active previews without being cached. Shared payloads count once
   within the cache; snapshots and the active
   preview can keep an evicted `Arc` alive, so this budget is not a hard process
   RSS limit. Keep the simple LRU unless measurement justifies changing it.
2. **Describe immutable and mutable identities separately.** A commit diff can
   use its OID and path scope; diff options are currently fixed. Branch names and
   stash indices can refer to different content later, so their identities carry
   independent invalidation revisions. Worktree, index, and untracked sources are
   distinct and carry the worktree revision after writes and observed external edits.
   A future cross-repository cache also needs repository/worktree identity.
3. **Use structured directory inputs.** Directory keys hash native path values,
   typed statuses, and directory flags without intermediate path strings or a
   joined buffer. Content freshness still uses refresh revisions; navigation does
   not perform a full-tree content scan.
4. **Keep cache freshness and task freshness distinct.** A task generation answers
   whether a completion is still wanted; it does not make a mutable cached value
   current. The scoped revision travels through lookup, execution, invalidation,
   and completion acceptance so an old read cannot repopulate a new cache epoch.

Regression cases should cover a branch moving without changing its name, a stash
index being reused, staged/unstaged transitions, external file edits, and a
refresh racing a read. Byte-budget tests should include an oversized patch,
replacement accounting, eviction order, and a retained old snapshot.

## Limit concurrency by resource

At the inspected revision, uv builds a current-thread Tokio runtime, configures
separate limits for downloads, builds, installs, and cache reads, and initializes
its Rayon pool lazily. Async coordination and parallel CPU work are separate
decisions. Sources:
[runtime construction](https://github.com/astral-sh/uv/blob/9385290f1bf895cb3f587118ce92cce9a0585f77/crates/uv/src/lib.rs#L3112-L3139),
[concurrency configuration](https://github.com/astral-sh/uv/blob/9385290f1bf895cb3f587118ce92cce9a0585f77/crates/uv-configuration/src/concurrency.rs),
[thread pool initialization](https://github.com/astral-sh/uv/blob/9385290f1bf895cb3f587118ce92cce9a0585f77/crates/uv-threads/src/lib.rs).

Ratagit also uses current-thread Tokio. Its
[`EffectExecutor`](../src/flux/runtime.rs) runs at most six reads concurrently
and preserves FIFO local writes. Fetch shares the read lane. Native repository
workers have bounded queues of 128 jobs; the external pool defaults to two to
six workers and supports `RATAGIT_EXTERNAL_WORKERS`, clamped to 1–12.

Retain that design. First measure whether slow history or fetch work delays
interactive diffs. If it does, reserve read capacity or adjust execution
selection for interactive work without changing write order. Distinguish the
[`TaskManager`](../src/flux/task_manager.rs) admission rule from execution:
high-priority admission bypass does not itself provide a priority execution queue.

Also distinguish bounded execution from bounded retention. The effect read/write
queues and runtime action/command channels can retain queued work beyond the
worker limits. Start with coalescing superseded reads and measuring backlog.
Any bounded-channel change must avoid blocking the single dispatch loop on a
consumer that needs that loop to make progress, and must never silently drop a
write, completion, or shutdown event.

Acceptance: with slow history/fetch work, the selected diff can still progress,
local writes remain FIFO, queued memory remains explainable, and shutdown does
not wait for uncancellable native Git calls. Use the existing controlled runtime
and worker scheduling tests to verify these contracts.

## Share repeated reads without weakening cancellation

uv's `OnceMap` lets one producer compute a value while other callers await the
same result. It recommends shared values when cloning would otherwise be costly.
Its registered-entry API also uses borrowing to constrain removal while handles
exist. Producers must eventually complete their registrations or waiters can
hang. Sources:
[OnceMap](https://github.com/astral-sh/uv/blob/9385290f1bf895cb3f587118ce92cce9a0585f77/crates/uv-once-map/src/lib.rs),
[registered entries](https://github.com/astral-sh/uv/blob/9385290f1bf895cb3f587118ce92cce9a0585f77/crates/uv-once-map/src/registered.rs).

Ratagit already suppresses an identical in-flight diff in `prepare_reload` and
coalesces pending refresh kinds in
[`background_poll.rs`](../src/app/background_poll.rs). Do not add a second global
task registry merely to reproduce uv's utility.

If traces show repeated equivalent backend reads, prototype sharing at the
effect/backend boundary. Start with immutable commit-file reads. Use a concrete
request identity containing its inputs and freshness revision, rather than the
existing `TaskKey`: several keys intentionally describe cancellation slots.
For example, all selected diffs use the `active` slot, and commit-file requests
share one slot even when their OIDs differ.

Each consumer must still pass its generation and selected-source checks before
updating state. Cancelling one consumer must not strand others. Producer failure,
panic, cancellation, and shutdown must settle every waiter. Mutable reads need a
new identity after repository changes, and retryable failures should not become
permanent memoized results. Keep the first implementation owned by an existing
runtime boundary; a concurrent map dependency is not automatically necessary.

Acceptance: N equivalent reads produce one backend call and N settled consumers;
different OIDs, limits, modes, or epochs do not merge. Include failure and
producer-cancellation cases, not just the success path.

## Retain structured errors until presentation

uv separates error handling from user-facing diagnostics. Its diagnostics layer
supports contextual hints through `Hinted`; command dispatch distinguishes user,
argument, and unexpected errors. Sources:
[diagnostic hints](https://github.com/astral-sh/uv/blob/9385290f1bf895cb3f587118ce92cce9a0585f77/crates/uv-errors/src/lib.rs),
[top-level error routing](https://github.com/astral-sh/uv/blob/9385290f1bf895cb3f587118ce92cce9a0585f77/crates/uv/src/lib.rs#L3139-L3167).

Implemented: [`GitError`](../src/git/error.rs) owns libgit2's raw code, class,
message, and a domain category. Native diagnostics use `Arc`; operation contexts
and related failures remain typed sources. Both persistent worker lanes retain
these values, and worker construction reports repository-open failures directly.
[`effects.rs`](../src/flux/effects.rs), all domain completion actions, and
[`BackgroundCompleted`](../src/background.rs) carry `GitError` without converting
it to a string. Repository operation state also retains the structured failure.
Git subprocess failures retain exit code, stdout, and stderr; process I/O failures
retain their kind and OS error code.

[`app/git_error.rs`](../src/app/git_error.rs) formats errors at presentation.
Conflict, authentication/certificate, lock, missing reference/path, existing
reference, invalid state, stale patch, and worker failures receive specific
guidance. Hints precede diagnostics because the command log truncates long rows.
The full diagnostic, including code/class and operation context, remains in the
log entry. Reducers use owned data and never interrogate a live `git2::Error`.
Legacy application messages and unclassified subprocess exits stay unclassified;
no category is inferred from English error text.

No automatic retry or lock removal is introduced. A successful repository control
followed by a failed state reload explicitly says the operation completed and
must not be repeated. If both steps fail, both original errors survive. A
selection operation that stages paths before an unstage failure reports partial
progress and asks for a refresh and review before repetition.

Regressions distinguish identical backend message text with different codes,
verify both worker lanes and cloned completions, and check command-log
invalidation through the production dispatcher. A real `index.lock` fixture
checks that failed staging preserves the lock and leaves the on-disk index
unchanged. Cancelled, superseded, duplicate, and freshness-rejected failures stay
silent. Stale hunk regressions check the reload/reselect hint, and UI golden
buffers cover the changed visible diagnostics. Focused commands:

```bash
cargo test git_error
cargo test git::error
cargo test git::repository::worker::tests
cargo test app::background_tests
cargo test app::diff_cache_tests
cargo test app::hunk_tests
cargo test ui::
```

## Extend existing snapshots and platform tests

uv uses `insta` snapshots, temporary test contexts, and filters for unstable
output such as versions and timing. The shared command formatter captures exit
status, stdout, and stderr and handles platform differences. Sources:
[test context and snapshot helpers](https://github.com/astral-sh/uv/blob/9385290f1bf895cb3f587118ce92cce9a0585f77/crates/uv-test/src/lib.rs),
[snapshot review workflow](https://github.com/astral-sh/uv/blob/9385290f1bf895cb3f587118ce92cce9a0585f77/CONTRIBUTING.md#testing).

Ratagit already has checked-in cell/style/cursor goldens, isolated runtime tests,
repository fixtures, and [`terminal_lifecycle.rs`](../tests/terminal_lifecycle.rs).
Keep the [existing golden workflow](../src/ui/goldens/README.md). Adopting `insta`
is optional; replacing the current cell-aware format would not itself improve
coverage.

Implemented: the [CI matrix](../.github/workflows/ci.yml) runs formatting,
compilation, tests, and Clippy on `ubuntu-latest` and `windows-latest`, with
`--locked` and the included application `Cargo.lock` for dependency resolution.
The lockfile is no longer ignored, so clean checkouts use the tested versions.
A failing platform does not cancel
the other job. Golden regeneration remains a reviewed local action.

[`ProcessFixture`](../tests/support/mod.rs) supplies a unique Unicode working
directory and private home, config, cache, temporary and Git configuration paths
for each child. Its environment starts empty, retaining executable/DLL lookup
and necessary Windows system variables, then adds explicit fixture values.
Inherited Git redirects, config parameters, and app/golden overrides are removed;
Git prompting and hooks are disabled. Both output pipes drain concurrently and
the child has a 30-second deadline plus kill/wait cleanup on failure.

[`src/process_tests.rs`](../src/process_tests.rs) re-executes exactly one dedicated
test entry in a child. Before any repository or worker starts, that child sets
libgit2's config search paths to its fixture directory. This isolates libgit2's
platform lookup as well as Git CLI lookup without mutating the parallel parent
test process. Windows Known Folder APIs ignore `APPDATA` overrides, so keymap
scenarios inject an explicit file path through the production load/save logic.
Missing and malformed keymaps recover silently, CRLF configuration overrides
load correctly, and failed persistence retains the existing default fallback.

Shared process regressions cover:

- Hostile Git/app environment overrides and matching native/CLI Git identity.
- Unicode names and spaces in repository roots and tracked paths, with raw CRLF
  bytes preserved through hunk stage and unstage. Windows tests backslash path
  separators; Unix additionally tests a literal backslash in a filename.
- Diagnostics retaining relative paths, categories, native codes/classes and exit
  status. Normalization replaces only temporary root prefixes, CRLF transport
  newlines and ANSI SGR color styles; native separators and console control
  sequences remain visible.
- Queued primary/external worker results settling after the repository handle is
  dropped, bounded child exit, and repository-directory rename after child exit.
- Independent ASCII/Unicode graph environments in different children. The former
  environment-mutating unit test now exercises a pure value conversion.

[`terminal_lifecycle.rs`](../tests/terminal_lifecycle.rs) runs missing-repository,
malformed `.git`, and unusable profiling-output startup cases on both platforms,
asserting a nonzero exit and no premature terminal output. The directory-as-file
failure retains Windows access-denied code 5 and Unix directory code 21, rather
than hiding their difference. Cross-platform Clippy also guards against libgit2
error-class enum signedness differences. Unix quit, profiling, and panic PTY
tests also use the isolated environment. Windows console lifecycle
coverage still requires a dedicated harness; Unix PTYs cannot establish it.

Run `cargo test process_tests` and `cargo test --test terminal_lifecycle` for the
focused cases. Both full suites were also run locally with Windows MSVC and WSL
Ubuntu 24.04; the ignored child entry is invoked explicitly by its parent tests.

## Preserve sharing before changing representations

uv's `SmallString` is an immutable identifier backed by `ArcStr`. Its name should
not be read as a promise of inline small-string storage. Source:
[identifier representation](https://github.com/astral-sh/uv/blob/9385290f1bf895cb3f587118ce92cce9a0585f77/crates/uv-small-str/src/lib.rs).

Ratagit's [`Shared<T>`](../src/shared.rs) and
[`DiffDocument`](../src/git/patch.rs) already share collection and patch payloads;
snapshot clones therefore do not require cloning every history item. Keep those
allocation regressions. Before changing individual strings, measure construction
and graph processing, where commit OIDs and parent OIDs are still repeatedly
represented as `String`.

If those allocations matter, consider a backend-neutral validated commit ID or
shared immutable string for repeated identifiers. Keep editable input buffers
owned and mutable. Avoid a blanket conversion to `Arc<str>`: reference-count
updates and conversions have costs, and collection sharing already removes many
clone costs. Maintain the Git backend abstraction when choosing an ID type.

## Defer changes that do not yet earn their complexity

**Workspace extraction.** uv uses a Cargo workspace with internal crates and
central dependency settings. Ratagit is one binary with explicit feature,
capability, runtime, and backend boundaries. Preserve those boundaries first.
Consider extracting backend-neutral Git contracts only when another binary,
backend, or library consumer needs them. A large workspace copied from uv would
add maintenance without proving a performance benefit. Source:
[uv workspace](https://github.com/astral-sh/uv/blob/9385290f1bf895cb3f587118ce92cce9a0585f77/Cargo.toml#L1-L19).

**Allocator replacement.** uv selects mimalloc on Windows and jemalloc on certain
other targets through a dedicated crate. This is a platform-specific implementation
choice, not a universal optimization. Benchmark Ratagit's allocation-heavy
workloads first; compare latency, RSS, executable size, and build cost. Preserve
the test-only allocation counter, and remember that Rust's global allocator does
not automatically replace allocation inside native libgit2. Sources:
[allocator selection](https://github.com/astral-sh/uv/blob/9385290f1bf895cb3f587118ce92cce9a0585f77/crates/uv-performance-memory-allocator/src/lib.rs),
[allocator dependencies](https://github.com/astral-sh/uv/blob/9385290f1bf895cb3f587118ce92cce9a0585f77/crates/uv-performance-memory-allocator/Cargo.toml).

**Persistent cache.** Ratagit currently needs an in-memory diff cache, not uv's
on-disk package cache. If persistence later improves startup, begin with immutable
commit data, repository identity, a format version, and recoverable corruption.
uv's filesystem helpers demonstrate temporary-file writes and replacement,
including Windows retry handling. Atomic visibility alone is not a complete
crash-durability guarantee; durability and concurrent writers need an explicit
contract. Source:
[filesystem helpers](https://github.com/astral-sh/uv/blob/9385290f1bf895cb3f587118ce92cce9a0585f77/crates/uv-fs/src/lib.rs).

**Larger thread stacks.** uv configures stack sizes for known deep futures and
platform differences. Ratagit should investigate measured stack pressure or a
reproduced overflow before increasing every worker's stack or moving startup to
another thread. Source:
[stack size rationale](https://github.com/astral-sh/uv/blob/9385290f1bf895cb3f587118ce92cce9a0585f77/crates/uv-threads/src/lib.rs).

## Suggested implementation order

1. Connected latency tracing and the profiling profile are implemented. Collect
   fixed cold/warm workload baselines and compare `"z"` versus `3` without
   changing the release default until results justify it.
2. The diff-cache byte budget and identity/freshness regressions are implemented,
   preserving shared payloads and task-generation rejection rules.
3. Structured Git failures now survive effects and completion actions, with
   targeted hints and process diagnostic regressions.
4. Windows CI and isolated process fixtures are implemented. Continue using the
   platform evidence to refine filesystem coverage and add a dedicated Windows
   console lifecycle harness when needed.
5. Use traces to decide whether resource reservations, shared in-flight reads, or
   identifier representation changes deserve implementation.

Keep each change independently reviewable. Extend the existing dispatcher,
runtime, and temporary-repository test helpers for changed behavior, then run:

```powershell
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Functional and allocation invariants belong in normal CI. Timing comparisons
belong in controlled benchmark runs; choose any latency thresholds from a recorded
baseline rather than copying uv's package-management benchmark numbers.
