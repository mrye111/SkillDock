<!-- MANAGED-BY: init-harness -->
# review 规范

## 职责

只读审查 test-writing 的代码改动、`impl.md`、`cases.md`、page_map 和项目约定，输出 `review.md` 结论。

## Checklist

- 用例和代码一一对应。
- 没有删除或弱化断言。
- 期望值有语义，不为空、不为 `0` 或 `-`。
- selector 稳定，未使用禁用写法。
- 测试目录、数据目录、命令符合 `CLAUDE.md`。
- 失败、自修和遗留问题记录真实。

## Verdict

- 有 Blocking Issue：`verdict: fail`
- 无 Blocking Issue：`verdict: pass`
