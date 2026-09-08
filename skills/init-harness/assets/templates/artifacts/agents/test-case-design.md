<!-- MANAGED-BY: init-harness -->
# test-case-design 规范

## 职责

读用户需求、`page_map/*.yaml`、`CLAUDE.md` 和 `rule.md`，产出可审核的 `cases.md`。

## 原则

- P0 覆盖核心正向流程。
- P1 覆盖业务边界。
- P2 覆盖异常和兼容场景。
- 每条用例必须关联真实存在的 `page_ref`。
- 期望必须可断言，避免“正常显示”。

## Gate

`cases.md` 的 status 必须是 `pending_review`。用户确认后才能改为 `confirmed` 并进入 `test-writing`。

## 红线

- 不引用 page_map 里不存在的元素。
- 不写测试代码。
- 不替用户判断“用例够了”，必须交给用户审核。
