# Ratagit

> A fast, intuitive terminal UI for Git operations, inspired by lazygit

**状态**: Phase 1 MVP 已完成 ✅

## 项目概述

Ratagit 是一个使用 Rust 编写的终端 Git 客户端，基于 ratatui 构建。目标是提供一个快速、直观、功能完整的 Git TUI 工具。

### 核心特性

- **混合架构**: TEA (The Elm Architecture) + Component 模式
- **类型安全**: 通过类型系统保证 Git 操作的安全性
- **异步支持**: 使用 Tokio 处理耗时的 Git 操作
- **可扩展**: GitRepository trait 抽象，便于迁移到 gix

## 当前功能 (Phase 1 MVP)

- ✅ 启动 TUI 界面
- ✅ 显示 Git status（Unstaged/Staged/Untracked 文件）
- ✅ Tab 切换（Status/Commits/Branches/Stash）
- ✅ 基础快捷键（q 退出，tab 切换）

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
| `q` | 退出 |
| `Tab` | 下一个 Tab |
| `Shift+Tab` | 上一个 Tab |
| `j`/`↓` | 向下移动（待实现） |
| `k`/`↑` | 向上移动（待实现） |
| `Space` | Stage/Unstage 文件（待实现） |
| `r` | 刷新状态（待实现） |

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

## 开发路线

### Phase 1: MVP ✅ (已完成)
基础架构和 Git status 显示

### Phase 2: 核心功能 (进行中)
- Stage/Unstage 文件
- Commit 功能
- 提交历史查看
- Diff 显示

### Phase 3: 高级功能
- Interactive Rebase
- Cherry-pick
- Stash 管理
- 远程操作

### Phase 4: 完善优化
- 配置系统
- 主题系统
- 性能优化
- 测试覆盖

详细路线图见 [docs/ROADMAP.md](docs/ROADMAP.md)

## 文档

- [架构设计](docs/ARCHITECTURE.md)
- [开发路线图](docs/ROADMAP.md)
- [技术决策](docs/DECISIONS.md)

## 贡献

项目目前处于早期开发阶段，欢迎贡献！

## 许可证

MIT
