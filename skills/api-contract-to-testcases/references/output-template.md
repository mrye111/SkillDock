# 接口测试用例输出模板

## 固定字段

生成的接口测试用例必须使用以下字段，顺序不可调整：

| 接口中文名称 | 前置条件 | 测试场景 | path | headers | body | 预期响应结果 | 断言响应 | 断言sql |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |

## 字段填写规则

### 1. 接口中文名称

- 取接口契约中的中文名称、`summary` 或最接近业务语义的标题。
- 不要自行重命名为模糊描述，例如“接口1”“创建接口”。

### 2. 前置条件

- 写清本条用例执行前必须准备的数据、账号、token 或上游资源。
- 如果接口统一走鉴权链路，需要明确写出“先调用登录接口获取 token”。
- 对 `lemonban.v3` 鉴权，前置条件中要明确 token、timestamp、sign 的准备方式。
- 推荐写成分步编号换行格式，避免一整句堆叠。例如：

```text
1、先调用登录接口获取 token；基于 token 生成 timestamp 和 sign。
2、headers 使用 X-Lemonban-Media-Type=lemonban.v3、Authorization=Bearer ${token}、timestamp、sign。
3、并准备有效的上游资源数据：communityId、userId。
```

### 3. 测试场景

- 需要包含优先级和场景类型。
- 推荐格式：
  - `P0_正向_注册成功`
  - `P1_异常_手机号缺失`
  - `P1_边界_用户名长度最小值`
  - `P1_状态流转_重复审核拦截`

### 4. path

- 写接口相对路径。
- 如果是 path 参数场景，展开为本条用例的实际值。
- 如果是 query 参数场景，可直接写完整相对路径，例如：
  - `/verificationCode/message?phone=18600000000`

### 5. headers

- 必须写成 JSON 对象字符串。
- 只填写本条用例实际需要的头。
- `lemonban.v3` 示例：

```json
{"X-Lemonban-Media-Type":"lemonban.v3","Authorization":"Bearer ${token}","timestamp":"${timestamp}","sign":"${sign}","Content-Type":"application/json"}
```

### 6. body

- `POST`、`PUT`、`PATCH` 的 JSON 接口：写 JSON 对象字符串。
- `GET` 接口：填空字符串。
- `form-data` 文件上传：写成结构化说明字符串或 JSON 说明对象，例如：

```json
{"file":"test.jpg"}
```

### 7. 预期响应结果

- 用自然语言写清接口行为结果。
- 必须能与断言响应一一对应。
- 不要只写“返回成功”“处理正常”。

### 8. 断言响应

- 必须可执行、可观察。
- 多个断言使用分号分隔。
- 推荐断言格式：
  - `http_status == 200`
  - `$.code == "0"`
  - `$.msg contains "成功"`
  - `$.data != null`
  - `$.data.id != null`
  - `$.data.records size > 0`

### 9. 断言sql

- 默认留空字符串。
- 在没有明确 SQL 校验规则前，不要虚构数据库断言。

## 输出原则

- 一条用例只覆盖一个明确测试目标。
- 不要把多个独立失败原因塞进同一条用例。
- 不要生成契约文档未说明的数据库、缓存、消息队列断言。
