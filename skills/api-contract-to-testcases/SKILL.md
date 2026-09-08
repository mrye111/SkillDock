---
name: api-contract-to-testcases
description: 根据 Swagger/OpenAPI JSON、YAML、Apifox 导出内容或纯文本接口契约文档，生成固定模板的接口测试用例，并输出测试要点、待确认项、审核评分报告。适用于需要从 API 契约提取接口、鉴权、参数约束、响应结构，按“接口中文名称、前置条件、测试场景、path、headers、body、预期响应结果、断言响应、断言sql”模板批量生成接口测试交付物，并对生成结果做二次检查与评分的场景。
---

# 接口契约转测试用例

优先把 skill 当成闭环生成器使用，不要只停留在提示词层。

## 默认执行方式

拿到契约文档后，优先直接运行：

`scripts/generate_deliverables.py <contract-file>`

默认输出四份最终交付物：

- `接口测试要点.md`
- `接口测试用例.xlsx`
- `待确认项.md`
- `审核评分报告.md`

只有在用户明确要求查看中间结构化结果时，才加 `--keep-json` 保留中间 JSON。

如果用户只要部分接口，不要全量生成，直接使用以下筛选参数：

- `--tag <模块名>`：只生成指定 tag/模块
- `--interface-name <接口名关键字>`：只生成指定接口中文名相关接口
- `--path-keyword <path关键字>`：只生成指定 path 相关接口

只要筛选后的接口依赖公共前置接口，例如 `/user/register`、`/user/login`、`/owner/login`、`/verificationCode/message`，生成时必须自动把这些公共接口一起带入交付物。不能只生成业务接口而漏掉注册、登录、获取验证码。

如果用户要求“按模块分别出文档”，直接使用：

- `--split-by-tag`

样例见：

- [references/input-output-examples.md](references/input-output-examples.md)

## 工作流

### 1. 识别输入类型

支持以下输入：

- Swagger 2.0 JSON
- OpenAPI 3.x JSON / YAML
- Apifox / Knife4j / YApi 导出文本
- 用户整理的纯文本接口契约

默认先运行：

- `scripts/normalize_contract.py`

归一化时要同时提取：

- 基础信息：title、version、base URL
- 公共规则：鉴权方式、通用请求头、通用响应结构、返回码说明
- 接口结构：method、path、summary、tag、path/query/header/body 参数、响应模型
- 约束信息：required、enum、min/max、minLength/maxLength、pattern、格式提示
- 依赖与状态：上游资源 ID、审核/绑定/入住/发布等状态流转特征

### 2. 生成接口测试用例

默认通过：

- `scripts/generate_deliverables.py`

生成用例时严格只使用以下方法：

- 等价类 + 边界值：单参数校验
- 场景法：主流程和依赖流程
- 错误推测：鉴权缺失、参数错误、资源不存在、上传异常
- 状态流转：审核、绑定、入住、发布、重复操作、非法状态

详细规则见：

- [references/case-design-rules.md](references/case-design-rules.md)
- [references/output-template.md](references/output-template.md)

对注册、登录、获取验证码这类公共接口，默认也要生成正向、异常、边界用例。如果用户只指定生成某个需要 token 的业务接口，也要把它依赖的注册/登录/验证码接口一并补齐。

输出顺序默认遵循接口文档顺序；如果自动补入了注册、登录、获取验证码这类公共前置接口，则要把这些接口排在其依赖的业务接口之前，而不是追加到文档末尾。

### 3. 生成待确认项

如果契约没有给出明确规则，不要猜。

待确认项至少识别这些情况：

- 成功码或失败码未明确
- 返回码文档只给链接，未给实际内容
- 鉴权说明模糊，无法判断是否必须带 token
- 出现审核/绑定/入住/发布等动作，但状态迁移规则未定义
- 字段描述有“长度”“范围”“格式”提示，但未给完整约束

### 4. 二次检查与评分

默认使用：

- `scripts/review_testcases.py`

审核重点：

- path、headers、body 是否与契约一致
- 正向、异常、边界/等价类是否覆盖
- 流程/状态场景是否覆盖
- 断言是否具体、可执行、可观察
- 是否出现幻觉字段、幻觉状态码、幻觉业务规则

评分规则见：

- [references/review-rubric.md](references/review-rubric.md)

## 输出要求

### 正式用例模板

必须使用以下固定字段，顺序不可调整：

- 接口中文名称
- 前置条件
- 测试场景
- path
- headers
- body
- 预期响应结果
- 断言响应
- 断言sql

不要额外增加列，除非用户明确要求。

### 断言要求

断言必须可执行，避免：

- `返回成功`
- `提示正常`
- `处理正确`

优先使用：

- `http_status == 200`
- `$.code == "0"`
- `$.msg contains "成功"`
- `$.data != null`
- `$.data.id != null`

### 特殊处理

- `GET` 接口默认 `body` 为空字符串
- 上传接口按 `form-data` 处理，不要强行写 JSON body
- 对登录/验证码等公共接口，优先按无 token 场景生成
- 如果用户明确要求统一使用 `lemonban.v3`，则业务接口统一按 `X-Lemonban-Media-Type=lemonban.v3 + Authorization + timestamp + sign` 生成
- 如果用户明确要求“包括获取 token”，则在前置条件中明确写出先调用登录接口获取 token，再生成 `timestamp/sign`

## 推荐脚本

### `scripts/generate_deliverables.py`

一键入口。读取契约文件，直接输出四份最终交付物。

常见用法：

- 全量生成：
  `scripts/generate_deliverables.py <contract-file>`
- 只生成某个模块：
  `scripts/generate_deliverables.py <contract-file> --tag "05-小区相关操作"`
- 只生成某个接口：
  `scripts/generate_deliverables.py <contract-file> --interface-name "登录"`
- 只生成某类 path：
  `scripts/generate_deliverables.py <contract-file> --path-keyword "/community"`
- 按模块拆分输出：
  `scripts/generate_deliverables.py <contract-file> --split-by-tag`

### `scripts/normalize_contract.py`

将不同格式契约归一化成统一 JSON，便于检查和复用。

### `scripts/review_testcases.py`

读取归一化契约和用例 JSON，输出问题清单、覆盖统计、评分表和最终结论。

### `scripts/render_testcases_xlsx.py`

将结构化用例 JSON 渲染成固定模板 `.xlsx`。
