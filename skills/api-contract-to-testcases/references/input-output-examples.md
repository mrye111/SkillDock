# 最小输入输出样例

## 1. 推荐输入类型

- Swagger 2.0 JSON
- OpenAPI 3.x JSON / YAML
- Apifox / Knife4j / YApi 导出文本
- 用户手工整理的纯文本接口契约

## 2. 一键生成命令

```powershell
python C:/Users/Administrator/.codex/skills/api-contract-to-testcases/scripts/generate_deliverables.py "C:/Users/Administrator/Desktop/物业接口契约文档.txt"
```

如需保留中间结构化数据：

```powershell
python C:/Users/Administrator/.codex/skills/api-contract-to-testcases/scripts/generate_deliverables.py "C:/Users/Administrator/Desktop/物业接口契约文档.txt" --keep-json
```

## 3. 默认输出文件

- `xxx_接口测试要点.md`
- `xxx_接口测试用例.xlsx`
- `xxx_待确认项.md`
- `xxx_审核评分报告.md`

可选中间文件：

- `xxx_normalized_contract.json`
- `xxx_testcases.json`

## 4. 生成原则

- 默认只输出最终交付物，不额外生成冗余中间文档。
- 如果契约没有给出明确规则，不虚构业务事实，转入待确认项。
- 评分报告优先用于二次检查 AI 生成结果，也可用于检查本 skill 自己生成的结果。
