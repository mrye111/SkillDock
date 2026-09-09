## 契约变更通知 v1.4（2026-09-09）：批量冲突解决 + 操作日志

**背景（用户实测反馈）**：① 120+ 个「可接管」项逐个点冲突抽屉不可行；② 重装后历史记录看似丢失（实为数据库损坏 + 沙箱副本数据目录分裂）。

**新增命令** `resolve_conflicts_bulk({ planId, planVersion, kinds, choice })` → `{ plan, outcome: { applied, skipped } }`：对指定冲突类型的未决项批量应用同一选择（如 `kinds: ['same_content']` + `adopt_existing` = 批量接管全部内容一致项），整批只递增一次 planVersion。建议 UI：预览冲突区加「全部接管内容一致项」「全部保留目标」批量按钮。用户数据实测：Cursor 目标 120/123 项与源逐字节一致，可零风险批量接管。

**操作日志**：每个任务完成时追加一行 JSON 到「文档\SkillDock\SkillDock-操作日志.jsonl」（append-only、人类可读、独立于数据库）。设置页可加「打开日志目录」按钮：`open_registered_path({ kind: 'log_dir', id: '' })`。

**健壮性修复（无需前端改动）**：启动时数据库完整性自检 → REINDEX 修复 → 不可修复则隔离重建（不再在损坏库上裸奔导致查询报错）；迁移备份前先收拢 WAL；任务完成与应用退出时检查点收拢。
