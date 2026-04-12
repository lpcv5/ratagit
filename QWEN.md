# Ratagit - 项目上下文

## 项目概述

Ratagit 是一个使用 Rust 和 `ratatui` 构建的快速、键盘优先的 Git 终端 UI 工具。它专为希望在不离开终端的情况下获得响应式 Git 工作流的开发者设计。

**核心特性：**
- 多面板 TUI，聚焦键盘工作流
- 仓库状态视图（未暂存/已暂存/未跟踪），支持树形导航
- 差异预览与平滑滚动
- 分支、提交和贮藏列表面板
- 支持自定义键位映射（`~/.config/ratagit/keymap.toml`）

## 技术栈

| 组件 | 技术 |
|------|------|
| TUI 框架 | `ratatui` + `crossterm` |
| Git 操作 | `git2` |
| 异步运行时 | `tokio` |
| 错误处理 | `anyhow`, `env_logger`, `log` |

## 项目结构

```
ratagit/
├── src/
│   ├── main.rs              # 入口点，启动前端和后端运行时
│   ├── app/                  # UI 层模块
│   │   ├── mod.rs           # 模块导出
│   │   ├── app.rs           # 主 App 结构体（UI 状态管理）
│   │   ├── runtime.rs       # UI 运行时（渲染循环、事件处理）
│   │   ├── ui_state.rs      # UI 状态定义（Panel, UiState）
│   │   └── cache.rs         # 缓存数据管理
│   ├── backend/              # 后端异步命令/事件循环模块
│   │   ├── mod.rs           # 模块导出
│   │   ├── commands.rs      # 后端命令枚举 (BackendCommand)
│   │   ├── events.rs        # 前端事件枚举 (FrontendEvent)
│   │   └── runtime.rs       # 后端运行时（Git I/O 执行）
│   ├── components/           # 共享 UI 组件定义
│   │   └── mod.rs
│   └── git/                  # Git 数据访问层
│       ├── mod.rs           # 模块导出
│       └── repo.rs          # GitRepo 封装（包装 git2::Repository）
├── docs/                     # 文档
│   └── plans/               # 计划文档
├── .github/workflows/        # CI 配置
├── Cargo.toml               # 项目依赖和构建配置
├── README.md                # 项目说明
├── AGENTS.md                # 仓库指南
└── CLAUDE.md                # 架构说明
```

## 架构设计

Ratagit 采用分层运行时架构，严格分离 Git I/O 和 UI 渲染：

```
main.rs
  ├── tokio::spawn(run_backend(cmd_rx, event_tx))   ← 后台任务处理 Git I/O
  └── App::new(cmd_tx, event_rx).run().await        ← 主线程处理 UI 渲染
```

**通道协议** — 使用无界 mpsc 通道，双向通信：

| 方向 | 类型 | 变体 |
|------|------|------|
| UI → Backend | `BackendCommand` | `RefreshStatus`, `RefreshBranches`, `RefreshCommits { limit }`, `RefreshStashes`, `GetDiff { file_path }`, `Quit` |
| Backend → UI | `FrontendEvent` | `FilesUpdated`, `BranchesUpdated`, `CommitsUpdated`, `StashesUpdated`, `DiffLoaded`, `Error` |

**布局** — 两列布局（34% / 66%）：
- 左列：Files、Branches、Commits、Stash 面板垂直堆叠
- 右列：Main View（差异/详情/概览）+ Log 面板

## 构建和运行命令

```bash
cargo run                  # 在当前 Git 仓库中运行 Ratagit
cargo build                # 编译调试版本
cargo check                # 快速编译/类型验证
cargo test                 # 运行所有测试
cargo fmt --check          # 验证格式（CI 强制要求）
cargo clippy --all-targets --all-features -- -D warnings  # 代码检查
```

**PR 前完整本地检查：**
```bash
cargo fmt --check && cargo check && cargo test && cargo clippy --all-targets --all-features -- -D warnings
```

## 开发约定

### 代码风格
- 遵循 `rustfmt` 默认值（4 空格缩进，标准 Rust 布局）
- 函数/模块/文件使用 `snake_case`，类型/枚举使用 `PascalCase`，常量使用 `SCREAMING_SNAKE_CASE`
- 模块按职责划分（UI 在 `app`，Git I/O 在 `git`，运行时接线在 `backend`）
- 状态转换使用小型纯函数辅助，副作用保留在 backend/runtime 边界

### 测试指南
- 窄单元测试使用 `#[cfg(test)]` 放在代码旁边
- 跨模块行为测试放在 `tests/` 中
- 测试名称应描述行为，如 `loads_diff_for_selected_file`
- Backend 更改应包含成功和错误路径覆盖

### 提交和 PR 指南
- 提交消息使用简洁的命令式总结，常用前缀：`feat:`, `refactor:`, `chore:`, `fix:`
- 每次提交应围绕一个逻辑变更
- PR 应包含：
  - 清晰的问题/解决方案摘要
  - 验证步骤和命令输出摘要
  - 链接的 issue（如适用）
  - TUI 可见更改的终端截图

## 默认键位绑定

| 键 | 操作 |
|----|------|
| `q` | 退出 |
| `h` / `←` | 上一个面板 |
| `l` / `→` | 下一个面板 |
| `1`-`4` | 跳转到面板 |
| `j` / `↓` | 向下移动 |
| `k` / `↑` | 向上移动 |
| `Space` | 暂存/取消暂存选中文件（Files 面板） |
| `Enter` | 展开/折叠目录（Files 面板） |
| `-` / `=` | 折叠/展开所有目录 |
| `Ctrl+U` / `Ctrl+D` | 差异向上/下滚动 |
| `r` | 刷新 |

## 关键模块说明

### `App` (src/app/runtime.rs)
- 管理所有 UI 状态：面板焦点（`Panel` 枚举）、四个 `ListState`、缓存的 Git 数据
- 渲染循环：排空后端事件 → 绘制 → 轮询输入（100ms 超时）

### `run_backend` (src/backend/runtime.rs)
- 打开 `GitRepo::discover()` 一次，然后循环等待 `cmd_rx.recv()`
- 分发到 `GitRepo` 方法并将结果作为 `FrontendEvent` 发回

### `GitRepo` (src/git/repo.rs)
- 封装 `git2::Repository`
- 所有 Git 操作集中在此：`get_status_files`, `get_branches`, `get_commits`, `get_stashes`, `get_diff`
