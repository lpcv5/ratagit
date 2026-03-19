# Ratagit Architecture Design

> A Rust-based terminal UI for Git operations, inspired by lazygit

## 项目概述

Ratagit 是一个使用 Rust 编写的终端 Git 客户端，基于 ratatui 构建。目标是提供一个快速、直观、功能完整的 Git TUI 工具。

### 核心目标

- **性能优先**: 利用 Rust 的性能优势，实现毫秒级响应
- **类型安全**: 通过类型系统保证 Git 操作的安全性
- **可扩展**: 模块化设计，便于添加新功能
- **用户友好**: 直观的界面和快捷键设计

## 技术栈

### 核心依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| `ratatui` | 0.30+ | 终端UI框架 |
| `crossterm` | 0.29+ | 跨平台终端控制 |
| `git2` | 0.19+ | Git操作库（初期） |
| `gix` | 最新 | Git操作库（长期替代git2） |
| `tokio` | 1.40+ | 异步运行时 |
| `thiserror` | 1.0+ | 错误处理 |
| `serde` | 1.0+ | 配置序列化 |

### 可选依赖

| 依赖 | 用途 |
|------|------|
| `arboard` | 剪贴板支持 |
| `regex` | 搜索过滤 |
| `chrono` | 时间格式化 |
| `dirs` | 配置目录 |

## 核心架构：TEA + Component 混合架构

Ratagit 采用 **TEA (The Elm Architecture) + Component 混合架构**，结合了两者的优势。

### 架构选择理由

1. **Git 状态天然是全局的**：status、commits、branches 在多个视图间共享
2. **异步 Git 操作需要 Command 模式**：耗时操作（fetch、push）需要异步执行
3. **调试 Git 操作需要时间旅行**：消息流可记录和回放
4. **组件封装提高代码组织性**：View trait 封装渲染和事件处理逻辑

### 混合架构设计

**主干：The Elm Architecture (TEA)**
- 全局状态：`App` 结构体管理所有状态
- 消息驱动：`Message` 枚举定义所有事件
- 纯函数更新：`update(app, msg) -> Option<Command>`
- 异步支持：`Command` 枚举处理异步操作

**辅助：Component 思想**
- `View` trait 封装渲染逻辑
- `handle_key()` 处理局部事件
- 组件级别的代码组织

### 架构图

```
┌─────────────────────────────────────────────────────────┐
│                     Event Loop                          │
│  ┌──────────────┐                                       │
│  │   Terminal   │                                       │
│  │    Input     │                                       │
│  └──────┬───────┘                                       │
│         │                                               │
│         ▼                                               │
│  ┌──────────────┐      ┌─────────────┐                 │
│  │   handle_    │─────▶│   Message   │                 │
│  │    event     │      │   (Event)   │                 │
│  └──────────────┘      └──────┬──────┘                 │
│                               │                         │
│                               ▼                         │
│                        ┌─────────────┐                  │
│                        │   update    │                  │
│                        │  (Message)  │                  │
│                        └──────┬──────┘                  │
│                               │                         │
│                               ▼                         │
│                        ┌─────────────┐                  │
│                        │    Model    │◀─────┐          │
│                        │   (State)   │      │          │
│                        └──────┬──────┘      │          │
│                               │             │          │
│                               ▼             │          │
│                        ┌─────────────┐      │          │
│                        │    view     │      │          │
│                        │   (Render)  │──────┘          │
│                        └─────────────┘                  │
└─────────────────────────────────────────────────────────┘
```

### 三大核心组件

#### 1. Model (状态)

```rust
struct App {
    // 应用状态
    running: bool,
    current_tab: Tab,

    // Git 状态
    repo: Repository,
    status: GitStatus,
    commits: Vec<Commit>,
    branches: Vec<Branch>,

    // UI 状态
    selected_index: usize,
    scroll_offset: usize,
    input_mode: Option<InputMode>,

    // 配置
    config: Config,
}

enum Tab {
    Status,
    Commits,
    Branches,
    Stash,
    Remotes,
}

enum InputMode {
    Normal,
    Insert,
    Search,
}
```

#### 2. Message (消息)

```rust
enum Message {
    // 导航
    TabNext,
    TabPrev,
    ScrollUp,
    ScrollDown,

    // Git 操作
    StageFile(PathBuf),
    UnstageFile(PathBuf),
    Commit(String),
    BranchCreate(String),
    BranchCheckout(String),

    // UI 操作
    SearchStart,
    SearchInput(String),
    PopupShow(PopupType),
    PopupClose,

    // 系统
    Quit,
    Refresh,
    Error(Error),
}
```

#### 3. Update (更新)

```rust
fn update(app: &mut App, msg: Message) -> Command {
    match msg {
        Message::StageFile(path) => {
            app.stage_file(&path)?;
            Command::Refresh
        }
        Message::Commit(message) => {
            app.commit(&message)?;
            Command::Refresh
        }
        // ...
    }
}
```

## 模块划分

### 目录结构

```
ratagit/
├── Cargo.toml
├── README.md
├── ARCHITECTURE.md
├── src/
│   ├── main.rs                    # 入口点
│   │
│   ├── app/                       # 应用核心
│   │   ├── mod.rs
│   │   ├── app.rs                 # App 结构体和主循环
│   │   ├── state.rs               # 应用状态定义
│   │   ├── message.rs             # 消息/事件定义
│   │   └── command.rs             # 命令定义
│   │
│   ├── git/                       # Git 操作层
│   │   ├── mod.rs
│   │   ├── repository.rs          # 仓库抽象
│   │   ├── status.rs              # 状态查询
│   │   ├── commit.rs              # 提交操作
│   │   ├── branch.rs              # 分支操作
│   │   ├── stash.rs               # Stash 操作
│   │   ├── rebase.rs              # Rebase 操作
│   │   ├── diff.rs                # Diff 操作
│   │   └── error.rs               # Git 错误
│   │
│   ├── ui/                        # UI 层
│   │   ├── mod.rs
│   │   ├── theme.rs               # 主题配置
│   │   ├── layout.rs              # 布局定义
│   │   │
│   │   ├── views/                 # 视图组件
│   │   │   ├── mod.rs
│   │   │   ├── status.rs          # Status 视图
│   │   │   ├── commits.rs         # Commits 视图
│   │   │   ├── branches.rs        # Branches 视图
│   │   │   ├── stash.rs           # Stash 视图
│   │   │   └── diff.rs            # Diff 视图
│   │   │
│   │   ├── widgets/               # 可复用组件
│   │   │   ├── mod.rs
│   │   │   ├── tabs.rs            # Tab 栏
│   │   │   ├── list.rs            # 列表组件
│   │   │   ├── popup.rs           # 弹窗
│   │   │   ├── input.rs           # 输入框
│   │   │   └── status_bar.rs      # 状态栏
│   │   │
│   │   └── renderer.rs            # 渲染器
│   │
│   ├── event/                     # 事件处理
│   │   ├── mod.rs
│   │   ├── handler.rs             # 事件处理器
│   │   └── keybind.rs             # 快捷键映射
│   │
│   ├── config/                    # 配置系统
│   │   ├── mod.rs
│   │   ├── config.rs              # 配置结构
│   │   └── loader.rs              # 配置加载
│   │
│   └── utils/                     # 工具函数
│       ├── mod.rs
│       └── logger.rs              # 日志系统
│
└── tests/                         # 集成测试
    ├── git_operations_test.rs
    └── ui_test.rs
```

## 数据流设计

### 单向数据流

```
User Input
    │
    ▼
Event Handler
    │
    ▼
Message
    │
    ▼
Update Function
    │
    ├─▶ Update Model
    │
    └─▶ Side Effects (Git Operations)
           │
           ▼
        New Message
           │
           ▼
        Update Again
           │
           ▼
        View Render
```

### Git 操作流程

```
┌─────────────┐
│  UI Action  │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Message   │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Git Module  │◀─────┐
└──────┬──────┘      │
       │             │
       ▼             │
┌─────────────┐      │
│   git2/gix  │      │
│   Library   │      │
└──────┬──────┘      │
       │             │
       ▼             │
┌─────────────┐      │
│   Result    │──────┘
└─────────────┘
```

## Git 操作抽象层

### Repository Trait

为了支持从 git2 迁移到 gix，我们定义抽象接口：

```rust
pub trait GitRepository {
    fn open(path: &Path) -> Result<Self, GitError>
    where
        Self: Sized;

    fn status(&self) -> Result<GitStatus, GitError>;
    fn commits(&self, limit: usize) -> Result<Vec<Commit>, GitError>;
    fn branches(&self) -> Result<Vec<Branch>, GitError>;

    fn stage(&self, path: &Path) -> Result<(), GitError>;
    fn unstage(&self, path: &Path) -> Result<(), GitError>;
    fn commit(&self, message: &str) -> Result<CommitId, GitError>;

    // ... 其他操作
}
```

### 实现层

```rust
// 初期：使用 git2
pub struct Git2Repository {
    repo: git2::Repository,
}

impl GitRepository for Git2Repository {
    // 实现 trait 方法
}

// 长期：迁移到 gix
pub struct GixRepository {
    repo: gix::Repository,
}

impl GitRepository for GixRepository {
    // 实现 trait 方法
}
```

## UI 组件设计

### 布局系统

```
┌─────────────────────────────────────────────────┐
│  Tabs: [Status] [Commits] [Branches] [Stash]   │  <- Tab Bar
├─────────────────────────────────────────────────┤
│                                                 │
│                 Main Content                    │  <- Dynamic View
│                                                 │
│                                                 │
│                                                 │
│                                                 │
│                                                 │
├─────────────────────────────────────────────────┤
│ Mode: Normal | Branch: main | ↑5 ↓3 | q:quit   │  <- Status Bar
└─────────────────────────────────────────────────┘
```

### 视图组件

每个视图实现 `View` trait：

```rust
pub trait View {
    fn render(&self, frame: &mut Frame, area: Rect, app: &App);

    fn handle_key(&self, key: KeyEvent) -> Option<Message>;

    fn selected_index(&self) -> usize;
    fn items_count(&self) -> usize;
}
```

### Status 视图布局

```
┌─────────────┬───────────────────────────────────┐
│ Unstaged    │                                   │
│  file1.rs   │        Diff Preview               │
│  file2.rs   │                                   │
│  M file3.rs │   + fn new_function() {           │
│             │   +     // code                   │
│ Staged      │   + }                             │
│  file4.rs   │                                   │
│             │                                   │
└─────────────┴───────────────────────────────────┘
```

## 配置系统

### 配置文件结构

`~/.config/ratagit/config.toml`:

```toml
[ui]
theme = "dark"
show_line_numbers = true

[editor]
command = "vim"

[keybindings]
quit = "q"
stage = " "
commit = "c"
# 自定义快捷键

[git]
auto_fetch = true
fetch_interval = 300
```

### 配置加载

```rust
pub struct Config {
    pub ui: UiConfig,
    pub editor: EditorConfig,
    pub keybindings: KeyBindings,
    pub git: GitConfig,
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        // 1. 加载默认配置
        // 2. 加载用户配置
        // 3. 合并配置
    }
}
```

## 错误处理策略

### 错误类型层次

```rust
#[derive(Debug, thiserror::Error)]
pub enum RatagitError {
    #[error("Git error: {0}")]
    Git(#[from] GitError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    #[error("UI error: {0}")]
    Ui(String),
}

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Repository not found: {0}")]
    NotFound(PathBuf),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Merge conflict")]
    MergeConflict,

    // ...
}
```

### 错误展示

```rust
// 错误转换为用户友好的消息
impl From<RatagitError> for Message {
    fn from(err: RatagitError) -> Message {
        Message::ShowError(err.to_string())
    }
}
```

## 性能优化策略

### 1. 增量更新

```rust
struct App {
    // 只在需要时刷新数据
    last_refresh: Instant,
    cache_duration: Duration,
}

impl App {
    fn should_refresh(&self) -> bool {
        self.last_refresh.elapsed() > self.cache_duration
    }
}
```

### 2. 异步 Git 操作

```rust
pub enum Command {
    None,
    Async(Task<Message>),
    Sync(Message),
}

// 重型操作使用异步
fn update(app: &mut App, msg: Message) -> Command {
    match msg {
        Message::Refresh => {
            Command::Async(tokio::spawn(async {
                let status = repo.status().await?;
                Message::StatusLoaded(status)
            }))
        }
    }
}
```

### 3. 虚拟滚动

对于大型列表（如大量提交历史），只渲染可见项。

## 测试策略

### 单元测试

- Git 操作逻辑
- 消息处理
- 状态转换

### 集成测试

- UI 渲染
- 端到端工作流

### 测试辅助

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_repo() -> TempDir {
        // 创建临时测试仓库
    }

    #[test]
    fn test_stage_file() {
        let repo = create_test_repo();
        let mut app = App::new(repo.path());
        // ...
    }
}
```

## 开发路线图

### Phase 1: MVP ✅ (已完成 - Week 1-2)

**实现状态**：基础架构已完成，可以启动 TUI 并显示 Git status。

**已完成**：
- ✅ 项目结构搭建（混合架构）
- ✅ 基本事件循环（TEA 模式）
- ✅ Tab 切换（Status/Commits/Branches/Stash）
- ✅ Git status 显示（使用 git2）
- ✅ 文件列表（Unstaged/Staged/Untracked）
- ✅ GitRepository trait 抽象层
- ✅ View trait 组件系统
- ✅ 基础渲染（Tab bar + Status view + Status bar）

**技术实现**：
- TEA 架构：`App`（Model）、`Message`、`update()`、`Command`
- Git 抽象：`GitRepository` trait + `Git2Repository` 实现
- UI 组件：`View` trait + `StatusView` 实现
- 终端控制：ratatui + crossterm

**验收标准**：
- ✅ 能启动 TUI
- ✅ 显示 Git status（unstaged、staged、untracked 文件）
- ✅ `q` 能退出
- ✅ `tab` 能切换（虽然其他视图未实现）

**待优化**（Phase 2）：
- 文件列表导航（j/k 键）
- Stage/Unstage 文件（Space 键）
- Diff 预览
- 异步 Git 操作

### Phase 2: 核心功能 (Week 3-4)
- [ ] Stage/Unstage 文件
- [ ] Commit 功能
- [ ] 提交历史查看
- [ ] 分支列表
- [ ] Diff 显示

### Phase 3: 高级功能 (Week 5-8)
- [ ] Interactive Rebase
- [ ] Cherry-pick
- [ ] Stash 管理
- [ ] 分支创建/切换/删除
- [ ] 远程操作（push/pull/fetch）

### Phase 4: 完善优化 (Week 9-12)
- [ ] 配置系统
- [ ] 自定义快捷键
- [ ] 主题系统
- [ ] 性能优化
- [ ] 错误处理完善
- [ ] 测试覆盖

### Phase 5: 发布准备
- [ ] 文档完善
- [ ] CI/CD 设置
- [ ] 打包发布
- [ ] 社区反馈

## 关键设计决策

### 1. 为什么选择 TEA 架构？

- **可预测性**: 单向数据流使状态变化可追踪
- **可测试性**: 纯函数式更新，易于测试
- **可维护性**: 清晰的关注点分离
- **扩展性**: 容易添加新功能

### 2. 为什么初期用 git2？

- **成熟稳定**: libgit2 绑定经过生产验证
- **文档丰富**: 大量示例和社区支持
- **快速开发**: 降低初期开发难度

### 3. 为什么规划迁移到 gix？

- **纯 Rust**: 无 C 依赖，编译和分发更简单
- **性能**: 更好的性能特性
- **类型安全**: 更强的类型保证
- **未来**: gix 是 Rust Git 操作的未来

### 4. 为什么用 Tokio？

- **非阻塞**: Git 操作可能很慢，避免阻塞 UI
- **取消支持**: 用户可以取消长时间操作
- **并发**: 多个 Git 操作可以并行

## 扩展性考虑

### 插件系统（未来）

```rust
pub trait Plugin {
    fn name(&self) -> &str;
    fn on_message(&self, msg: &Message) -> Option<Message>;
    fn render(&self, frame: &mut Frame, area: Rect);
}
```

### 自定义视图（未来）

```rust
pub trait CustomView: View {
    fn id(&self) -> &str;
    fn config(&mut self, config: Value);
}
```

## 参考资料

- [ratatui documentation](https://ratatui.rs/)
- [git2-rs examples](https://docs.rs/git2/)
- [lazygit source code](https://github.com/jesseduffield/lazygit)
- [Elm Architecture](https://guide.elm-lang.org/architecture/)
- [gitoxide documentation](https://github.com/gitoxidelabs/gitoxide)

---

**下一步行动**: 开始 Phase 1 MVP 开发，创建基础项目结构。
