# Ratagit Development Plan

> 详细的开发规划和任务分解

## Phase 1: MVP - 基础框架 (Week 1-2)

### Week 1: 项目初始化和基础结构

#### Day 1-2: 项目搭建
- [ ] 初始化 Cargo 项目
- [ ] 配置依赖 (ratatui, crossterm, git2, tokio, thiserror, serde)
- [ ] 创建基础目录结构
- [ ] 设置日志系统 (tracing)
- [ ] 创建基本的错误类型

#### Day 3-4: 事件循环和架构
- [ ] 实现基本的 App 结构体
- [ ] 定义 Message 枚举
- [ ] 实现 update 函数框架
- [ ] 实现主事件循环
- [ ] 实现基本的终端初始化/清理

#### Day 5-7: 基础 UI
- [ ] 实现 Tab 栏组件
- [ ] 实现基本的布局系统
- [ ] 实现状态栏
- [ ] 实现 Tab 切换功能
- [ ] 添加基本快捷键 (q: quit, tab: next tab)

### Week 2: Git Status 显示

#### Day 1-2: Git 抽象层
- [ ] 定义 GitRepository trait
- [ ] 实现 Git2Repository
- [ ] 定义 GitStatus, FileStatus 类型
- [ ] 实现 status() 方法
- [ ] 编写单元测试

#### Day 3-4: Status 视图
- [ ] 实现 Status 视图组件
- [ ] 显示 unstaged 文件列表
- [ ] 显示 staged 文件列表
- [ ] 实现列表选择和滚动
- [ ] 显示文件状态图标 (M, A, D, ?)

#### Day 5-7: Diff 显示
- [ ] 实现 diff 获取
- [ ] 实现 Diff 视图
- [ ] 分屏显示：文件列表 + diff 预览
- [ ] Diff 语法高亮
- [ ] 处理大文件 diff

**Phase 1 交付物**:
- ✅ 可运行的 TUI 应用
- ✅ 显示 Git 仓库状态
- ✅ 文件列表和 diff 预览
- ✅ Tab 切换功能

---

## Phase 2: 核心功能 (Week 3-4)

### Week 3: 基本操作

#### Day 1-2: Stage/Unstage
- [ ] 实现 stage 操作
- [ ] 实现 unstage 操作
- [ ] 添加快捷键 (space: toggle stage)
- [ ] 实现批量 stage (a: stage all)
- [ ] 实现行级 stage (未来功能准备)

#### Day 3-4: Commit 功能
- [ ] 实现提交输入框
- [ ] 实现 commit 操作
- [ ] 添加快捷键 (c: commit)
- [ ] 提交后自动刷新
- [ ] 提交历史查看基础

#### Day 5-7: 提交历史
- [ ] 实现 commits 列表视图
- [ ] 显示提交信息、作者、时间
- [ ] 实现提交历史滚动
- [ ] 显示提交 hash
- [ ] 提交详情查看

### Week 4: 分支管理

#### Day 1-2: 分支列表
- [ ] 实现 branches() 方法
- [ ] 显示分支列表
- [ ] 标记当前分支
- [ ] 显示分支信息 (upstream, ahead/behind)

#### Day 3-4: 分支操作
- [ ] 实现分支切换 (checkout)
- [ ] 实现分支创建
- [ ] 实现分支删除
- [ ] 添加快捷键 (n: new branch, d: delete)

#### Day 5-7: 优化和测试
- [ ] 性能优化
- [ ] 错误处理完善
- [ ] 单元测试补充
- [ ] 集成测试编写
- [ ] Bug 修复

**Phase 2 交付物**:
- ✅ Stage/Unstage 文件
- ✅ 提交功能
- ✅ 提交历史查看
- ✅ 分支管理基础功能

---

## Phase 3: 高级功能 (Week 5-8)

### Week 5-6: Stash 和远程操作

#### Stash 管理
- [ ] 实现 stash 列表
- [ ] 实现 stash create
- [ ] 实现 stash pop/apply
- [ ] 实现 stash drop
- [ ] Stash 视图 UI

#### 远程操作
- [ ] 显示 remote 列表
- [ ] 实现 fetch 操作
- [ ] 实现 pull 操作
- [ ] 实现 push 操作
- [ ] 显示 remote 状态

### Week 7-8: 高级 Git 操作

#### Interactive Rebase
- [ ] 设计 rebase UI
- [ ] 实现 rebase TODO 编辑
- [ ] 支持 squash/fixup/drop/reorder
- [ ] 实现 rebase 继续/中止
- [ ] Rebase 冲突处理

#### Cherry-pick
- [ ] 实现 cherry-pick 选择
- [ ] 实现 cherry-pick 执行
- [ ] Cherry-pick 冲突处理
- [ ] 批量 cherry-pick

#### 其他功能
- [ ] Merge 功能
- [ ] Reset 操作
- [ ] Revert 操作
- [ ] 标签管理

**Phase 3 交付物**:
- ✅ 完整的 stash 管理
- ✅ 远程仓库操作
- ✅ Interactive rebase
- ✅ Cherry-pick
- ✅ 其他高级操作

---

## Phase 4: 完善优化 (Week 9-12)

### Week 9: 配置系统

#### 配置加载
- [ ] 定义配置结构
- [ ] 实现 TOML 配置加载
- [ ] 配置文件路径处理
- [ ] 默认配置
- [ ] 配置验证

#### 快捷键系统
- [ ] 可配置快捷键
- [ ] 快捷键冲突检测
- [ ] 快捷键帮助显示
- [ ] 快捷键分组

### Week 10: UI 增强

#### 主题系统
- [ ] 定义主题结构
- [ ] 实现多个预设主题
- [ ] 主题切换功能
- [ ] 自定义颜色

#### 交互优化
- [ ] 搜索过滤功能
- [ ] 弹窗系统完善
- [ ] 输入框组件增强
- [ ] 确认对话框

### Week 11: 性能优化

#### 异步优化
- [ ] Git 操作异步化
- [ ] 后台自动 fetch
- [ ] 操作取消支持
- [ ] 进度显示

#### 渲染优化
- [ ] 虚拟滚动
- [ ] 增量渲染
- [ ] 缓存优化
- [ ] 减少重绘

### Week 12: 质量保证

#### 测试
- [ ] 单元测试覆盖率 > 70%
- [ ] 集成测试
- [ ] 端到端测试
- [ ] 性能测试

#### 文档
- [ ] API 文档
- [ ] 用户手册
- [ ] 配置文档
- [ ] 贡献指南

**Phase 4 交付物**:
- ✅ 完整的配置系统
- ✅ 主题支持
- ✅ 性能优化
- ✅ 完善的测试
- ✅ 完整的文档

---

## Phase 5: 发布准备 (Week 13-14)

### 打包和分发

#### 二进制发布
- [ ] Linux (x86_64, ARM)
- [ ] macOS (Intel, Apple Silicon)
- [ ] Windows (x86_64)
- [ ] 静态链接优化

#### 包管理器
- [ ] Homebrew formula
- [ ] AUR package
- [ ] Cargo install
- [ ] Snap package (可选)

### CI/CD

#### GitHub Actions
- [ ] 自动测试
- [ ] 自动构建
- [ ] 自动发布
- [ ] 代码质量检查

#### 文档网站
- [ ] GitHub Pages
- [ ] 在线文档
- [ ] 示例和教程

---

## 功能优先级矩阵

### P0 - 必须有 (MVP)
- Status 显示
- Stage/Unstage
- Commit
- 基本分支操作

### P1 - 应该有 (核心)
- 提交历史
- Diff 查看
- Push/Pull
- Stash

### P2 - 可以有 (增强)
- Interactive Rebase
- Cherry-pick
- 搜索过滤
- 主题系统

### P3 - 未来考虑
- 插件系统
- 自定义视图
- Git flow 支持
- Worktree 管理

---

## 技术债务跟踪

### 需要重构的部分
- [ ] Git 操作错误处理统一
- [ ] UI 组件抽象优化
- [ ] 配置系统重构
- [ ] 日志系统完善

### 需要优化的部分
- [ ] 大型仓库性能
- [ ] 内存使用优化
- [ ] 启动时间优化
- [ ] 渲染性能

### 需要测试的部分
- [ ] Git 操作边界情况
- [ ] UI 交互流程
- [ ] 配置加载
- [ ] 错误处理路径

---

## 风险和缓解

### 技术风险

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| git2 库限制 | 高 | 规划迁移到 gix |
| 性能问题 | 中 | 早期性能测试，异步设计 |
| 跨平台兼容 | 中 | CI 多平台测试 |
| UI 复杂度 | 低 | 采用成熟架构模式 |

### 项目风险

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 功能蔓延 | 高 | 严格的 MVP 定义 |
| 时间估计不准 | 中 | 迭代开发，定期评估 |
| 缺少测试 | 高 | 测试驱动开发 |

---

## 成功指标

### Phase 1 成功标准
- [ ] 可以显示当前仓库状态
- [ ] UI 流畅，无明显卡顿
- [ ] 基本导航可用

### Phase 2 成功标准
- [ ] 可以完成基本 Git 工作流
- [ ] 代码测试覆盖率 > 50%
- [ ] 无重大 Bug

### Phase 3 成功标准
- [ ] 功能接近 lazygit 80%
- [ ] 性能优于 lazygit
- [ ] 测试覆盖率 > 70%

### 发布标准
- [ ] 所有核心功能可用
- [ ] 文档完整
- [ ] 多平台测试通过
- [ ] 社区反馈积极

---

## 每周检查清单

### 每周一
- [ ] 回顾上周进度
- [ ] 确认本周目标
- [ ] 识别潜在阻塞

### 每日
- [ ] 提交代码
- [ ] 更新文档
- [ ] 运行测试

### 每周五
- [ ] 代码审查
- [ ] 技术债务清理
- [ ] 更新路线图

---

**下一步行动**: 开始 Phase 1 - Day 1 任务：项目初始化
