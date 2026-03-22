# Ratagit

> A fast, intuitive terminal UI for Git operations, inspired by lazygit

**Status**: Milestone execution tracked in `.track/`

## 项目概述

Ratagit 是一个使用 Rust 编写的终端 Git 客户端，基于 ratatui 构建。目标是提供一个快速、直观、功能完整的 Git TUI 工具。

### 核心特性

- **Flux 架构**: 单向数据流（Action -> Dispatcher -> Stores -> Effect Runtime）
- **类型安全**: 通过类型系统保证 Git 操作的安全性
- **异步支持**: 使用 Tokio 处理耗时的 Git 操作
- **可扩展**: GitRepository trait 抽象，便于迁移到 gix

## Current Features

- TUI lifecycle and multi-panel layout
- Git status (unstaged/staged/untracked) with file tree
- Diff preview with scrolling
- Branch/commit/stash listing panels
- Configurable keymap (`~/.config/ratagit/keymap.toml`)

## 快速开始

### 构建

```bash
cargo build
```

### 运行

在 Git 仓库目录中运行：

```bash
cargo run
```

### 快捷键

| 按键 | 功能 |
|------|------|
| `q` | Quit |
| `h`/`←` | Previous panel |
| `l`/`→` | Next panel |
| `1`-`4` | Jump to panel |
| `j`/`↓` | Move down |
| `k`/`↑` | Move up |
| `Space` | Stage/unstage selected file (Files panel) |
| `Enter` | Toggle directory expand/collapse (Files panel) |
| `-` / `=` | Collapse/expand all directories |
| `Ctrl+U` / `Ctrl+D` | Scroll diff up/down |
| `r` | Refresh |

## 技术栈

- **ratatui** - 终端 UI 框架
- **crossterm** - 跨平台终端控制
- **git2** - Git 操作库（初期）
- **tokio** - 异步运行时
- **thiserror** - 错误处理

## 架构

采用 **Flux + Tokio 三循环架构**：

- **Action/Dispatcher/Stores**: 输入映射为 `Action`，由 `Dispatcher` 按序驱动各域 store
- **Effect Runtime**: 所有 Git I/O 通过 `EffectRequest` 在运行时执行，reducer 不直接做 I/O
- **Snapshot 渲染**: UI 只消费 `AppStateSnapshot`，渲染与状态更新解耦

架构与路线说明见 [docs/DECISIONS.md](docs/DECISIONS.md) 与 [docs/DEVELOPMENT_MODEL.md](docs/DEVELOPMENT_MODEL.md)

## Development Tracking

Use the `.track/` workspace and the `project-tracker` skill for all planning/execution tracking.
See [docs/DEVELOPMENT_MODEL.md](docs/DEVELOPMENT_MODEL.md) for the process model.

## 文档

- [开发模型](docs/DEVELOPMENT_MODEL.md)
- [技术决策](docs/DECISIONS.md)

## 贡献

项目目前处于早期开发阶段，欢迎贡献！

## 许可证

MIT
