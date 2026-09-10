## Resolution（前端会话完成，后端会话验收后关闭）

前端会话汇报完成，后端验收通过（`npm run build` 编译通过 + 四项关键改动逐项核对）：

1. **v1.4 新能力接入**：`resolve_conflicts_bulk` 批量冲突解决已接入（AppContext 调用 + Mock 层实现）；设置页「打开日志目录」按钮走 `open_registered_path` kind=`log_dir`。
2. **批量关联保留已暂停映射**：`batchAssignTarget` 的 currentTargets 改为「mappingId 存在且非 no_mapping 即保留」——暂停的关联不再被批量操作误停用（语义注释清楚）。
3. **复选框部分关联态**：表头 checkbox indeterminate 状态就位。
4. **空态/错误态**：新增 ErrorBoundary 组件与视觉打磨（RibbonBackdrop、logo 资产等）。

遗留说明（不阻塞）：AC-30 的 100%/150%/200% 缩放实机走查建议随 #17 发布后用户实测反馈迭代。
