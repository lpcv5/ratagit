# Ratagit 技术决策记录 (ADR)

> Architecture Decision Records - 记录重要的技术决策及其理由

## ADR-001: 采用 The Elm Architecture (TEA)

### 状态
✅ 已接受

### 背景
Ratagit 需要一个清晰、可维护的架构来处理复杂的 UI 状态和 Git 操作。我们需要一个能够：
- 清晰地管理应用状态
- 处理用户输入和异步事件
- 使代码易于测试和扩展
- 降低 Bug 率

### 决策
采用 The Elm Architecture (TEA) 模式，包含三个核心组件：
1. **Model**: 应用状态的单一数据源
2. **Message**: 表示用户操作或系统事件
3. **Update**: 纯函数，根据 Message 更新 Model

### 理由
- **可预测性**: 单向数据流使状态变化完全可追踪
- **可测试性**: Update 是纯函数，易于编写单元测试
- **可维护性**: 清晰的关注点分离，代码易于理解
- **扩展性**: 添加新功能只需添加新的 Message 和处理逻辑
- **经过验证**: Elm、Redux 等已经在生产环境中证明了该模式的有效性

### 后果
- **优点**: 代码结构清晰，Bug 少，易于维护
- **缺点**: 初期需要编写更多样板代码
- **替代方案**: MVC、Component-based architecture

---

## ADR-002: 初期使用 git2，长期迁移到 gix

### 状态
✅ 已接受

### 背景
Ratagit 需要与 Git 仓库交互。Rust 生态系统中有两个主要选择：
- **git2-rs**: libgit2 的 Rust 绑定
- **gix (gitoxide)**: 纯 Rust 实现的 Git 库

### 决策
- **Phase 1-3**: 使用 git2-rs 进行快速开发
- **Phase 4+**: 逐步迁移到 gix

### 理由

#### 为什么初期用 git2？
1. **成熟稳定**: libgit2 已经在生产环境使用多年
2. **文档丰富**: 大量示例和教程
3. **社区支持**: 遇到问题容易找到解决方案
4. **功能完整**: 支持所有 Git 操作

#### 为什么要迁移到 gix？
1. **纯 Rust**: 无 C 依赖，编译和分发更简单
2. **性能**: 专为性能优化的实现
3. **类型安全**: 更强的类型系统和错误处理
4. **未来**: gix 是 Rust Git 操作的未来

### 后果
- **优点**: 快速开发 + 长期技术优势
- **缺点**: 需要维护抽象层，迁移工作量大
- **缓解**: 定义 GitRepository trait，隔离实现细节

---

## ADR-003: 使用 Tokio 异步运行时

### 状态
✅ 已接受

### 背景
Git 操作（如 clone、fetch、push）可能需要较长时间。如果同步执行，会阻塞 UI，导致糟糕的用户体验。

### 决策
使用 Tokio 作为异步运行时，将耗时的 Git 操作异步化。

### 理由
1. **非阻塞 UI**: 主线程专注于 UI 渲染
2. **取消支持**: 用户可以取消长时间操作
3. **并发**: 多个 Git 操作可以并行执行
4. **生态系统**: Tokio 是 Rust 异步生态的标准
5. **性能**: 高效的异步 I/O

### 后果
- **优点**: UI 流畅，用户体验好
- **缺点**: 增加代码复杂度，需要处理异步错误
- **示例**: 使用 Command 模式管理异步操作

```rust
enum Command {
    Async(tokio::task::JoinHandle<Message>),
    Sync(Message),
}
```

---

## ADR-004: 使用 thiserror 进行错误处理

### 状态
✅ 已接受

### 背景
Ratagit 涉及多种可能的错误：
- Git 操作错误
- I/O 错误
- 配置错误
- UI 渲染错误

### 决策
使用 `thiserror` 库定义自定义错误类型。

### 理由
1. **类型安全**: 编译时错误检查
2. **可读性**: 清晰的错误消息
3. **可组合**: 使用 `#[from]` 自动转换
4. **零成本**: 无运行时开销
5. **标准**: Rust 社区推荐做法

### 后果
- **优点**: 错误处理清晰，易于调试
- **缺点**: 需要为每种错误定义类型

### 示例
```rust
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Repository not found: {0}")]
    NotFound(PathBuf),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}
```

---

## ADR-005: 配置使用 TOML 格式

### 状态
✅ 已接受

### 背景
Ratagit 需要支持用户配置，包括：
- UI 主题
- 快捷键绑定
- Git 行为配置

### 决策
使用 TOML 作为配置文件格式。

### 理由
1. **可读性**: 对人类友好，易于编辑
2. **类型支持**: 支持字符串、数字、数组、表等
3. **Rust 生态**: `serde` + `toml` 无缝集成
4. **行业标准**: Rust 社区广泛使用 (Cargo.toml)
5. **工具支持**: 有良好的编辑器支持和验证工具

### 后果
- **优点**: 配置文件清晰，易于维护
- **缺点**: 不支持复杂的数据结构（需要时可用 JSON）

### 配置位置
```
~/.config/ratagit/config.toml
```

---

## ADR-006: 日志使用 tracing 库

### 状态
✅ 已接受

### 背景
开发和调试过程中需要详细的日志记录。

### 决策
使用 `tracing` 库进行日志记录。

### 理由
1. **结构化日志**: 支持结构化字段
2. **异步友好**: 与 Tokio 完美集成
3. **性能**: 零成本抽象，可配置日志级别
4. **丰富功能**: Span、Event、Subscriber
5. **生态系统**: tracing-subscriber, tracing-appender 等

### 后果
- **优点**: 调试方便，性能好
- **缺点**: 需要学习 tracing API

### 日志位置
```
~/.local/share/ratagit/logs/ratagit.log
```

---

## ADR-007: UI 组件化设计

### 状态
✅ 已接受

### 背景
Ratagit 的 UI 需要支持多个视图和可复用组件。

### 决策
采用组件化设计，每个组件实现 `View` trait。

### 理由
1. **复用性**: 组件可以在不同地方复用
2. **封装**: 每个组件管理自己的状态
3. **测试**: 组件可以独立测试
4. **扩展**: 容易添加新组件

### 设计
```rust
pub trait View {
    fn render(&self, frame: &mut Frame, area: Rect, app: &App);
    fn handle_key(&self, key: KeyEvent) -> Option<Message>;
}
```

### 组件层次
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

---

## ADR-008: Git 操作抽象层设计

### 状态
✅ 已接受

### 背景
为了支持从 git2 迁移到 gix，需要隔离底层实现。

### 决策
定义 `GitRepository` trait，抽象 Git 操作。

### 理由
1. **解耦**: 业务逻辑不依赖具体实现
2. **可测试**: 可以创建 Mock 实现
3. **可替换**: 方便迁移到不同库
4. **清晰**: 明确的 API 边界

### 设计
```rust
pub trait GitRepository {
    fn status(&self) -> Result<GitStatus, GitError>;
    fn commit(&self, message: &str) -> Result<CommitId, GitError>;
    // ... 其他方法
}
```

---

## ADR-009: 增量更新策略

### 状态
✅ 已接受

### 背景
大型仓库的 Git 状态查询可能很慢，需要优化刷新策略。

### 决策
实现增量更新和缓存机制。

### 策略
1. **手动刷新**: 用户按 `r` 手动刷新
2. **智能刷新**: 执行操作后自动刷新相关部分
3. **后台刷新**: 定期后台 fetch
4. **缓存**: 缓存不变的数据

### 理由
1. **性能**: 减少不必要的 Git 操作
2. **体验**: UI 响应更快
3. **资源**: 降低 CPU 和 I/O 使用

---

## ADR-010: 快捷键系统设计

### 状态
✅ 已接受

### 背景
快捷键是 TUI 应用的核心交互方式，需要灵活可配置。

### 决策
实现可配置的快捷键系统，支持：
- 单键绑定
- 组合键 (Ctrl+X, Alt+X)
- 模式相关绑定 (Normal mode vs Insert mode)

### 设计
```rust
pub struct KeyBindings {
    pub quit: KeyEvent,
    pub stage: KeyEvent,
    pub commit: KeyEvent,
    // ...
}

impl KeyBindings {
    pub fn from_config(config: &Config) -> Self {
        // 从配置加载
    }
}
```

### 默认快捷键
- `q`: 退出
- `tab`: 下一个 tab
- `space`: stage/unstage
- `c`: commit
- `r`: refresh

---

## ADR-011: 测试策略

### 状态
✅ 已接受

### 背景
确保代码质量和稳定性需要全面的测试。

### 决策
采用多层测试策略：
1. **单元测试**: 测试独立函数和模块
2. **集成测试**: 测试模块间交互
3. **端到端测试**: 测试完整用户流程

### 工具
- `cargo test`: 单元测试和集成测试
- 临时目录: 创建测试 Git 仓库
- Mock: 模拟 Git 操作

### 目标覆盖率
- Phase 1-2: > 50%
- Phase 3: > 70%
- 发布: > 80%

---

## ADR-012: 版本控制和发布策略

### 状态
✅ 已接受

### 背景
Ratagit 需要明确的版本管理和发布流程。

### 决策
采用语义化版本控制 (SemVer):
- **MAJOR**: 不兼容的 API 变化
- **MINOR**: 向后兼容的新功能
- **PATCH**: 向后兼容的 Bug 修复

### 发布渠道
1. **GitHub Releases**: 官方二进制发布
2. **Crates.io**: `cargo install ratagit`
3. **Homebrew**: macOS 用户
4. **AUR**: Arch Linux 用户

### 发布频率
- **Alpha**: 每周（Phase 1-3）
- **Beta**: 每月（Phase 4）
- **Stable**: Phase 5 完成后

---

## 未来决策

以下决策将在后续阶段确定：

### 待定-001: 插件系统
- **问题**: 是否支持插件扩展？
- **考虑**: 复杂度 vs 灵活性
- **时间**: Phase 5+

### 待定-002: GUI 版本
- **问题**: 是否开发 GUI 版本？
- **考虑**: 依赖 egui 或 iced
- **时间**: 取决于社区反馈

### 待定-003: 协作功能
- **问题**: 是否支持协作功能（如 PR review）？
- **考虑**: GitHub/GitLab API 集成
- **时间**: 远期规划

---

## 决策回顾流程

### 每月回顾
- 评估已做决策的有效性
- 识别需要调整的决策
- 更新技术债务列表

### 回顾问题
1. 这个决策是否仍然合理？
2. 有什么负面后果需要缓解？
3. 是否有新的信息需要重新考虑？

---

**维护说明**:
- 每个 ADR 创建后不再修改，只标记为"已废弃"或"已替代"
- 新的决策创建新的 ADR
- 定期回顾和清理
