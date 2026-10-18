# Agent 开发方式评估与更新

调研日期：2026-10-16。范围：仓库开发规则、测试选择、CI；不涉及更换模型或改造应用运行时。

## 结论

采用「明确目标与不变量，按风险选择验证，允许 agent 自主实现」的开发方式。
减掉重复流程，保留独立、可执行的结果检查。这是本项目的工程判断，尚未通过
本仓库的 agent 对照实验证明提效，不能承诺具体时间或 token 节省比例。

这里有两个不同的 harness：

- Agent harness：模型的指令、工具、上下文与执行反馈，本次优化的主要对象。
- ratagit-harness：产品的场景测试以及真实 TUI 使用的 Runtime/AsyncRuntime/
  scheduler。它不是仅为弥补旧模型能力而存在的脚手架，不应据此删除。

## 近期代表性方案

以下选择有公开一手资料的主流产品实践，不代表经过市场份额排序的“最热门榜单”。

| 方案与资料日期 | 公开实践 | 对 ratagit 的选择 |
| --- | --- | --- |
| OpenAI Codex，2026-09-11 | 精简 skill/AGENTS 指令，按需展开上下文，重新审视过度具体的操作配方 | 根规则保留不变量、验证入口和文档导航；不给每种改动指定相同步骤 |
| Cursor，2026-01-09 / 04-30 | 简短项目规则、动态上下文，并结合 eval 与真实使用反馈改进 harness | 按需读文档，先跑相关测试；通过真实任务评估流程修改 |
| Anthropic Managed Agents，2026-04-08 | 将 session、harness、sandbox 解耦；旧模型需要的上下文重置可能成为新模型负担 | 不引入自建 agent 执行平台；把模型补偿策略与产品质量约束分开 |
| Anthropic 长时应用开发，2026-03-24 | planner/generator/evaluator，按交付验收契约迭代，验证实际用户流程 | 借鉴验收与反馈；多 agent 不设为默认，小任务不承担协调成本 |
| OpenAI skills eval | 对执行轨迹、产物与成功标准进行可重复评价 | 用正确性、返工和耗时共同判断流程改进 |

来源：

- [OpenAI：Rethinking skills and prompts](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra)
- [Cursor：Best practices for coding with agents](https://cursor.com/blog/agent-best-practices)
- [Cursor：Continually improving our agent harness](https://cursor.com/blog/continually-improving-agent-harness)
- [Anthropic：Scaling Managed Agents](https://www.anthropic.com/engineering/managed-agents)
- [Anthropic：Harness design for long-running application development](https://www.anthropic.com/engineering/harness-design-long-running-apps)
- [OpenAI：Testing Agent Skills Systematically with Evals](https://developers.openai.com/blog/eval-skills)

这些资料也有适用边界：厂商案例不是本项目的对照结果，前端应用实验不能直接
推导 Git TUI 的收益，多 agent 的输出质量收益也不等同于成本或速度收益。

## 仓库审计

审计起点：50e9f39，工作区初始无改动。

| 发现 | 影响 | 本次处理 |
| --- | --- | --- |
| AGENTS 要求每个 feature 走固定七步，DEVELOPMENT_LOOP 对小改动却允许简化 | 规则冲突，容易制造无价值计划和重复测试 | 合并为风险驱动规则；小改动用会话目标与证据 |
| 计划文件及其结构检查增加维护负担 | 工作被流程记录阻塞 | 删除计划文件与专用检查，目标和验证保留在会话中 |
| 所有可见改动都要求 harness，CI 文档进一步要求每个新 feature 新场景 | 纯布局/文字修改也可能复制完整交互场景 | UI 变化保留快照；交互/Git 语义保留 harness，可扩展已有覆盖 |
| 跨层改动与完整计划强绑定，流程强调最小步骤 | 容易拆出无法独立验证的半成品 | 允许一个完整且可验证的跨层改动，无需计划文件 |
| 完整 cargo test 后再跑 snapshots/harness 名称过滤 | 重复执行已经通过的测试，名称过滤也不代表完整类别 | 显式 --workspace 一次执行；保留双平台矩阵和失败产物 |
| CI 严格 lint 所有测试目标，AGENTS/TOOLING 只要求生产目标严格 lint | 实际门禁与书面规则不一致 | 统一 --workspace --lib --bins；测试仍全量编译运行 |
| ARCHITECTURE 规定单文件 500 行，但现有多个文件已超过 | 可能诱发与任务无关的拆分 | 按职责与维护成本拆分，不再以行数触发强制重构 |

保留：AppContext 单一状态、纯渲染、GitBackend 边界、请求归属与 mutation barrier、
固定输入的确定性、真实 Git 测试隔离、快照人工/agent 审查、失败产物。
这些约束保护产品正确性，不随模型变强自动失效。

## 后续任务怎么做

1. 明确可观察的验收结果，查看相关实现与现有测试。
2. 根据 [TESTING.md](TESTING.md) 选择覆盖；目标、风险和验证保留在会话中。
3. 完成有意义的改动，快速运行相关包/测试，按失败证据修复。
4. 代码或 CI 改动完成后执行完整门禁一次，审查 diff 和快照。
5. 交付说明实际证据和未验证项；长任务保留恢复所需的状态。

例如：改面板文案用对应快照；改快捷键流程用输入到 UI 的场景；改部分暂存
需要真实 Git 内容断言；改异步调度需要 stale-result/barrier 测试。
已有断言确实覆盖变更即可复用，不能把“运行了旧测试”当作覆盖新语义的证明。

## 如何确认确实提效

下次模型或工作流升级时做一次有界比较，不为每个日常任务增加记录负担：

- 选 6 个代表任务：文档修正、布局调整、纯逻辑缺陷、快捷键交互、部分暂存、
  异步结果顺序。固定起始提交、任务描述、验收标准和评审标准。
- 分别在独立工作区测试旧规则与新规则，保持模型版本、推理设置、工具权限、
  操作系统与构建缓存条件一致；记录顺序并交替执行，避免缓存偏向后一组。
- 每类至少重复两次；使用同一套事先确定的验收检查，而非只让实现 agent
  自己新写的测试评分。允许为发现的新风险补测试，但单独记录。
- 记录通过率、人工干预/返工次数、漏测回归、总耗时、测试耗时、工具调用数；
  token 数据可得时记录，不可得就标为缺失。
- 正确性与 Git 数据完整性优先；比较耗时中位数和范围，不凭一次快速任务下结论。
  如发现新增数据破坏、漏掉异步回归或覆盖退化，先恢复相应约束，再调查原因。
- 初期按实际 PR 观察结果补充案例；数据不足时保留“待验证”结论。

本次只验证流程/CI的一致性及现有测试是否通过，没有执行上述 agent A/B 实验。
后续可以再评估独立评审、按目录指令或专用 skill；出现重复需求后再引入。
