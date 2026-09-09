## Destination

SkillDock P0 发布就绪（需求文档 §15 M4 出口条件满足）：P0 验收逐项有据（AC-01~AC-32 核对表）、适配器实测版本矩阵齐全、前端衔接收尾、发布物完整（双 EXE + 校验值 + 版本说明 + 许可说明 + 用户指南 + 安装/卸载测试记录）。

## Notes

- 前图：[SkillDock 后端 P0 路线图](https://github.com/mrye111/SkillDock/issues/1) 已走完（#2–#10 全关闭 + §13 性能实测达标）；本图只管发布收口。
- 规格基线：docs/SkillDock-需求与技术设计.md v1.3（§12.3 发布流程、§14 验收场景、§15 M4 出口、§17 交付清单）。
- 当前事实：契约 v1.4；44 个 Rust 测试全绿；release/ 下双 EXE（标准 2.7 MiB / 离线 252.5 MiB）+ SHA256SUMS；Codex 适配器已按 codex-cli 0.147.0 实测校正；前后端代码均已入库。
- 多智能体分工：「前端衔接收尾」票由前端会话认领；其余票后端会话执行。契约变更先改 docs/api-contract.md + src/lib/backend-contract.ts 并通知前端。
- 文件保护规则与测试隔离规则沿用前图 Notes（不静默覆盖、不自动删除、计划失效重生成、备份失败不覆盖；测试只用临时目录）。

## Decisions so far

<!-- 索引：每关闭一张票，在此追加一行 gist + 链接 -->

- [P0 验收核对表：AC-01~AC-32 逐项证据与缺口处置](https://github.com/mrye111/SkillDock/issues/12) — docs/acceptance-checklist.md：27 项已验证、3 项部分覆盖、1 项待前端、界外已注明；补缺 AC-18/AC-22c 测试并修复移除计划复核 bug

- [适配器实测版本矩阵：Claude Code / Cursor / Copilot 核实](https://github.com/mrye111/SkillDock/issues/13) — docs/adapter-matrix.md：三款实证（2.1.141 / 3.13.25 / 1.120.0+0.48.1）；Copilot .agents 改版本限定；核实日期 2026-09-09

## Not yet specified

- Windows 10 22H2 兼容实测（§1 兼容测试项）：需要 Win10 环境，当前机器为 Win11。
- VS Code 更新切换（new_Code.exe）后复核 Copilot 扩展版本与 .agents 默认列表（票 #13 遗留）。
- 公开分发签名（§12.2）：需要代码签名证书决策；P0 阶段使用明确标记的内部测试包即可。
- P1 功能地图（监听/自动同步/托盘/方案/标签/导入导出/深色/便携 ZIP）：P0 发布后另行开图。

## Out of scope

- AC-28 干净虚拟机复测、AC-29 无 WebView2 离线包实测、AC-31 真实 Agent 加载验证——用户已明确取消（2026-09-09），如重启需另开新努力。
- P1/P2 功能（需求 §3.2/§3.3）：跟随上一条雾区，不属于本图。


