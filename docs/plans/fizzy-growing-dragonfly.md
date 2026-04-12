# Ratagit 重构计划：前后端分层与组件化（最终版）

## 1. 背景与现状

Ratagit 是基于 Rust + ratatui 的 Git TUI 客户端，目前已实现文件状态、分支、提交、暂存四个核心面板。当前 `App` 模块承担过多职责：

- UI 渲染逻辑（约 6 个 `render_*` 方法）
- 输入事件处理（按键匹配与状态修改混在一起）
- 后端通信（事件处理与本地状态编排耦合较紧）
- 状态管理（18 个字段散落在 `App` 中）

**重构目标**：
1. **前后端分离**：后端负责 Git 数据与副作用，前端负责 UI 渲染与视角状态。
2. **组件化前端**：基于 Ratatui 官方组件模式，抽取可复用的原子组件和业务组件。
3. **清晰的数据流**：细粒度事件通信，避免全量状态克隆。
4. **易于扩展**：新增面板、模态、树形视图等均有明确的扩展路径。

### 1.1 执行策略切换（MVP 特权）

当前项目处于 MVP 阶段，允许进行 **Gen2 全量重构**：
- 接受目录与模块结构重排（`src/app`、`src/backend`、`src/components` 重构）
- 接受中间阶段的内部 API 破坏（不要求对外兼容）
- 仍要求每个阶段末可编译、可运行、可回归核心交互

目标是一次性把架构债务清理到位，而不是做最小扰动修补。

## 2. 架构原则

### 2.1 后端管理“事实”（Facts）
后端只负责：
- Git 数据读取（status / branches / commits / stashes / diff）
- Git 副作用操作（stage / unstage / checkout / commit / stash 等）
- 将结果通过细粒度事件发送给前端

后端**不负责**任何 UI 视角状态（`active_panel`、`selected_idx`、`scroll` 等）。

### 2.2 前端管理“视角”（View State）
前端 `App` 负责：
- 焦点面板
- 各列表选中状态（使用 `ListState`）
- 滚动偏移
- 模态状态
- 将用户输入映射为本地 UI 操作或后端命令

### 2.3 通信使用细粒度事件
- 不再使用全量状态快照。
- 后端发送携带所有权的数据事件（如 `FilesUpdated(Vec<StatusEntry>)`）。
- 前端被动接收并更新本地数据缓存。

### 2.4 组件遵循 Ratatui 官方模式
- 定义 `Component` trait，包含 `handle_event` 和 `render`。
- 组件不持有状态，通过 `&UiState` 和 `&CachedData` 读取上下文并返回 `Intent`。
- 组件返回 `Intent` 而非直接发送命令，保持与后端的解耦。

## 3. 架构全景图

```
┌──────────────────────────────────────────────────────────────────┐
│                         Frontend (UI Thread)                      │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                         App (容器)                          │  │
│  │  - ui_state: UiState (active_panel, panel_states, scrolls) │  │
│  │  - data_cache: CachedData (files, branches, commits, ...)  │  │
│  │  - components: 各面板组件实例                                │  │
│  │  - cmd_tx, event_rx                                         │  │
│  └────────────────────────────────────────────────────────────┘  │
│         │                      │                      │           │
│         ▼                      ▼                      ▼           │
│  ┌─────────────┐        ┌─────────────┐       ┌─────────────┐    │
│  │FileListComp │        │BranchListCmp│       │   ...       │    │
│  │(Component)  │        │(Component)  │       │             │    │
│  └─────────────┘        └─────────────┘       └─────────────┘    │
│         │                      │                      │           │
│         └──────────────────────┼──────────────────────┘           │
│                                ▼                                 │
│                    handle_event() → Intent                        │
│                                │                                 │
│              ┌─────────────────┼─────────────────┐               │
│              ▼                 ▼                 ▼               │
│       本地 UI 更新         发送 Command       无操作              │
│    (修改 UiState)       (通过 cmd_tx)                            │
└──────────────────────────────────────────────────────────────────┘
                                │
                      BackendCommand (mpsc)
                                ▼
┌──────────────────────────────────────────────────────────────────┐
│                       Backend (Tokio Task)                        │
│  - 接收 BackendCommand                                            │
│  - 调用 git_ops 执行 Git 操作                                      │
│  - 发送 FrontendEvent (携带数据所有权)                             │
│  - 不持有任何 UI 状态                                              │
└──────────────────────────────────────────────────────────────────┘
                                │
                      FrontendEvent (mpsc)
                                ▼
                    前端 event_rx 接收 → 更新 CachedData
```

## 4. 通信协议

### 4.1 BackendCommand（前端 → 后端）

```rust
pub enum BackendCommand {
    // 数据读取
    RefreshStatus,
    RefreshBranches,
    RefreshCommits { limit: usize },
    RefreshStashes,
    GetDiff { file_path: String },

    // 副作用操作（按需实现）
    StageFile { file_path: String },
    UnstageFile { file_path: String },
    CheckoutBranch { branch: String },
    CreateCommit { message: String },
    StashApply { index: usize },
    StashPop { index: usize },
    StashDrop { index: usize },

    Quit,
}
```

### 4.2 FrontendEvent（后端 → 前端）

```rust
pub enum FrontendEvent {
    // 数据更新（携带所有权，无克隆）
    FilesUpdated { files: Vec<StatusEntry> },
    BranchesUpdated { branches: Vec<BranchEntry> },
    CommitsUpdated { commits: Vec<CommitEntry> },
    StashesUpdated { stashes: Vec<StashEntry> },
    DiffLoaded { file_path: String, diff: String },

    // 操作反馈
    ActionSucceeded { message: String },
    ActionFailed { message: String },

    // 系统错误
    Error(String),
}
```

### 4.3 Intent（组件 → App）

```rust
pub enum Intent {
    // 本地 UI 操作
    SelectNext,
    SelectPrevious,
    SwitchFocus(Panel),
    ScrollMainView(i16),
    ToggleExpand(PathBuf),   // 用于树形视图

    // 触发后端命令
    RefreshStatus,
    GetDiff { file_path: String },
    StageFile { file_path: String },
    // ... 其他副作用命令

    None,
}
```

## 5. 核心数据结构

### 5.1 UiState（前端视角状态）

```rust
#[derive(Default)]
pub struct UiState {
    pub active_panel: Panel,
    pub files_panel: PanelState,
    pub branches_panel: PanelState,
    pub commits_panel: PanelState,
    pub stashes_panel: PanelState,
    pub main_view_scroll: u16,
    pub log_scroll: u16,
    pub modal: Option<ModalState>,
}

pub struct PanelState {
    pub list_state: ListState,   // 管理 selected 和 offset
    // 未来可加: filter_input, sort_order 等
}
```

### 5.2 CachedData（后端数据镜像）

```rust
#[derive(Default)]
pub struct CachedData {
    pub files: Vec<StatusEntry>,
    pub branches: Vec<BranchEntry>,
    pub commits: Vec<CommitEntry>,
    pub stashes: Vec<StashEntry>,
    pub current_diff: Option<(String, String)>, // (path, diff)
}
```

### 5.3 Component Trait

```rust
pub trait Component {
    fn handle_event(&mut self, event: &Event, ui_state: &UiState, data: &CachedData) -> Intent;
    fn render(&self, frame: &mut Frame, area: Rect, ui_state: &UiState, data: &CachedData, focused: bool);
}
```

## 6. 重构阶段（Gen2 全量重构）

### Phase 1：结构落位与协议切换（2-3 天）

**目标**：先完成模块边界重排和通信协议切换，建立 Gen2 骨架。

**具体任务**：
1. 落地新目录结构：`src/app/`、`src/backend/`、`src/components/`、`src/utils/`。
2. 将当前 `app.rs`、`backend.rs` 拆分到对应模块（先“搬迁+适配”，再“重构”）。
3. 定义并接入新的 `BackendCommand` / `FrontendEvent`（细粒度事件）。
4. 在前端落地 `UiState` 与 `CachedData`，替换 App 中散落字段。
5. 完成 `main.rs` 的模块路径与启动流程适配。
6. 完成一次端到端跑通：启动、刷新、导航、退出。

**验收标准**：
- 项目在新目录下 `cargo run` 可启动。
- 核心链路可用：`Refresh*`、`GetDiff`、`Quit`。
- 无 `StateUpdated(Box<...>)` 全量快照路径。

### Phase 2：Git 层拆分与渲染组件化（3-4 天）

**目标**：在 Gen2 结构内完成后端 Git 操作分层与前端渲染组件化。

**具体任务**：
1. 后端拆分：`commands.rs`、`events.rs`、`runtime.rs`、`git_ops/*`。
2. 将 Git 能力按领域拆到 `git_ops/status.rs`、`branches.rs`、`commits.rs`、`stash.rs`、`diff.rs`。
3. 前端实现原子组件：`SelectableList`、`ScrollableText`、`Tree`（可先占位）。
4. 前端实现面板组件：Files/Branches/Commits/Stash/MainView/Log。
5. `App::render` 改为布局 + 组件组合，移除旧 `render_*`。

**验收标准**：
- 面板显示与高亮行为与当前版本一致。
- 大仓库下无明显 clone 导致的卡顿。
- 后端逻辑模块边界清晰，可单独测试 `git_ops`。

### Phase 3：输入意图化与收口清理（2-3 天）

**目标**：完成 `Intent` 驱动输入流，收敛 App 到容器与调度职责。

**具体任务**：
1. 各组件实现 `handle_event`，只返回 `Intent`，不直接发命令。
2. `App::handle_input` 统一执行 `Intent`：
   - 本地 UI intent：修改 `UiState`
   - 后端命令 intent：发送 `BackendCommand`
3. 补充 `clamp_selection` 和 diff 请求关联策略（`request_id` 或“仅接受当前选中 path”）。
4. 清理旧输入分支与重复逻辑，补充关键单元测试。

**验收标准**：
- 高频导航保持本地即时响应。
- Diff 不出现乱序覆盖（旧请求覆盖新选中项）。
- `App` 明显瘦身，组件可独立测试。

## 7. 文件结构（最终状态）

```
src/
├── main.rs
├── app/
│   ├── mod.rs
│   ├── app.rs                 # App 结构体，事件循环
│   ├── ui_state.rs            # UiState, PanelState, Panel, ModalState
│   ├── cache.rs               # CachedData
│   └── intent.rs              # Intent 枚举
├── components/
│   ├── mod.rs                 # Component trait, Intent 导出
│   ├── core/
│   │   ├── mod.rs
│   │   ├── selectable_list.rs # 原子：可高亮列表
│   │   ├── tree.rs            # 原子：树形视图
│   │   └── scrollable_text.rs # 原子：可滚动文本
│   ├── panels/
│   │   ├── mod.rs
│   │   ├── file_list.rs
│   │   ├── branch_list.rs
│   │   ├── commit_list.rs
│   │   ├── stash_list.rs
│   │   ├── main_view.rs
│   │   └── log.rs
│   └── modals/
│       ├── mod.rs
│       ├── select_modal.rs    # 选择弹窗
│       ├── input_modal.rs     # 输入弹窗
│       └── confirm_modal.rs   # 确认弹窗
├── backend/
│   ├── mod.rs
│   ├── runtime.rs             # 后端主循环
│   ├── commands.rs            # BackendCommand
│   ├── events.rs              # FrontendEvent
│   └── git_ops/
│       ├── mod.rs
│       ├── repo.rs            # GitRepo 结构体
│       ├── status.rs
│       ├── branches.rs
│       ├── commits.rs
│       ├── stash.rs
│       └── diff.rs
└── utils/
    ├── mod.rs
    ├── debounce.rs            # 去抖动工具
    └── errors.rs
```

## 8. 未来扩展指南

### 8.1 新增数据面板（如 Remotes）

1. 后端增加 `RefreshRemotes` 命令与 `RemotesUpdated` 事件。
2. `CachedData` 添加 `remotes: Vec<RemoteEntry>`。
3. 创建 `RemoteListComponent`，复用 `SelectableList` 原子组件。
4. `Panel` 枚举增加 `Remotes`，`UiState` 增加 `remotes_panel: PanelState`。
5. 在 `App` 中实例化组件，加入焦点切换和布局。

### 8.2 新增树形视图（如 Files Tree）

1. `CachedData` 增加树形结构 `file_tree: TreeNode`。
2. 创建 `FileTreeComponent`，内部调用原子组件 `Tree`。
3. `UiState` 增加 `file_tree_expanded: HashSet<PathBuf>`。
4. 组件 `handle_event` 返回 `Intent::ToggleExpand(path)`，`App` 更新展开状态。

### 8.3 新增模态交互（如确认框、输入框）

1. `UiState` 中 `modal` 字段设为 `Some(ModalState::...)`。
2. 创建对应模态组件，实现 `Component`。
3. 在 `App::handle_input` 中优先检查 `modal`，若存在则调用模态组件的 `handle_event`。
4. 模态返回 `Intent::Confirm` 或 `Intent::Cancel`，`App` 执行相应动作并清除模态。

### 8.4 新增副作用命令（如 Fetch、Push）

1. 后端 `BackendCommand` 添加命令。
2. `git_ops` 中实现对应函数。
3. 后端处理命令，执行操作后**自动发送相关数据刷新事件**（如 `BranchesUpdated`），前端无需额外请求。
4. 前端在合适的组件中返回对应的 `Intent`，由 `App` 发送命令。

## 9. 验收清单

每个 Phase 完成后，必须通过以下检查：

| 检查项 | Phase 1 | Phase 2 | Phase 3 |
| :--- | :---: | :---: | :---: |
| `cargo fmt --check` | ✓ | ✓ | ✓ |
| `cargo check` | ✓ | ✓ | ✓ |
| `cargo test` | ✓ | ✓ | ✓ |
| `cargo clippy --all-targets --all-features -- -D warnings` | ✓ | ✓ | ✓ |
| 手动：Files 面板导航与 Diff | ✓ | ✓ | ✓ |
| 手动：Branches 面板切换分支 | ✓ | ✓ | ✓ |
| 手动：Commits 面板查看 | ✓ | ✓ | ✓ |
| 手动：Stash 面板操作 | ✓ | ✓ | ✓ |
| 手动：Tab/Shift+Tab 焦点切换 | ✓ | ✓ | ✓ |
| 手动：大量文件时无卡顿 | ✓ | ✓ | ✓ |
| 单元测试：组件 `handle_event` 返回正确 Intent | - | - | ✓ |

## 10. 风险与缓解

| 风险 | 缓解措施 |
| :--- | :--- |
| 状态迁移导致选中越界 | 每次数据更新后调用 `clamp_selection` |
| 渲染行为变化 | Phase 2 仅做等价迁移，不做样式调整 |
| 按键处理遗漏 | Phase 3 采用增量迁移，每迁移一类按键即手动回归 |
| Diff 回包乱序/高频请求 | 请求关联校验（`request_id` 或 path 校验）+ 可选 50ms 去抖 |

## 11. 总结

本计划遵循 Ratatui 官方组件架构模式，通过细粒度事件、意图驱动的单向数据流，实现了前后端职责的彻底分离。重构后的代码库将具备：

- **高内聚低耦合**：后端可独立测试，前端组件可复用。
- **清晰的扩展路径**：新增面板、模态、树形视图均有章可循。
- **优秀的性能**：高频 UI 操作不跨通道，数据更新无克隆开销。

现在即可按照 Phase 1 开始执行。
