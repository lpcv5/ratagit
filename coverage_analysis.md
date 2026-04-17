# Code Coverage Analysis - ratagit-restart

**Overall Coverage: 53.24% lines (3210/6029)**

## Summary

Based on the coverage report, untested code falls into three categories:

### 1. **Production Runtime Code (0% coverage - NOT dead code)**
These files are actively used but not covered by unit tests:

- `backend\handlers.rs` (0% - 1151 lines) - **ACTIVE** Command handler implementations
- `backend\runtime.rs` (0% - 157 lines) - **ACTIVE** Backend runtime loop
- `backend\events.rs` (0% - 3 lines) - **ACTIVE** Event envelope definitions
- `main.rs` (0% - 14 lines) - **ACTIVE** Application entry point
- `app\keyhints.rs` (0% - 76 lines) - **NEEDS VERIFICATION** Keyhints system

**Status: These are PRODUCTION CODE, not dead code. They need integration tests.**

**Action: ADD INTEGRATION TESTS for runtime code**

### 2. **Unimplemented Features (Low coverage)**
These are implemented but not yet wired to the UI:

- `components\panels\main_view.rs` (10.44%) - Diff viewer not fully wired
- `components\dialogs\modal.rs` (32.77%) - Many modal variants unused
- `components\panels\commit_panel.rs` (31.25%) - Commit details not wired
- `app\input_handler.rs` (14.46%) - V2 input handling not fully active
- `app\runtime.rs` (17.44%) - V2 runtime loop not fully active
- `components\core\simple_list.rs` (18.03%) - Unused UI primitive
- `components\core\multi_select.rs` (25.00%) - Multi-select not wired

**Action: WIRE to UI or mark as future features**

### 3. **Missing Test Coverage (Medium coverage)**
These are active code that needs more tests:

- `app\renderer.rs` (63.89%) - Rendering logic needs edge case tests
- `app\state.rs` (56.36%) - State management needs tests
- `components\panels\log.rs` (50.00%) - Log panel needs tests
- `components\panels\branch_list.rs` (50.00%) - Branch panel needs tests
- `components\panels\stash_list.rs` (52.73%) - Stash panel needs tests
- `app\request_tracker.rs` (50.00%) - Request tracking needs tests
- `components\core\tree_component.rs` (57.20%) - Tree component needs tests
- `components\core\theme.rs` (41.57%) - Theme system needs tests

**Action: ADD TESTS for active code**

## Well-Tested Code (>85% coverage)

- `shared\path_utils.rs` (100%) ✅
- `backend\git_ops\diff.rs` (97.27%) ✅
- `backend\git_ops\branch_graph.rs` (99.11%) ✅
- `backend\git_ops\branches.rs` (98.91%) ✅
- `backend\git_ops\commit_diff.rs` (92.31%) ✅
- `backend\git_ops\commit_files.rs` (92.44%) ✅
- `components\panels\file_list.rs` (89.16%) ✅
- `app\processors\git_processor.rs` (85.04%) ✅
- `app\processors\modal_processor.rs` (88.89%) ✅
- `components\core\selectable_list.rs` (92.59%) ✅
- `components\component_v2.rs` (93.75%) ✅

## Detailed Breakdown by Category

### Category 1: Production Runtime Code (0% coverage - needs integration tests)

| File | Lines | Coverage | Status |
|------|-------|----------|--------|
| backend\handlers.rs | 1151 | 0% | **ACTIVE** - 26 command handlers, called from run_backend |
| backend\runtime.rs | 157 | 0% | **ACTIVE** - Backend event loop, spawned from main.rs |
| backend\events.rs | 3 | 0% | **ACTIVE** - EventEnvelope type definitions |
| main.rs | 14 | 0% | **ACTIVE** - Application entry point |
| app\keyhints.rs | 76 | 0% | **VERIFY** - May be unused after Intent removal |

**Total: 1,401 lines of production runtime code without test coverage**

**Why 0% coverage?** These files contain the main runtime loop and are only executed when the application runs. Unit tests don't exercise the full event loop.

### Category 2: Unimplemented/Unwired Features (10-35% coverage)

| File | Lines | Coverage | Status |
|------|-------|----------|--------|
| components\panels\main_view.rs | 182 | 10.44% | Diff viewer partially implemented |
| app\input_handler.rs | 83 | 14.46% | V2 input handling exists but not active |
| app\runtime.rs | 281 | 17.44% | V2 runtime exists but not active |
| components\core\simple_list.rs | 61 | 18.03% | UI primitive not used yet |
| components\core\multi_select.rs | 104 | 25.00% | Multi-select feature not wired |
| components\dialogs\modal.rs | 296 | 32.77% | Many modal types not used |
| components\panels\commit_panel.rs | 192 | 31.25% | Commit details not wired |

**Total: 1,199 lines of unwired features**

### Category 3: Active Code Needing Tests (40-65% coverage)

| File | Lines | Coverage | Priority |
|------|-------|----------|----------|
| components\core\theme.rs | 89 | 41.57% | Medium |
| app\request_tracker.rs | 12 | 50.00% | High |
| components\panels\log.rs | 38 | 50.00% | Medium |
| components\panels\branch_list.rs | 64 | 50.00% | High |
| components\panels\stash_list.rs | 55 | 52.73% | High |
| app\state.rs | 110 | 56.36% | High |
| components\core\tree_component.rs | 257 | 57.20% | Medium |
| app\renderer.rs | 252 | 63.89% | Medium |

**Total: 877 lines needing better test coverage**

## Recommendations

### Immediate Actions (Priority 1)

1. **Verify and clean up keyhints.rs**:
   - Check if `app\keyhints.rs` is still used after Intent system removal
   - If unused, delete it
   - If used, add tests

2. **Add tests for active panels** (Priority: High):
   - `components\panels\branch_list.rs` - Test branch selection, checkout, delete
   - `components\panels\stash_list.rs` - Test stash apply, pop, drop
   - `app\request_tracker.rs` - Test request tracking logic
   - `app\state.rs` - Test state management

### Medium-term Actions (Priority 2)

3. **Add integration tests for runtime code**:
   - `backend\runtime.rs` - Test backend event loop
   - `backend\handlers.rs` - Test command handlers end-to-end
   - Create integration test that spawns backend and sends commands

4. **Wire or document unimplemented features**:
   - Document `main_view.rs` diff viewer as "in progress"
   - Document `commit_panel.rs` details view as "planned"
   - Mark `multi_select.rs` as "future feature"
   - Mark `simple_list.rs` as "unused primitive"

5. **Add tests for rendering logic**:
   - `app\renderer.rs` - Test edge cases
   - `components\core\tree_component.rs` - Test tree operations
   - `components\core\theme.rs` - Test theme application

### Long-term Actions (Priority 3)

6. **End-to-end tests**:
   - Add full workflow tests (stage → commit → push)
   - Test UI interactions with real Git repositories

## Coverage Goals

- **Current**: 53.24% (3210/6029 lines)
- **After verifying keyhints**: ~53-54% (if keyhints is dead code, remove it)
- **After adding panel tests**: ~65% (testing 400 more lines in panels)
- **After adding integration tests**: ~75% (testing runtime and handlers)
- **Target**: 80%+ for production-ready code

## Key Insight

The 0% coverage files are NOT dead code - they're production runtime code that unit tests don't exercise. The backend runtime loop (`run_backend`) is spawned from `main.rs` and processes commands through handlers. This code runs in production but needs integration tests to verify correctness.
