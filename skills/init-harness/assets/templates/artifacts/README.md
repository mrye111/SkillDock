<!-- MANAGED-BY: init-harness -->
# Multi-Agent Web UI 测试编排规范

本目录定义多 agent Web UI 自动化测试的 artifact 协作规范。agent 之间不直接对话，通过 `artifacts/<task_id>/` 下的文档接力。

## Agent 职责

| Agent | 职责 | 产物 |
|---|---|---|
| orchestrator | 主控、路由、维护状态机 | `state.md` |
| page-map-sync | 探索页面并更新页面地图 | `sync.md`, `page_map/*.yaml` |
| test-case-design | 需求转测试用例 | `cases.md` |
| test-writing | 用例转自动化测试并自跑自修 | `impl.md`, `tests/*`, `data/*` |
| review | 静态审查测试改动 | `review.md` |

## Artifact Schema

每份 artifact 使用 YAML frontmatter + Markdown 正文。

```yaml
---
task_id: 2026-07-02_feature_flow
agent: page-map-sync
status: completed
inputs: []
outputs: {}
next_agent: test-case-design
created_at: 2026-07-02T10:00:00Z
---
```

状态枚举：`pending`、`in_progress`、`pending_review`、`confirmed`、`completed`、`failed`、`blocked`。

## 强制 Gate

`cases.md` 产出后必须停在 `status: pending_review`，等用户编辑或确认后才能进入 `test-writing`。

## Progress Log

每个任务必须维护：

```text
artifacts/<task_id>/progress.log
```

每个关键阶段 append 一行：

```text
[YYYY-MM-DD HH:MM:SS] <agent> <event>
```

必写事件：`start`、`route:<agent>`、`step:<summary>`、`gate:cases_review`、`done:<artifact>`、`error:<summary>`。

## 红线

- 不绕过 `cases.md` 审核 gate。
- 不删断言来让测试通过。
- 不把期望值改成空、`0` 或 `-` 来迁就实际结果。
- selector 禁用 hash class、`:nth-child`、xpath 位置索引。
- 失败要如实写入 artifact，不写假完成。
