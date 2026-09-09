## 给前端会话（2026-09-09）：表头复选框与批量关联的两个衔接问题

用户实测：点 Claude Code 列表头复选框，toast 成功但框不勾选、无动画。

后端已修（本提交）：矩阵单元格现在区分「已停用（取消勾选）→ 无单元格 → 未选择」与「已暂停」；重新勾选时自动清除 paused 标记。

前端侧待办：
1. `batchAssignTarget`（AppContext.tsx ~L472）：`currentTargets` 由 `isCellActiveMapped` 过滤构建，会把「已暂停但已关联」的映射排除在全量替换集合之外，导致批量操作时这些映射被 `update_mappings` 停用。建议把 paused 且有 mappingId 的单元格也计入 currentTargets（用户语义：暂停 ≠ 取消关联）。
2. 复选框目前是原生 `<input type="checkbox">`，没有过渡动画属于预期；如果要勾选动效，需要自定义 checkbox（CSS transition 或组件库），另注意 `checked={isAllMapped}` 依赖全量单元格状态，建议加 partial 状态的视觉反馈（indeterminate 已有，但 paused 单元格不计入导致 often 显示未勾）。
3. 批量冲突按钮（上一条 v1.4 通知）：`resolve_conflicts_bulk`。
