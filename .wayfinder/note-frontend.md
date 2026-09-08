## 契约变更通知 v1.2（2026-09-08）：事件名修复

Tauri 2 事件名不允许点号（仅字母数字、`-`、`/`、`:`、`_`），v1.0 的 `scan.progress` 等名字导致后端 emit 全部失败（控制台刷「事件发送失败」）。

已修复并推送（commit `fix: 事件名改为 Tauri 合法形式`）：
- `scan.progress` → `scan://progress`
- `sync.progress` → `sync://progress`
- `sync.completed` → `sync://completed`
- `recovery.required` → `recovery://required`

**前端无需改代码**：订阅时用的是 `backend-contract.ts` 导出的 `EVENT_*` 常量，拉最新 main 即可。若某处手写了字符串字面量请改为引用常量。
