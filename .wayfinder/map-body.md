## Destination

SkillDock 后端 P0 完成并通过核心验收：跑通「扫描 → 预览 → 同步到自定义目录 → 校验结果」闭环，补齐适配器、冲突、备份与恢复，提供稳定的前后端接口契约（`docs/api-contract.md` + `src/lib/backend-contract.ts`），并具备 Windows 打包配置，最终与前端联调产出 NSIS 安装 EXE。

## Notes

- 领域：Windows 桌面应用后端，Tauri 2 + Rust + SQLite（rusqlite），React/TS 前端由另一会话负责。
- 规格基线：[docs/SkillDock-需求与技术设计.md](https://github.com/mrye111/SkillDock/blob/main/docs/SkillDock-%E9%9C%80%E6%B1%82%E4%B8%8E%E6%8A%80%E6%9C%AF%E8%AE%BE%E8%AE%A1.md) —— §4 概念与扫描规则、§5 适配器、§8 同步语义与事务、§10.4 接口约定、§11 数据存储为权威定义。
- **执行模式**：用户明确指示本工作携带执行（覆盖 wayfinder 默认的"只规划"），按「接口契约 → 扫描/存储 → 计划 → 执行 → 恢复 → 接线 → 打包」顺序逐票推进并实际落码验证。
- 文件保护规则（需求 §8，不可妥协）：不静默覆盖非托管内容；不自动删除源已移除的整项技能；预览后内容变化必须重新生成计划；备份失败不得继续覆盖。
- 多智能体边界：本会话只修改 `src-tauri/`、`docs/api-contract.md`、`src/lib/backend-contract.ts`、`tests/`、根级工程配置；不动前端页面与样式。
- 所有文件操作测试使用隔离临时目录（`tests/` + tempfile），不得触碰真实 Agent 技能目录。

## Decisions so far

<!-- 索引：每关闭一张票，在此追加一行 gist + 链接 -->

## Not yet specified

- 真实 Agent 加载验证矩阵（AC-31）：需要各工具实测版本与作用域，发版前锁定。
- 性能实测与调优（§13：200 Skills / 5000 文件扫描 ≤5s 等）：待核心闭环完成后实测。
- 离线安装包（内嵌 WebView2 引导）体积与实测：打包配置完成后评估。
- 与前端会话的联调节奏：契约冻结时间点、事件订阅方式的联调清单。

## Out of scope

- P1/P2 功能：文件监听、自动同步、托盘、同步方案、深色主题、便携 ZIP、更多适配器、CLI 入口（需求 §3.2/§3.3）。
- 云端同步、账号体系、技能市场等（需求 §3.4）。
- 前端页面与样式实现 —— 由前端会话负责，本图只管后端与接口。
