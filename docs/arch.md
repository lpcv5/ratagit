# Ratagit 设计文档 v2

**日期**: 2026-03-24
**版本**: 2.0
**状态**: 设计完成

## 项目概述

Ratagit 是一个基于 ratatui 的 Git TUI 工具，类似 lazygit，提供直观的界面和强大的 Git 操作功能。

### 设计目标

- 类似 lazygit 的多面板布局和交互体验
- 支持标准 Git 操作和高级工作流
- 符合 Vim 用户习惯的键位设计
- 高性能异步架构，支持大型 monorepo
- 可配置和可扩展
- 稳定的架构设计，支持功能扩展而不需要架构变动

### 核心特性

**标准功能**：

- 查看仓库状态、提交历史、分支、标签
- 基本 Git 操作（add、commit、checkout、merge）
- 查看 diff 和交互式 patch 编辑
- Stash 管理
- Remote 操作（push、pull、fetch）

**高级功能**：

- 交互式 rebase（调整顺序、squash、fixup）
- 文件/patch 级别操作（move、drop、amend）
- Cherry-pick 跨分支操作
- Worktrees 管理
- Submodules 管理
- Reflog 查看
- 多选操作（Visual 模式）
- 动态面板布局
- 命令面板

## 架构设计原则

本设计遵循以下架构原则，确保后续功能扩展不需要修改核心架构：

1. **统一的组件接口**：所有交互组件实现统一的 trait，保证一致性
2. **分层的快捷键系统**：Global/Panel/Mode 三层快捷键，支持任意扩展
3. **可扩展的状态管理**：清晰的状态分离，易于测试和维护
4. **模块化的组件设计**：高复用性的通用组件（SelectList、FileTree、DiffViewer 等）
5. **性能优先**：针对大型 monorepo 的优化策略
6. **完整的日志系统**：不输出到终端，支持性能追踪和错误排查

## UI 设计

### 整体布局

```
┌──────────┬─────────────────────────┐
│ Files    │                         │
├──────────┤      Details            │
│ Branches │      (Diff/Content)     │
├──────────┤                         │
│ Commits  ├─────────────────────────┤
├──────────┤      Log                │
│ Stash    │      (Git operations)   │
└──────────┴─────────────────────────┘
│ ⠋ git status | space:stage | s:stash | ?:commands │
└────────────────────────────────────┘
```

**面板说明**：

- **左侧**：4 个主面板（Files、Branches、Commits、Stash）
- **右上**：Details 面板（显示详细信息）
- **右下**：Log 面板（显示 Git 操作日志）
- **底部**：Keybindings 提示（动态显示，包含加载指示器）

### 动态面板布局

**设计目标**：

- 面板高度根据聚焦状态和内容动态调整
- 未聚焦时使用默认高度
- 聚焦时，如果内容超过阈值，扩展到更大的固定比例

**左侧面板配置**：

| 面板     | 默认高度 | 聚焦高度 | 扩展阈值 | 说明                                |
| -------- | -------- | -------- | -------- | ----------------------------------- |
| Files    | 25%      | 40%      | 10 行    | 内容 > 10 行时扩展                  |
| Branches | 25%      | 25%      | -        | 不扩展                              |
| Commits  | 40%      | 50%      | 10 行    | 内容 > 10 行时扩展                  |
| Stash    | 1 行     | 按内容   | 1 行     | 未聚焦只显示 1 行，聚焦时按内容扩展 |

**右侧面板比例**：

- 左侧：30%
- 右侧：70%
  - Details：60%
  - Log：40%

## 核心架构

### 1. 统一组件系统

所有交互组件实现统一的 trait，确保一致性和可扩展性：

```rust
// 核心 Widget trait - 所有可交互组件的基础
pub trait InteractiveWidget {
    type State;
    type Action;

    fn render(&self, frame: &mut Frame, area: Rect, state: &Self::State);
    fn handle_event(&mut self, event: &Event, state: &mut Self::State) -> Option<Self::Action>;
    fn keybindings(&self, state: &Self::State) -> Vec<KeyBinding>;
}

// 支持多选的 Widget
pub trait SelectableWidget: InteractiveWidget {
    fn enter_visual_mode(&mut self);
    fn exit_visual_mode(&mut self);
    fn get_selection(&self) -> Vec<usize>;
    fn toggle_select(&mut self, index: usize);
}

// 支持动态高度的面板
pub trait DynamicPanel {
    fn default_height(&self) -> u16;
    fn focused_height(&self, content_lines: usize) -> u16;
    fn max_height(&self) -> u16;
    fn should_expand(&self, content_lines: usize) -> bool;
}
```

### 2. 分层快捷键系统

快捷键分为三层：Global（全局）、Panel（面板级）、Mode（模式级），按优先级查找。

```rust
pub enum KeyBindingScope {
    Global,           // 全局快捷键（如 q 退出，数字切换面板）
    Panel(PanelType), // 面板级快捷键（如 Files 的 s）
    Mode(Mode),       // 模式级快捷键（如 Visual 模式的 v）
}

pub struct KeyBindingManager {
    bindings: HashMap<KeyBindingScope, Vec<KeyBinding>>,
    current_scope: Vec<KeyBindingScope>, // 作用域栈
}
```

**查找优先级**：Mode > Panel > Global

**全局快捷键**：

- `q` - 退出应用
- `Esc` - 返回上一级（模式栈）
- `1-5` - 快速切换面板
- `h/l` - 左右切换面板
- `?` - 显示命令面板

**导航系统**：

- `Esc` 和 `q` 职责分离
  - `Esc`：返回上一级（退出模式、返回面板、关闭对话框）
  - `q`：直接退出应用
- 模式栈管理：Normal → Visual → PatchEdit，ESC 逐层返回

### 3. 状态管理

```rust
// 全局应用状态
pub struct AppState {
    pub git: GitState,
    pub ui: UiState,
    pub background_tasks: TaskManager,
}

// Git 状态（由 backend 更新）
pub struct GitState {
    pub status: Option<Status>,
    pub branches: Option<Branches>,
    pub commits: Option<Vec<Commit>>,
    pub stashes: Option<Vec<Stash>>,
}

// UI 状态（由 frontend 管理）
pub struct UiState {
    pub focused_panel: PanelType,
    pub panel_states: HashMap<PanelType, Box<dyn Any>>,
    pub layout: DynamicLayout,
    pub mode_stack: Vec<Mode>,
}

// 后台任务管理
pub struct TaskManager {
    pub running_tasks: Vec<Task>,
    pub task_results: mpsc::Receiver<TaskResult>,
}
```

### 4. 动态布局管理器

```rust
pub struct DynamicLayout {
    left_panels: Vec<PanelConfig>,
    focused_panel: PanelType,
    panel_heights: HashMap<PanelType, u16>,
}

pub struct PanelConfig {
    pub panel_type: PanelType,
    pub default_height_percent: u16,
    pub focused_height_percent: u16,
    pub expand_threshold: usize,
    pub min_height: u16,
}
```

## 通用组件设计

### 1. SelectList 组件

通用的选择列表组件，支持所有选择场景：

```rust
pub struct SelectList<T> {
    title: String,
    items: Vec<SelectItem<T>>,
    selected: usize,
    filter: String,
    show_filter: bool,
}

pub struct SelectItem<T> {
    label: String,
    description: Option<String>,
    value: T,
    enabled: bool,
}
```

**使用场景**：

- Reset 模式选择（hard/soft/mixed）
- 删除分支选项（本地/远程/两者）
- 命令面板（? 键）
- Cherry-pick 目标选择

**操作**：

- `j/k` - 上下移动
- `Enter` - 确认选择
- `Esc` - 取消
- `/` - 进入过滤模式
- 过滤模式下输入文本进行模糊匹配

### 2. 多选支持（Visual Mode）

```rust
pub struct VisualMode {
    anchor: usize,
    cursor: usize,
    selections: HashSet<usize>,
}

pub trait MultiSelectable {
    fn enter_visual_mode(&mut self, anchor: usize);
    fn exit_visual_mode(&mut self);
    fn toggle_selection(&mut self, index: usize);
    fn get_selections(&self) -> Vec<usize>;
    fn is_in_visual_mode(&self) -> bool;
}
```

**支持多选的组件**：

- FileTree（Files 面板）
- CommitList（Commits 面板）

**操作**：

- `v` - 进入/退出 Visual 模式
- `Space` - 切换当前项选择状态
- `j/k` - 移动光标（扩展选择范围）
- 操作键（如 `s`、`d`）应用到所有选中项

### 3. LoadingIndicator 组件

```rust
pub struct LoadingIndicator {
    frames: Vec<&'static str>,
    current_frame: usize,
    message: String,
}

impl LoadingIndicator {
    const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}
```

**显示位置**：Keybindings 区域最前面

**显示时机**：后台任务运行时（git 操作、刷新等）

### 4. CommandPalette 组件

```rust
pub struct CommandPalette {
    all_commands: Vec<Command>,
    filtered_commands: Vec<Command>,
    select_list: SelectList<Command>,
}

pub struct Command {
    pub key: String,
    pub name: String,
    pub description: String,
    pub scope: KeyBindingScope,
    pub action: Action,
}
```

**功能**：

- `?` 键弹出
- 显示所有可用命令（包括基础命令）
- 支持 `/` 过滤
- 可以查看也可以选择执行

## 面板详细设计

### 1. Files 面板（原 Status 面板）

**Tab 结构**：

- Tab 1: Files（文件列表）
- Tab 2: Submodules（子模块）
- Tab 3: Worktrees（工作树）

**Files Tab 显示内容**：

- Unstaged Changes（未暂存，红色）
- Staged Changes（已暂存，绿色）
- Untracked Files（未跟踪，灰色）
- 文件状态标记：M（修改）、A（新增）、D（删除）、?（未跟踪）

**快捷键**：

- `Space` - stage/unstage 文件
- `a` - stage all
- `Enter` - 进入 diff/patch 编辑模式
- `c` - commit（弹出输入框）
- `d` - discard changes（弹出确认框）
- `s` - stash 当前文件/选中文件（新增）
- `R` - reset（弹出 SelectList 选择模式）（新增）
- `v` - 进入 Visual 模式（多选）（新增）
- `i` - 添加到 .gitignore
- `D` - 删除未跟踪文件

**Visual 模式操作**：

- `v` - 退出 Visual 模式
- `Space` - 切换选择
- `s` - stash 选中的文件
- `d` - discard 选中的文件

**Reset 对话框选项**：

- Soft（保留工作区和暂存区）
- Mixed（保留工作区，清空暂存区）
- Hard（清空工作区和暂存区）

### 2. Branches 面板

**Tab 结构**：

- Tab 1: Local（本地分支）
- Tab 2: Remote（远程分支）
- Tab 3: Tags（标签）

**Local Tab 显示内容**：

- 本地分支列表
- 当前分支高亮显示（\*）
- 显示最后提交信息

**快捷键**：

- `Enter` - 进入二级页面查看该分支的 commits（新增）
- `n` - 新建分支
- `d` - 删除分支（弹出 SelectList 选择：本地/远程/两者）（修改）
- `D` - 强制删除分支
- `r` - rename 分支
- `m` - merge 分支到当前分支
- `M` - merge with --no-ff
- `Space` - 查看分支详情（右侧显示）

**二级页面（Branch Commits View）**：

- 显示该分支的 commits（复用 CommitList 组件）
- `Enter` - 展开文件树
- `C` - 标记 commit 用于 cherry-pick
- `l` - 跳转到 Commits 面板
- `V` - 在 Commits 面板粘贴选择的 commits
- `Esc` - 返回分支列表

**删除分支对话框选项**：

- 删除本地分支
- 删除远程分支
- 删除本地和远程分支

### 3. Commits 面板

**Tab 结构**：

- Tab 1: Current Branch（当前分支提交）
- Tab 2: Reflog（引用日志）

**显示内容**（四项）：

1. **Graph 节点**：使用 Unicode 字符绘制（默认开启）
   - `│` 直线
   - `├` 分叉
   - `─` 横线
   - `╮` `╯` `╰` `╭` 转角
   - `┼` 十字（交叉）（新增）

2. **Hash**（7 位，带颜色）：
   - 绿色：commit 存在于 origin/main
   - 黄色：commit 已推送到 origin/当前分支，但不在 origin/main
   - 红色：commit 只在本地，还没推送

3. **作者名**（两字母缩写，不同作者不同颜色）

4. **Commit message**（白色）

**快捷键**：

- `Enter` - 查看 commit 详情（右侧显示 diff），可再次回车进入 patch 编辑（新增）
- `Space` - 展开显示文件列表
- `v` - 进入 Visual 模式（多选）（新增）
- `C` - copy commit hash
- `V` - 粘贴 cherry-pick 的 commits（新增）
- `c` - cherry-pick
- `r` - revert commit
- `R` - reset to commit（弹出选择：soft/mixed/hard）
- `e` - edit commit message（reword）

**Visual 模式操作**（新增）：

- `v` - 退出 Visual 模式
- `J` - 向下移动选中的 commits
- `K` - 向上移动选中的 commits
- `s` - squash 选中的 commits
- `d` - drop 选中的 commits

**文件列表模式**（按 Space 展开）：

- 显示该 commit 的文件变更
- `Enter` - 查看文件 diff，可进入 patch 编辑
- `m` - move file/patch 到其他 commit
- `d` - drop file/patch
- `a` - amend file/patch 到指定 commit

### 4. Stash 面板

**显示内容**：

- Stash 列表（stash@{0}, stash@{1}, ...）
- 显示 stash message 和时间

**动态高度**：

- 未聚焦：1 行
- 聚焦：如果内容 > 1 行，按内容扩展

**快捷键**：

- `Enter` - 查看 stash 内容（右侧显示 diff）
- `Space` - apply stash
- `p` - pop stash
- `d` - drop stash（弹出确认框）
- `b` - 从 stash 创建分支
- **移除** `n` - 新建 stash（改为在 Files 面板使用 `s`）

### 5. Details 面板（右上）

**动态内容**（根据聚焦面板显示）：

| 聚焦面板 | Details 显示内容 | 说明                                                       |
| -------- | ---------------- | ---------------------------------------------------------- |
| Files    | 文件 diff        | 回车进入 patch 编辑模式                                    |
| Branches | 分支对比 graph   | 显示选中分支与当前分支的 `git log --graph`（保持原生颜色） |
| Commits  | Commit diff      | 显示该 commit 的所有 diff，回车可进入 patch 编辑           |
| Stash    | Stash diff       | 显示 stash 的 diff                                         |

**操作**：

- `j/k` - 滚动
- `/` - 搜索
- `n/N` - 下一个/上一个搜索结果
- `Enter` - 进入 patch 编辑模式（适用于 diff 内容）

### 6. Log 面板（右下）

**显示内容**：

- **只显示 Git 相关操作及其结果**（修改）
- Git 命令执行记录
- Git 命令输出
- 操作日志（info/success/error）
- 颜色区分不同级别

**日志级别**：

- GitCommand（青色）：执行的 Git 命令
- GitOutput（灰色）：Git 命令输出
- Success（绿色）：操作成功
- Error（红色）：错误信息
- Info（白色）：一般信息

**特性**：

- 自动滚动到最新
- 可手动滚动查看历史
- **移除清空功能**（不支持 `C` 清空）

### 7. Keybindings 提示（底部）

**动态显示**：

- 根据当前焦点面板和模式显示对应的快捷键
- **不显示基础快捷键**（jkhl、Enter 等）
- **最前面显示加载指示器**（后台任务运行时）
- **最后添加 `?` 键**（弹出命令面板）

**格式**：

```
⠋ git status | space:stage | s:stash | R:reset | ?:commands
```

## DiffViewer 与 Patch 编辑

### DiffViewer 组件

```rust
pub struct DiffViewer {
    diff: Diff,
    mode: DiffViewMode,
    hunks: Vec<Hunk>,
    selected_hunks: HashSet<usize>,
    current_hunk: usize,
}

pub enum DiffViewMode {
    View,           // 只读查看
    PatchEdit,      // Patch 编辑模式
}
```

### Patch 编辑模式

**设计原则**：

- **不支持字符级编辑**（修改）
- **只支持整行或 hunk 选择**
- 通过选择/取消选择来决定哪些变更要应用

**操作**：

- `Enter` - 从 View 模式进入 PatchEdit 模式
- `Space` - 选择/取消选择当前 hunk
- `j/k` - 移动到下一个/上一个 hunk
- `v` - 进入行选择模式（在 hunk 内选择特定行）
- `a` - 全选所有 hunks
- `s` - stage 选中的 hunks
- `d` - drop 选中的 hunks
- `Esc` - 退出 PatchEdit 模式

**渲染**：

- 选中的 hunk 高亮显示（蓝色背景）
- `+` 行显示为绿色
- `-` 行显示为红色
- 上下文行显示为默认颜色

## 性能优化架构

### 1. 大型 Monorepo 支持

**仓库大小检测**：

```rust
pub enum RepoSize {
    Small,   // < 1000 文件
    Medium,  // 1000-10000 文件
    Large,   // > 10000 文件
}
```

**刷新策略**：

```rust
pub enum RefreshStrategy {
    Auto(FileWatcher),      // 小仓库：文件系统监听
    Manual,                 // 大仓库：手动刷新（r 键）
    Hybrid(HybridConfig),   // 混合模式（推荐）
}

pub struct HybridConfig {
    file_watcher: FileWatcher,
    debounce_ms: u64,               // 防抖间隔（500ms）
    min_refresh_interval: Duration, // 最小刷新间隔（1000ms）
    pause_on_git_op: bool,          // Git 操作时暂停刷新
}
```

**策略选择**：

- 小仓库（< 1000 文件）：使用文件监听自动刷新
- 中等仓库（1000-10000 文件）：混合模式（文件监听 + 防抖 + 暂停）
- 大仓库（> 10000 文件）：手动刷新模式

### 2. Git 操作锁管理

```rust
pub struct GitLockManager {
    active_operations: Arc<RwLock<HashSet<String>>>,
}
```

**功能**：

- 防止并发 Git 操作导致的锁冲突
- 显示加载指示器让用户知道程序没有崩溃
- 操作完成后自动释放锁

**加载指示器显示**：

- 在 Keybindings 区域最前面显示
- 使用 Unicode 动画（⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏）
- 显示操作名称（如 "git status", "git commit"）

### 3. 虚拟滚动

```rust
pub struct VirtualList<T> {
    items: Vec<T>,
    visible_range: Range<usize>,
    viewport_height: usize,
    scroll_offset: usize,
}
```

**功能**：

- 只渲染可见区域的项
- 支持大列表（10000+ 项）的流畅滚动
- 预加载上下几行避免滚动卡顿

**应用场景**：

- Commits 列表
- Files 列表
- Branches 列表

## 日志系统

### 应用级日志

**设计原则**：

- **所有日志输出到文件，不输出到终端**（TUI 应用特性）
- 支持性能追踪和错误排查
- 关键操作埋点

**日志文件位置**：

- Linux/macOS: `~/.local/share/ratagit/logs/ratagit.log`
- Windows: `%APPDATA%\ratagit\logs\ratagit.log`

**日志级别**：

- `trace` - 详细的调试信息
- `debug` - 调试信息（包括性能埋点）
- `info` - 一般信息
- `warn` - 警告
- `error` - 错误

**实现**：

```rust
pub struct AppLogger {
    file_appender: FileAppender,
    performance_tracker: PerformanceTracker,
}

// 使用 tracing 框架
use tracing::{info, debug, error, instrument};

#[instrument(skip(backend))]
async fn git_status(backend: &GitBackend) -> Result<Status> {
    info!("Starting git status");
    let result = backend.status().await;
    info!("Git status completed");
    result
}
```

### 性能追踪

**功能**：

- 记录每个操作的耗时
- 生成性能报告
- 用于性能分析和优化

**埋点位置**：

- Git 操作（status、diff、log、commit 等）
- UI 渲染
- 文件系统监听
- 后台任务执行

**实现**：

```rust
pub struct PerformanceTracker {
    spans: HashMap<String, Vec<Duration>>,
}

impl PerformanceTracker {
    #[instrument(skip(self))]
    pub fn track<F, T>(&mut self, name: &str, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();

        self.spans.entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(duration);

        debug!(
            operation = name,
            duration_ms = duration.as_millis(),
            "Operation completed"
        );

        result
    }
}
```

## Git 操作实现

### 混合方式架构

**设计决策**：使用混合模式（libgit2 + Git 命令），不需要单独配置。

**抽象层**：

```rust
pub trait GitBackend: Send + Sync {
    async fn status(&self) -> Result<Status>;
    async fn diff(&self, path: &Path) -> Result<Diff>;
    async fn commit(&self, message: &str) -> Result<Oid>;
    async fn branches(&self) -> Result<Vec<Branch>>;
    async fn log(&self, branch: &str) -> Result<Vec<Commit>>;
    // ... 更多操作
}
```

**实现方式**：

```rust
pub struct HybridGitBackend {
    libgit2: LibGit2Backend,
    command: GitCommandBackend,
    repo_path: PathBuf,
}
```

**操作分配**：

| 操作类型    | 使用方式 | 原因     |
| ----------- | -------- | -------- |
| status      | libgit2  | 性能优先 |
| diff        | libgit2  | 性能优先 |
| log         | libgit2  | 性能优先 |
| branches    | libgit2  | 性能优先 |
| tags        | libgit2  | 性能优先 |
| stash list  | libgit2  | 性能优先 |
| rebase      | Git 命令 | 功能完整 |
| cherry-pick | Git 命令 | 功能完整 |
| worktree    | Git 命令 | 功能完整 |
| submodule   | Git 命令 | 功能完整 |
| reflog      | Git 命令 | 功能完整 |

## 测试策略

### 测试原则

**使用临时目录**：

- 所有测试使用临时文件夹（`std::env::temp_dir()`）
- 避免污染本地仓库
- 测试结束后自动清理

### 1. 单元测试

**测试范围**：

- 组件的 render 和 handle_event
- Git 操作的正确性
- 配置加载和验证
- 状态管理逻辑

**示例**：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    fn create_test_repo() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        // 初始化 git 仓库
        std::process::Command::new("git")
            .args(&["init"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();
        temp_dir
    }

    #[test]
    fn test_file_tree_toggle_expand() {
        let mut tree = FileTree::new();
        tree.add_file("src/main.rs");
        tree.add_file("src/lib.rs");

        assert!(!tree.is_expanded("src"));
        tree.toggle_expand("src");
        assert!(tree.is_expanded("src"));
    }

    #[test]
    fn test_visual_mode_selection() {
        let mut tree = FileTree::new();
        tree.add_file("file1.rs");
        tree.add_file("file2.rs");
        tree.add_file("file3.rs");

        tree.enter_visual_mode(0);
        tree.toggle_selection(1);
        tree.toggle_selection(2);

        assert_eq!(tree.get_selections(), vec![0, 1, 2]);
    }

    #[test]
    fn test_commit_color_calculation() {
        let temp_repo = create_test_repo();
        let panel = CommitsPanel::new(temp_repo.path());

        // 测试三层颜色逻辑
        let commit_in_main = create_test_commit(&temp_repo, "abc123");
        assert_eq!(
            panel.calculate_commit_color(&commit_in_main),
            CommitColor::Green
        );
    }
}
```

### 2. 集成测试

**测试范围**：

- 完整的用户交互流程
- Git 操作的端到端测试
- 错误处理和恢复

**示例**：

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_stage_and_commit_flow() {
        let temp_repo = create_test_repo();
        let mut app = App::new(temp_repo.path()).await.unwrap();

        // 1. 创建文件
        create_file(&temp_repo, "test.txt", "content");

        // 2. 刷新状态
        app.refresh().await.unwrap();

        // 3. Stage 文件
        app.handle_action(Action::ToggleStage).await.unwrap();

        // 4. Commit
        app.handle_action(Action::Commit).await.unwrap();

        // 5. 验证
        let status = app.git_backend.status().await.unwrap();
        assert!(status.is_clean());
    }

    #[tokio::test]
    async fn test_multi_select_and_stash() {
        let temp_repo = create_test_repo();
        let mut app = App::new(temp_repo.path()).await.unwrap();

        // 创建多个文件
        create_file(&temp_repo, "file1.txt", "content1");
        create_file(&temp_repo, "file2.txt", "content2");

        app.refresh().await.unwrap();

        // 进入 Visual 模式
        app.handle_key(Key('v')).await.unwrap();

        // 选择多个文件
        app.handle_key(Key(' ')).await.unwrap();
        app.handle_key(Key('j')).await.unwrap();
        app.handle_key(Key(' ')).await.unwrap();

        // Stash 选中的文件
        app.handle_key(Key('s')).await.unwrap();

        // 验证
        let stashes = app.git_backend.stash_list().await.unwrap();
        assert_eq!(stashes.len(), 1);
    }
}
```

### 3. 性能测试

**测试范围**：

- 大仓库的性能
- 内存使用
- 渲染帧率

**示例**：

```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    use criterion::{black_box, criterion_group, criterion_main, Criterion};
    use tempfile::TempDir;

    fn bench_git_status(c: &mut Criterion) {
        let temp_repo = create_large_test_repo(10000); // 10000 文件

        c.bench_function("git_status_large_repo", |b| {
            b.iter(|| {
                let backend = GitBackend::new(temp_repo.path()).unwrap();
                black_box(backend.status())
            });
        });
    }

    fn bench_virtual_list_render(c: &mut Criterion) {
        let items: Vec<_> = (0..100000).collect();
        let mut list = VirtualList::new(items, 50);

        c.bench_function("virtual_list_render", |b| {
            b.iter(|| {
                black_box(list.render())
            });
        });
    }

    criterion_group!(benches, bench_git_status, bench_virtual_list_render);
    criterion_main!(benches);
}
```

### 测试辅助函数

```rust
// 创建测试仓库
fn create_test_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    std::process::Command::new("git")
        .args(&["init"])
        .current_dir(temp_dir.path())
        .output()
        .unwrap();
    temp_dir
}

// 创建大型测试仓库
fn create_large_test_repo(file_count: usize) -> TempDir {
    let temp_dir = create_test_repo();
    for i in 0..file_count {
        create_file(&temp_dir, &format!("file{}.txt", i), "content");
    }
    temp_dir
}

// 创建文件
fn create_file(repo: &TempDir, path: &str, content: &str) {
    let file_path = repo.path().join(path);
    std::fs::write(file_path, content).unwrap();
}
```

## 配置文件设计

### 配置文件位置

- Linux/macOS: `~/.config/ratagit/config.toml`
- Windows: `%APPDATA%\ratagit\config.toml`

### 配置结构

```toml
[ui]
fps = 60
panel_ratio = [30, 70]  # 左右面板比例
details_log_ratio = [60, 40]  # Details 和 Log 比例

[ui.dynamic_layout]
# 动态布局配置
status_default_height = 25
status_focused_height = 40
status_expand_threshold = 10

commits_default_height = 40
commits_focused_height = 50
commits_expand_threshold = 10

stash_default_height = 1
stash_min_height = 1

[theme]
name = "default"  # 或 "dracula", "gruvbox", "nord"

# 自定义颜色
[theme.colors]
primary = "blue"
secondary = "cyan"
error = "red"
success = "green"
warning = "yellow"
info = "white"

[keymap]
# 全局快捷键
quit = "q"
escape = "Esc"
refresh = "r"
command_palette = "?"

# Files 面板
[keymap.files]
stage = "space"
stage_all = "a"
commit = "c"
discard = "d"
stash = "s"
reset = "R"
visual_mode = "v"

# Branches 面板
[keymap.branches]
delete = "d"
cherry_pick = "C"

# Commits 面板
[keymap.commits]
copy_hash = "C"
paste_cherry_picks = "V"
move_down = "J"
move_up = "K"
visual_mode = "v"

[features]
enable_submodules = true
enable_worktrees = true
enable_commit_graph = true
enable_reflog = true

[git]
backend = "hybrid"  # 固定使用混合模式
default_branch = "main"
auto_fetch = false
fetch_interval = 300  # 秒

[performance]
# 性能配置
repo_size_detection = "auto"  # auto | small | medium | large
refresh_strategy = "hybrid"   # auto | manual | hybrid
debounce_ms = 500
min_refresh_interval_ms = 1000
virtual_scroll_enabled = true
virtual_scroll_buffer = 10    # 预加载行数

[performance.thresholds]
small_repo_files = 1000
large_repo_files = 10000

[logging]
# 日志配置
enabled = true
level = "info"  # trace | debug | info | warn | error
file = "~/.local/share/ratagit/logs/ratagit.log"
max_size_mb = 10
max_files = 5
performance_tracking = true
```

## 技术架构

### 目录结构

```
src/
├── main.rs
├── app/
│   ├── mod.rs
│   ├── state.rs
│   ├── config.rs
│   └── channels.rs
│
├── frontend/
│   ├── mod.rs
│   ├── ui_loop.rs
│   ├── event_handler.rs
│   ├── panels/
│   │   ├── mod.rs
│   │   ├── files.rs
│   │   ├── branches.rs
│   │   ├── commits.rs
│   │   ├── stash.rs
│   │   ├── details.rs
│   │   └── log.rs
│   ├── components/
│   │   ├── mod.rs
│   │   ├── file_tree.rs
│   │   ├── commit_graph.rs
│   │   ├── diff_viewer.rs
│   │   ├── select_list.rs
│   │   ├── loading_indicator.rs
│   │   ├── command_palette.rs
│   │   └── dialogs/
│   │       ├── mod.rs
│   │       ├── confirm.rs
│   │       ├── input.rs
│   │       └── message.rs
│   ├── stores/
│   │   ├── mod.rs
│   │   ├── git_store.rs
│   │   └── ui_store.rs
│   └── layout/
│       ├── mod.rs
│       └── dynamic_layout.rs
│
├── backend/
│   ├── mod.rs
│   ├── runtime.rs
│   ├── executor/
│   │   ├── mod.rs
│   │   ├── executor.rs
│   │   ├── worker_pool.rs
│   │   ├── task_queue.rs
│   │   └── task.rs
│   ├── git/
│   │   ├── mod.rs
│   │   ├── backend.rs
│   │   ├── libgit2_backend.rs
│   │   ├── command_backend.rs
│   │   ├── hybrid_backend.rs
│   │   └── lock_manager.rs
│   ├── tasks/
│   │   ├── mod.rs
│   │   ├── status_task.rs
│   │   ├── diff_task.rs
│   │   ├── commit_task.rs
│   │   └── ...
│   └── performance/
│       ├── mod.rs
│       ├── performance_manager.rs
│       ├── file_watcher.rs
│       └── virtual_list.rs
│
└── shared/
    ├── mod.rs
    ├── command.rs
    ├── update.rs
    ├── error.rs
    ├── types.rs
    ├── keybindings.rs
    └── logger.rs
```

### 依赖项

```toml
[dependencies]
# TUI
ratatui = "0.30"
crossterm = "0.28"

# 异步运行时
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# Git
git2 = "0.19"  # libgit2 绑定

# 错误处理
anyhow = "1"
thiserror = "1"

# 配置
serde = { version = "1", features = ["derive"] }
toml = "0.8"
dirs = "5"

# 日志
tracing = "0.1"
tracing-subscriber = "0.3"
tracing-appender = "0.2"

# 文件监听
notify = "6"

# 其他
chrono = "0.4"  # 时间处理
unicode-width = "0.1"  # Unicode 宽度计算

[dev-dependencies]
tempfile = "3"  # 测试用临时目录
criterion = "0.5"  # 性能测试
```

## 开发路线

### Phase 1: 核心架构（2-3 周）

- 实现核心组件系统（InteractiveWidget、SelectableWidget、DynamicPanel）
- 实现分层快捷键系统
- 实现状态管理
- 实现动态布局管理器
- 实现基础 Git 后端（混合模式）

### Phase 2: 基础面板（3-4 周）

- Files 面板（基础功能）
- Branches 面板（基础功能）
- Commits 面板（基础功能）
- Stash 面板
- Details 面板
- Log 面板

### Phase 3: 通用组件（2 周）

- SelectList 组件
- LoadingIndicator 组件
- CommandPalette 组件
- DiffViewer 组件
- 各种 Dialog 组件

### Phase 4: 高级功能（3-4 周）

- Visual 模式（多选）
- Patch 编辑
- Cherry-pick 跨分支操作
- Branch 二级页面
- Commit 移动和操作

### Phase 5: 性能优化（2 周）

- 大型 monorepo 支持
- 虚拟滚动
- Git 操作锁管理
- 性能追踪和优化

### Phase 6: 测试和文档（2 周）

- 单元测试
- 集成测试
- 性能测试
- 用户文档

## 总结

Ratagit v2 提供了一个功能完整、性能优良、架构稳定的 Git TUI 工具：

**核心优势**：

- **稳定的架构**：前期设计好核心架构，后续功能扩展不需要修改架构
- **统一的组件系统**：所有组件遵循统一接口，易于扩展和维护
- **分层的快捷键系统**：Global/Panel/Mode 三层，支持任意扩展
- **强大的多选支持**：Visual 模式支持批量操作
- **动态面板布局**：根据聚焦状态和内容自动调整高度
- **大型 monorepo 支持**：混合刷新策略、虚拟滚动、Git 锁管理
- **完整的日志系统**：文件日志、性能追踪、关键操作埋点
- **全面的测试策略**：使用临时目录，避免污染本地仓库

**与 v1 的主要改进**：

1. 面板动态高度调整
2. 后台任务加载指示器
3. ESC/q 职责分离
4. Details 面板根据聚焦显示不同内容
5. Branch 二级页面查看 commits
6. Reset 操作支持
7. 快捷键分 local/global
8. SelectList 通用组件
9. Commits 面板多选和移动
10. Files 面板多选支持
11. Stash 操作移到 Files 面板
12. Log 面板只显示 Git 相关操作
13. Keybindings 动态显示和命令面板
14. Commit graph 添加十字字符
15. Patch 编辑整行/hunk 选择
16. 大型 monorepo 性能优化
17. Git 操作混合模式
18. 完整的日志系统
19. 使用临时目录的测试策略

**架构保证**：

- 所有新功能都有明确的架构位置
- 组件间通过统一接口通信
- 状态管理清晰分离
- 扩展点设计合理
- 后续添加功能不需要修改核心架构
