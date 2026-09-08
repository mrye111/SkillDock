<!-- MANAGED-BY: init-harness -->
# orchestrator 规范

## 职责

- 接收用户需求，判断该走哪条流水线。
- 维护 `artifacts/<task_id>/state.md`。
- 调度 `page-map-sync`、`test-case-design`、`test-writing`、`review`。
- 遇到 `cases.md status=pending_review` 必须停下等用户确认。

## 路由

| 用户意图 | 调用 |
|---|---|
| 探索/同步页面地图 | page-map-sync |
| 设计测试用例 | test-case-design |
| 写自动化测试 | test-case-design -> gate -> test-writing -> review |
| 完整补测试 | page-map-sync -> test-case-design -> gate -> test-writing -> review |
| 审查测试改动 | review |

## 红线

- 不写业务测试代码。
- 不直接改 page_map。
- 不跳过 `cases.md` gate。
- 每次路由和 gate 都写 `progress.log`。
