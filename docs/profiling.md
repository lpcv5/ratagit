# Profiling Ratagit interactions

Build with symbols and release optimization:

```powershell
cargo build --profile profiling
cargo run --profile profiling -- --profile-latency
```

Run from a Git repository in an interactive terminal. `--profile-latency` is
explicitly opt-in in every build, including release. It truncates
`ratagit-latency.jsonl` in the working directory at startup and enables the
per-second `ratagit-debug.log` counters even without debug assertions. Failure to
open the latency file is reported before terminal initialization. `--debug` can
also enable the existing application and detailed Git job logs.

The profiling profile inherits release's `opt-level = "z"`. It retains full
symbols, disables stripping and cross-crate LTO, uses 16 codegen units, and
preserves unwinding. These settings reduce profiling iteration cost; the shipping
release profile is unchanged. Recheck important timing conclusions using
`cargo run --release -- --profile-latency` because code generation differs.

## Read the event stream

Each JSON line has schema version 1. All times use a monotonic clock and
microseconds. No typed characters, paths, branch names, commit messages, file
contents, or credentials are included in latency events.

| Field | Meaning |
| --- | --- |
| `request_id` | One input or refresh origin; shared by its follow-up work |
| `span_id` and `parent_span_id` | A root interaction or a child repository effect |
| `origin` and `operation` | Stable labels such as `input`, `auto_refresh`, `diff`, `status_fast` |
| `generation` | Background task generation, when applicable |
| `elapsed_us` | Time since the originating input/refresh, including debounce and queues |
| `span_us` | Time since this effect was published, or a synchronous result became ready |
| `duration_us` | The duration of this stage, rather than a cumulative timestamp |
| `state_version` | The exact published/rendered snapshot version |
| `label` and `lane` | Native Git job label and worker lane; 0 is primary, external lanes start at 1 |

`action_dequeued` and `command_dequeued` report channel waits. `effect_started`
reports waiting for an executor slot. `git_started` reports the native worker
queue wait, and `git_finished` reports native execution. `dispatch_finished`
measures reduction; `snapshot_built` includes render-cache rebuilding and snapshot
construction. `effect_finished` includes thread launch, backend waits, and result
conversion, so its duration overlaps the native Git timings and must not be
added to them.

Only `frame_drawn`, emitted after successful terminal drawing and flushing, is a
frame latency sample. A root span measures the first visible feedback; a child
effect span measures the first frame of its published result. Both use the
original request's `elapsed_us`, so a diff result includes the input's debounce
delay. Follow-up runtime actions retain their origin. Deferred diff and refresh
origins live in the runtime, outside reducers and domain snapshots.

A deferred diff satisfied synchronously from cache (or an empty target) gets a
`diff_ready` child span. Its result latency still starts at the original input,
even if navigation feedback was drawn before the debounce flush.

An intermediate loading frame is not the result frame. Repeated draws of a span
are counted once. Watch channels can replace publications before drawing:
`snapshot_superseded` records those omissions, and shutdown records
`frame_unobserved` for the final undrawn publication. A failed draw records
`draw_failed`. These events are excluded from frame latency distributions. The
measurement attributes the latest publication actually drawn; it does not invent
samples for earlier publications whose changes may also survive in that frame.

Rejected background generations emit `completion_rejected` and cannot produce a
result sample. A current generation still must cause a publication to produce a
frame sample; a source mismatch or no-op can emit `dispatch_no_snapshot`.
`completion_failed` identifies a current background failure: its visible error
can produce a frame sample, which measures error feedback rather than success.
Quit is not drawn by the normal UI, so its final publication is unobserved.

Summarize median, nearest-rank p95, maximum, and sample counts in PowerShell:

```powershell
./scripts/Measure-Latency.ps1 ./ratagit-latency.jsonl -Verbose | Format-Table -AutoSize
```

The summary groups by metric, origin, and operation. `request_to_result_frame`
includes visible error feedback; inspect `completion_failed` events when
separating successful and failed workloads. Superseded requests are omitted,
so report sample counts and the verbose omitted-publication count alongside
percentiles. Logging writes add overhead; use the same logging settings for
comparisons. A process killed while writing can leave a partial final JSON line;
remove that incomplete line before summarizing the remaining complete records.

## Compare repeatable workloads

Use a disposable repository or fixed test fixture. Record its revision, modified
file count, history/diff size, input sequence, compiler, platform, worker count,
terminal dimensions, build profile, repetitions, and logging flags.

1. Start a fresh process, wait for status, and record its refresh/result frames.
   This resets application caches, not necessarily the OS filesystem cache.
2. Select two modified files repeatedly to compare cold and warm diff requests.
   Deferred cache hits use `diff_ready` result spans without a repository effect;
   immediate synchronous results can also finish in the root's first frame.
3. Move rapidly through selections to exercise debounce, supersession, and
   unobserved publications. Report omitted counts instead of treating these as
   zero-latency successes.
4. Open a fixed large history and diff, then overlap reads with a local write or
   controlled slow backend. Compare input feedback, result latency, and queue
   stages separately. Do not assume network fetch durations are reproducible.

The regression suite exercises actual dispatch, executor completion, native Git
workers, in-memory terminal drawing, coalescing, disabled recording, and draw
failures. It asserts causal identities and lifecycle ordering rather than fragile
wall-clock speed limits:

```powershell
cargo test runtime::latency_tests
cargo test latency::tests
cargo test test_snapshot_profile -- --ignored --nocapture --test-threads=1
```

The last command is the existing allocation experiment. Functional/identity
regressions run in normal CI; performance comparisons require the controlled
workloads above.

The per-second reporter also writes [diff cache statistics](diff-cache.md),
including retained bytes, the byte budget, cache hits/misses, evictions,
oversized entries, invalidations, and rejected diff identities. These counters
help distinguish warm reuse, memory pressure, and obsolete work when comparing
latency traces.
