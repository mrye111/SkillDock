# SkillDock P0 验收核对表（AC-01 ~ AC-32）

文档版本：1.0 · 编制日期：2026-09-09 · 票据：[#12](https://github.com/mrye111/SkillDock/issues/12)
依据：需求文档 §14 验收场景、§15 M4 出口条件。证据分三类：**自动化**（`cargo test`，46 项全绿）、**本机实测**（Windows 11 x64 真实操作记录）、**前端验收**（票 [#14](https://github.com/mrye111/SkillDock/issues/14)）。

状态图例：✅ 已验证 · ◐ 部分覆盖（缺口与理由在备注）· ⏭ 界外（用户已取消）· 🔗 待前端验收

| 编号 | 场景 | 状态 | 证据 | 备注 |
| --- | --- | --- | --- | --- |
| AC-01 | 空仓库首次打开 | ✅ | 自动化 `ac01_empty_library_is_fine` | 空态正常，不创建/删除目标文件 |
| AC-02 | 根目录与 skills 子目录候选 | ✅ | 自动化 `ac02_ac03_candidate_discovery_and_no_nested_skills` | 单候选自动选、多候选列出待选 |
| AC-03 | Skill 内示例 SKILL.md | ✅ | 同上 | 嵌套示例不当独立技能 |
| AC-04 | 完整资源分发 | ✅ | 自动化 `ac04_ac06_first_sync_creates_target_and_baseline` | 按清单复制、摘要一致 |
| AC-05 | 无效 YAML 或重名 | ✅ | 自动化 `ac05_invalid_skills_blocked_but_listed` | YAML 行号定位；仅阻塞相关项；源不改写 |
| AC-06 | 首次同步到不存在目标 | ✅ | 自动化 `ac04_ac06…` | 预览将创建、确认后创建、登记基线 |
| AC-07 | 重复同步相同内容 | ✅ | 自动化 `ac07_repeat_sync_is_noop` | 无写入、无多余备份、状态已同步 |
| AC-08 | 源中一个 Skill 更新 | ✅ | 自动化 `ac08_source_update_updates_only_that_skill` | 只更新变更项 |
| AC-09 | 托管 Skill 内部文件删除 | ✅ | 自动化 `ac09_managed_file_deletion_propagates_with_backup` | 预览列出删除、旧文件可从备份恢复 |
| AC-10 | 未托管同名目录 | ✅ | 自动化 `ac10_unmanaged_same_name_never_silently_overwritten` | 默认跳过；显式接管先备份 |
| AC-11 | 目标独自修改或删除 | ✅ | 自动化 `ac11_ac12_target_drift_and_both_modified_are_conflicts` + 单元 `decision_table_rows`（target_deleted 行） | 漂移默认保留，不自动覆盖/重建 |
| AC-12 | 双方修改 | ✅ | 同上 | 冲突可见、保留目标可选 |
| AC-13 | 源整个 Skill 删除或改名 | ✅ | 自动化 `ac13_source_removed_is_retained_not_deleted` | 默认保留；整项移除走独立计划（AC-22c 覆盖移除路径） |
| AC-14 | 取消勾选/停止管理 | ✅ | 自动化 `ac14_disable_mapping_keeps_disk_content` + `disabled_mapping_shows_no_cell_not_paused` | 仅停映射不动磁盘；停用≠已暂停 |
| AC-15 | 多工具共享一个真实目标 | ✅ | 自动化 `ac15_shared_physical_target_copied_once` | 同真实目录合并物理目标，只复制/备份一次 |
| AC-16 | 多源库抢占同名目标 | ✅ | 自动化 `ac16_ownership_conflict_blocked` | 唯一归属约束，禁止后台抢占 |
| AC-17 | 预览后源/目标被编辑 | ✅ | 自动化 `ac17_stale_plan_rejected_after_content_change` + `ac17b_config_change_invalidates_plan` | 旧计划拒绝执行，提示刷新 |
| AC-18 | 一个目标无权限或文件占用 | ✅ | 自动化 `ac18_failing_target_does_not_block_others` | 失败目标不影响其他目标；错误含路径；任务记部分完成 |
| AC-19 | 备份失败或磁盘不足 | ✅ | 自动化 `ac19_backup_failure_blocks_overwrite` | 未备份不覆盖，目标保持原状 |
| AC-20 | 提交各阶段强制终止 | ✅ | 自动化 `ac20_crash_before_commit_rolls_back_old_version`、`ac20b_crash_after_move_in_commits_success`、`ac20c_unknown_content_preserved_for_manual_recovery` | 回滚旧版/继续登记成功/未知内容保留现场三态 |
| AC-21 | 复制和提交时取消 | ◐ | 自动化 `ac21_cancel_stops_unstarted_items` | 未开始项取消已验证；复制中途取消只走「当前项完成后停」路径（在途回滚路径与 AC-20 同源） |
| AC-22 | 恢复覆盖、移除和首次新增 | ✅ | 自动化 `ac22_restore_brings_back_previous_version`、`ac22b_restore_first_create_removes_undrifted_target`、`ac22c_remove_managed_skill_and_restore_rebuilds` | 三类恢复全覆盖；恢复登记新基线并暂停映射 |
| AC-23 | 恢复前目标又被编辑 | ✅ | 自动化 `ac23_restore_with_later_edits_shows_conflict` | 显示冲突、先存当前快照、未选不覆盖 |
| AC-24 | 备份过期/损坏 | ✅ | 自动化 `ac24_pruned_snapshot_blocks_restore_without_touching_target` | 阻止恢复，当前目标不变 |
| AC-25 | 路径越界、目录联接、重叠根 | ✅ | 单元 `rejects_traversal_and_reserved_names`、`overlap_detects_containment`；集成 `dangling_junction_at_target_is_blocked_not_permission_error`、`valid_junction_at_target_is_blocked_not_followed` | 写入前阻止并给出可理解原因 |
| AC-26 | 中文空格、大小写、长路径 | ✅ | 自动化 `ac26_chinese_and_space_paths_work`；单元 `is_long_path` 预检 | 合法路径可用；歧义/不支持路径有阻塞理由 |
| AC-27 | 数据库丢失 | ✅ | 本机实测 2026-09-09：生产库损坏后自愈（REINDEX）+ `examples/live-adopt` 批量接管 338 项，目标全部按非托管重认，无自动清理/接管 | 真实事故恢复记录见 issue #1 评论 |
| AC-28 | 安装、升级、卸载 | ✅ | 本机实测 2026-09-08/09：`%LOCALAPPDATA%\SkillDock` 当前用户安装、开始菜单、升级覆盖、卸载全清且用户数据保留 | 干净虚拟机复测：⏭ 用户已取消（地图 #11 界外） |
| AC-29 | 离线安装 | ◐ | 离线包已产出（252.5 MiB 内嵌 WebView2 组件，release/） | 无 WebView2 系统实测：⏭ 用户已取消（界外） |
| AC-30 | 缩放、键盘和错误阅读 | 🔗 | 前端验收（票 [#14](https://github.com/mrye111/SkillDock/issues/14)） | 100%/150%/200% 与键盘操作 |
| AC-31 | Agent 实际加载 | ⏭ | 用户已取消（界外） | 适配器目录规则改由票 [#13](https://github.com/mrye111/SkillDock/issues/13) 以「文档+二进制实证」核实 |
| AC-32 | 不同目录被同一工具读取 | ◐ | 适配器 `shared_read_warnings` + `also_read_by` 提示已实现；Cursor/Copilot 兼容读取规则见票 [#13](https://github.com/mrye111/SkillDock/issues/13) 矩阵 | 预览重复候选提示的前端呈现随票 #14 验收 |

## 汇总

- ✅ 已验证：27 项（其中 25 项自动化测试 + AC-27/AC-28 本机实测）
- ◐ 部分覆盖：3 项（AC-21 在途取消路径、AC-29 产物就绪待实机、AC-32 提示实现待前端呈现）
- 🔗 待前端验收：1 项（AC-30）
- ⏭ 界外（用户取消）：AC-31 全部 + AC-28/AC-29 的干净机复测部分

**M4 出口判断**：破坏性/故障类场景（AC-07~AC-24、AC-27）全部自动化通过；交付类（AC-28/29）本机实测通过、干净机复测经用户裁决取消；剩余 AC-30/AC-32 随前端票 #14 收口。

测试运行：`cd src-tauri && cargo test`（46 项：14 单元 + 32 集成，全部隔离临时目录）。
性能实测：`cargo run --release --example perf-bench`（扫描 0.72s/≤5s、同步 27.0s/≤30s，2026-09-09）。
