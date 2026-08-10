# Agent Router

Choose the workflow by intent, not by brittle keyword matching. Chinese and English prompts use the same mapping.

| Intent | Skill | Workflow |
| --- | --- | --- |
| Clarify, grill, align, 商量, 对齐 | [cancan-design-grill](./skills/cancan-design-grill/SKILL.md) | [design-grill](./workflows/design-grill.md) |
| Implement, build, change a slice, 实现, 构建, 变更 | [cancan-implementation-cycle](./skills/cancan-implementation-cycle/SKILL.md) | [implement-feature](./workflows/implement-feature.md) |
| Review, audit, evaluate, 审查, 评估 | [cancan-code-review](./skills/cancan-code-review/SKILL.md) | [review-code](./workflows/review-code.md) |
| Test, simulate, QA, 测试, 模拟 | [cancan-testing-simulation](./skills/cancan-testing-simulation/SKILL.md) | [simulated-testing](./workflows/simulated-testing.md) |
| Architecture, boundaries, 架构, 边界 | [cancan-architecture-refinement](./skills/cancan-architecture-refinement/SKILL.md) | [refine-architecture](./workflows/refine-architecture.md) |
| UI, visual polish, 界面, 视觉 | [cancan-ui-quality](./skills/cancan-ui-quality/SKILL.md) | [refine-ui](./workflows/refine-ui.md) |
| Connector, Gmail, source integration, 数据源 | [cancan-implementation-cycle](./skills/cancan-implementation-cycle/SKILL.md) | [plugin-work](./workflows/plugin-work.md) |
| Orientation, status, plan, 进度, 规划 | [cancan-docs-orientation](./skills/cancan-docs-orientation/SKILL.md) | [development-cycle](./workflows/development-cycle.md) |
| Issue, triage, track, 工单, 追踪, 闭环 | [cancan-issue-triage](./skills/cancan-issue-triage/SKILL.md) | [issue-delivery](./workflows/issue-delivery.md) |
| File an issue, report a finding, 提issue, 报问题 | [cancan-issue-filing](./skills/cancan-issue-filing/SKILL.md) | [file-issues](./workflows/file-issues.md) |
| Fix a bug, fix an issue, close an issue, 修复, 修issue, 处理issue | [cancan-issue-fixing](./skills/cancan-issue-fixing/SKILL.md) | [fix-issue](./workflows/fix-issue.md) |

For mixed intents, resolve the earliest blocker first. Product, money-correctness, security, and irreversible data ambiguities block implementation. Unknown intent defaults to orientation, never implementation.

Fix vs implement: a reported defect or a filed bug routes to fixing (verify-before-trust loop); building new or changed behavior for a slice routes to implementation. Both run inside the issue loop.

Implementation intents (Implement, Review, Test, and similar rows) assume a tracked GitHub issue already exists for the work (AGENTS.md §10). If none exists, start with `cancan-issue-triage` / `issue-delivery` instead — implementation never starts issue-less.
