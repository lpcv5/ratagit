# EXEC_PLAN.md

## Current Slice

Large-repository commit list backend optimization.

## Goal

- bring `HybridGitBackend` commit-list and commit-pagination performance close
  to the parsed Git CLI baseline in synthetic large repositories
- preserve `CommitHashStatus` semantics for merged, pushed, and unpushed commits
  so commit rewrite safety checks remain strict
- reuse the existing `tmp/perf/suite/large` synthetic repository for validation
  and avoid regenerating large test data during the optimization loop

## Vertical Slice

1. Backend behavior
- read commit page metadata through `git log` formatted output instead of
  per-commit libgit2 revwalk object lookup on the hot path
- parse full hash, short hash, parents, author, and full message from
  NUL-delimited CLI output so multiline commit bodies remain stable
- keep a libgit2 fallback if the Git CLI page read fails

2. Commit status classification
- classify page commit hashes in batches by walking `main` and upstream tips
  once per page instead of running reachability checks per commit
- preserve detached-head behavior by skipping upstream classification when HEAD
  is detached
- keep `MergedToMain` precedence over `Pushed`, with remaining commits marked
  `Unpushed`

3. Performance suite
- rerun the release large-only suite without `--regenerate`
- require `commits` and `load-more-commits` backend medians to be no more than
  1.25x the parsed Git CLI medians
- ensure `status`, `commit-details-diff`, and `files-details-diff` do not
  regress by more than 10% from the prior release baseline

4. Tests
- add parser coverage for commit metadata, multiline message bodies, merge
  parents, and empty-summary filtering
- add batch hash-status classification coverage for main, upstream, unpushed,
  and detached-head behavior
- keep existing integration, snapshot, and harness coverage passing without UI
  changes

5. Documentation
- record the optimization slice and release performance validation command

6. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`
- run `cargo run --release --bin perf-suite -- --scales large --operations
  commits,load-more-commits,status,commit-details-diff,files-details-diff
  --iterations 5 --warmup 1`
