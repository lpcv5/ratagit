# Ratagit

> A fast, intuitive terminal UI for Git operations, inspired by lazygit

**Status**: Phase 2 in progress

## 项目概述

Ratagit 是一个使用 Rust 编写的终端 Git 客户端，基于 ratatui 构建。目标是提供一个快速、直观、功能完整的 Git TUI 工具。

### 核心特性

- **混合架构**: TEA (The Elm Architecture) + Component 模式
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
| `Enter`/`Space` | Toggle directory expand/collapse (Files panel) |
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

采用 **TEA + Component 混合架构**：

- **TEA 主干**: 全局状态、消息驱动、纯函数更新
- **Component 辅助**: View trait 封装渲染和事件处理

详细设计见 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)

## Development Roadmap

See [docs/ROADMAP.md](docs/ROADMAP.md) for re-baselined phases and exit criteria, and [docs/STATUS.md](docs/STATUS.md) for the live parity scorecard.

## 文档

- [架构设计](docs/ARCHITECTURE.md)
- [开发路线图](docs/ROADMAP.md)
- [状态看板](docs/STATUS.md)
- [技术决策](docs/DECISIONS.md)

## 贡献

项目目前处于早期开发阶段，欢迎贡献！

## 许可证

MIT
