# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

Ratagit 是一个使用 Rust 编写的终端 Git 客户端，基于 ratatui 构建，灵感来自 lazygit。项目目前处于架构设计阶段（Phase 0），尚未开始实际代码开发。

## 常用命令

```bash
# 构建项目
cargo build

# 运行项目（开发完成后）
cargo run

# 运行测试
cargo test

# 运行单个测试
cargo test test_name

# 发布构建（优化体积和性能）
cargo build --release

# 检查代码
cargo check

# 格式化代码
cargo fmt

# 代码检查
cargo clippy
```

## 核心架构

### The Elm Architecture (TEA)

项目采用 TEA 模式，包含单向数据流：

```
User Input → Event → Message → Update → Model → View → Render
```

**三大核心组件**：

1. **Model**: 应用状态（`App` 结构体）
2. **Message**: 用户操作或系统事件（`Message` 枚举）
3. **Update**: 纯函数，根据 Message 更新 Model（`update()` 函数）

### 模块结构

```
src/
├── app/          # 应用核心（TEA 架构）
│   ├── app.rs        # App 结构体和主循环
│   ├── state.rs      # 应用状态定义
│   ├── message.rs    # 消息/事件定义
│   └── command.rs    # 命令定义
├── git/          # Git 操作抽象层
│   ├── repository.rs # Repository trait（重要：抽象 git2/gix）
│   ├── status.rs     # 状态查询
│   └── ...
├── ui/           # UI 组件
│   ├── views/        # 视图组件（Status, Commits, Branches, Stash）
│   ├── widgets/      # 可复用组件（Tab, List, Popup, Input）
│   └── renderer.rs   # 渲染器
├── event/        # 事件处理
│   ├── handler.rs    # 事件处理器
│   └── keybind.rs    # 快捷键映射
├── config/       # 配置系统
└── utils/        # 工具函数
```

## 关键技术决策

### GitRepository Trait 抽象层

**重要**: Git 操作必须通过 `GitRepository` trait 进行，不直接调用 git2。

```rust
pub trait GitRepository {
    fn status(&self) -> Result<GitStatus, GitError>;
    fn commit(&self, message: &str) -> Result<CommitId, GitError>;
    // ... 其他方法
}
```

**原因**：
- 初期使用 git2（成熟稳定，文档丰富）
- 长期迁移到 gix（纯 Rust，无 C 依赖，性能更好）
- 抽象层使迁移更容易

实现位于 `git/repository.rs`：
- `Git2Repository`: git2 实现（Phase 1-3）
- `GixRepository`: gix 实现（Phase 4+，未来）

### 异步 Git 操作

Git 操作使用 Tokio 异步执行，避免阻塞 UI：

```rust
enum Command {
    Async(tokio::task::JoinHandle<Message>),
    Sync(Message),
}
```

### 错误处理

使用 `thiserror` 定义自定义错误类型：

```rust
#[derive(Debug, thiserror::Error)]
pub enum RatagitError {
    #[error("Git error: {0}")]
    Git(#[from] GitError),
    // ...
}
```

## 开发路线

### Phase 1: MVP (Week 1-2)
- 基础事件循环
- Git status 显示
- 文件列表和 diff 预览
- Tab 切换

### Phase 2: 核心功能 (Week 3-4)
- Stage/Unstage 文件
- Commit 功能
- 提交历史查看
- 分支管理

### Phase 3: 高级功能 (Week 5-8)
- Interactive Rebase
- Cherry-pick
- Stash 管理
- 远程操作

### Phase 4: 完善优化 (Week 9-12)
- 配置系统
- 主题系统
- 性能优化
- 测试覆盖

详细计划见 `docs/ROADMAP.md`

## 配置

配置文件位置：`~/.config/ratagit/config.toml`

格式：TOML（使用 serde 反序列化）

配置结构定义在 `config/config.rs`

## 日志

使用 `tracing` 库进行结构化日志记录。

日志位置：`~/.local/share/ratagit/logs/ratagit.log`

## 测试策略

- **单元测试**: 测试独立函数和模块（`#[cfg(test)]`）
- **集成测试**: 测试模块间交互（`tests/` 目录）
- **测试辅助**: 使用 `tempfile` 创建临时 Git 仓库进行测试

目标覆盖率：
- Phase 1-2: > 50%
- Phase 3: > 70%
- 发布: > 80%

## UI 组件设计

每个视图实现 `View` trait：

```rust
pub trait View {
    fn render(&self, frame: &mut Frame, area: Rect, app: &App);
    fn handle_key(&self, key: KeyEvent) -> Option<Message>;
}
```

组件层次：
```
App
├── TabBar
├── MainView (动态)
│   ├── StatusView
│   ├── CommitsView
│   ├── BranchesView
│   └── StashView
└── StatusBar
```

## 性能优化

- **增量更新**: 只在需要时刷新数据
- **异步操作**: 重型 Git 操作使用 Tokio
- **虚拟滚动**: 大型列表只渲染可见项

## 文档

- `docs/ARCHITECTURE.md` - 详细架构设计
- `docs/ROADMAP.md` - 开发路线图
- `docs/DECISIONS.md` - 技术决策记录（ADR）

## 注意事项

1. **Git 操作**: 始终通过 `GitRepository` trait，不直接调用 git2/gix
2. **状态更新**: 遵循 TEA 模式，通过 Message 更新状态
3. **异步处理**: 耗时操作使用 `Command::Async`
4. **错误处理**: 使用 `thiserror` 定义清晰的错误类型
5. **测试**: 为新功能编写单元测试和集成测试
