## Resolution

已定稿并提交（commit `docs: 前后端接口约定 v1.0`）：

- **`docs/api-contract.md`** —— 权威契约：统一错误模型（15 个错误码 + context/retryable/diagnosticId）、核心枚举（矩阵状态 15 态覆盖 §8.2 决策表全部行、任务状态机、计划动作/冲突选择）、§10.4 全部 9 个命令 + 11 个查询/辅助命令的参数与返回值、4 类事件（`scan.progress` / `sync.progress` / `sync.completed` / `recovery.required`，均含 taskId 与全局递增 seq）、8 条契约级不变量（含四条文件保护规则的接口表达）。
- **`src/lib/backend-contract.ts`** —— 前端 TypeScript 镜像：全部类型 + `ContractCommands` 命令签名表 + 类型安全的 `call()` / `onContractEvent()` 封装（依赖 `@tauri-apps/api`，需前端会话安装）。

关键设计取舍：
1. 命令名 snake_case、字段 camelCase、枚举值 snake_case；ID 均为后端生成的字符串。
2. 执行类命令只接受登记对象 ID 与计划 ID + 版本号，实际路径由后端解析——路径授权不经过前端（§10.1/§10.4）。
3. 恢复复用同步计划机制（`operation = 'restore'` + `execute_sync_plan`），冲突以 `restore_drift` 类型走统一 `resolve_conflict` 流程。
4. 计划指纹 = 配置版本 + 逐项（映射 ID、S/T/B 摘要、动作、决策）；展示层设置不影响指纹。
