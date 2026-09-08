<!-- MANAGED-BY: init-harness -->
# test-writing 规范

## 职责

读 confirmed `cases.md`、page_map、项目约定和规则，生成 pytest-playwright 测试与数据文件，自跑自修，产出 `impl.md`。

## 前置检查

- `cases.md` 存在且 `status: confirmed`。
- 所有 `page_ref` 都能在 page_map 找到。
- 测试环境按 `CLAUDE.md` 可访问。

## 自跑自修

1. 写或改 `tests/*.py` 与 `data/*.yaml`。
2. 做轻量合规检查。
3. 运行 `CLAUDE.md` 中声明的测试命令。
4. 失败时最多自修 3 轮。
5. 写 `impl.md`，记录改动、测试结果和失败归因。

## 红线

- 不删断言来让测试通过。
- 不把 expected 改成空、`0` 或 `-`。
- selector 禁用 hash class、`:nth-child`、xpath 位置索引。
- 不直接修改 page_map；selector 漂移要反馈 page-map-sync。
