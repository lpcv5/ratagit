# EXEC_PLAN.md

## Current Slice

Commit Files directory diff performance and command-length fix.

## Goal

- prevent Commit Files folder selections from expanding into very large
  `git show` pathspec lists
- avoid Windows command-line length failures when selected commit folders contain
  many changed files
- cap very large commit file/folder patch previews so Details rendering remains
  responsive
- keep Details behavior the same for users: selecting a commit file shows that
  file's patch, selecting a commit folder shows that folder's combined patch

## Vertical Slice

1. Core selection behavior
- keep file-row selections resolving to the exact changed file entry so rename
  old paths remain included
- resolve directory-row selections to one directory pathspec instead of all
  descendant changed files

2. Git backend behavior
- continue using `git show --patch <commit> -- <pathspec>` for commit-file
  Details
- rely on Git's directory pathspec matching for folder rows instead of sending
  every descendant path as a command argument
- keep rename file targets passing both old and new paths
- apply the existing 1 MiB patch preview cap to commit file/folder diffs

3. Tests
- update core tests so Commit Files directory selections emit one directory
  pathspec
- add a regression test with many files under one directory to prove command
  generation stays bounded
- add a Git CLI integration test proving a directory pathspec returns patches
  for descendant files
- add a backend integration test proving large commit-file patches are truncated
  deterministically

4. Documentation
- record that Commit Files folder diffs use directory pathspecs rather than
  expanded descendant lists

5. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`
