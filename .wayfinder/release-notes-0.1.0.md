# SkillDock 0.1.0（P0 内部测试包）

**一处维护，随处可用。** SkillDock 把本地维护的 AI 技能库（含 `SKILL.md` 的技能文件夹）安全地同步到多个 AI 编程工具的技能目录：Codex、Claude Code、Cursor、GitHub Copilot（VS Code），或任意自定义目录。

> ⚠️ 本版本为 **P0 内部测试包**：功能完整但按内部验证用途分发，未做代码签名，可能出现系统信誉提示；公开分发版本将另行签名发布。

## 下载

| 文件 | 说明 |
| --- | --- |
| `SkillDock-0.1.0-windows-x64-setup.exe` | 标准安装包（2.7 MiB；WebView2 缺失时引导安装） |
| `SkillDock-0.1.0-windows-x64-offline-setup.exe` | 离线安装包（252.5 MiB；内嵌 WebView2 安装组件，无网络可装） |
| `SHA256SUMS.txt` | 两个安装包的 SHA-256 校验值 |
| `THIRD-PARTY-NOTICES.md` | 第三方依赖许可说明（246 Rust crates + 199 npm 包） |

当前用户安装到 `%LOCALAPPDATA%\SkillDock`，免管理员权限；卸载默认保留配置与历史记录（也可在卸载时勾选清除）。

## 本版包含（P0 完整能力）

- **技能库管理**：候选源根自动发现（根目录 / `skills/` / `.agents/skills/` / `.claude/skills/` / `.codex/skills/`）、单技能登记、最近打开
- **扫描与校验**：SKILL.md 元数据校验（YAML 行号定位）、SHA-256 内容清单与技能摘要、无效项逐项标注
- **同步矩阵**：技能 × 目标真实状态（已同步/待更新/待新增/冲突/源已移除/可接管等 15 态）
- **同步预览**：按物理目标分组的操作清单、逐文件变化（含文件删除）、冲突与阻塞项、计划指纹失效保护
- **冲突处理**：保留目标 / 备份后覆盖 / 接管现有目录 / 接管并覆盖 / 暂停映射，支持**批量接管**同类冲突
- **安全执行**：单技能事务（暂存→校验→备份→复核→改名替换→提交基线）、全程持久化日志、取消、部分失败隔离、最多 2 目标并行
- **备份与恢复**：修改前可验证快照（备份失败不覆盖）、30 天 / 2 GiB 保留策略、历史单项/批量恢复、崩溃中断自动恢复（未知内容保留现场）
- **数据库自愈**：启动完整性检查 → REINDEX → 不可修复则隔离重建；操作日志独立写入 `文档\SkillDock\SkillDock-操作日志.jsonl`

## 实测数据（2026-09-09，Windows 11 x64，Defender 实时防护开启）

- 自动化测试 **46 项全绿**（14 单元 + 32 集成，全部隔离临时目录）
- 扫描 200 技能 / 5,000 文件 / 92 MiB：**0.72s**（预算 ≤5s）
- 首次同步 92 MiB 到 3 个目标（含校验与备份）：**27.0s**（预算 ≤30s）
- 运行内存：约 80–125 MiB（预算 ≤250 MiB）
- 验收核对：[AC-01~AC-32 逐项证据](https://github.com/mrye111/SkillDock/blob/main/docs/acceptance-checklist.md)

## 适配器版本矩阵（2026-09-09 实证核实）

| 工具 | 实测版本 | 技能目录 |
| --- | --- | --- |
| Codex | codex-cli 0.147.0 | `~/.codex/skills/`（随 `CODEX_HOME`） |
| Claude Code | 2.1.141 | `~/.claude/skills/`（随 `CLAUDE_CONFIG_DIR`） |
| Cursor | 3.13.25 | `~/.cursor/skills/`（兼容读取 `.agents/.codex/.claude`） |
| Copilot (VS Code) | VS Code 1.120.0 + Copilot Chat 0.48.1 | `~/.copilot/skills/`、项目级 `.github/skills/`（另读 `.claude`） |

详见 [docs/adapter-matrix.md](https://github.com/mrye111/SkillDock/blob/main/docs/adapter-matrix.md)。注意：文件同步成功 ≠ 工具已加载，如未显示请按工具说明刷新或开启新会话。

## 已知限制

- 目标为符号链接/目录联接（含悬空链接）时按设计阻止并提示，需手动处理后同步。
- 首版仅本地磁盘目录：UNC/网络映射盘/WSL/未落盘云占位文件不支持。
- 干净虚拟机与无 WebView2 环境的安装实测、真实 Agent 加载验证未纳入本内部测试包验收范围（经产品决策取消）。
- Windows 10 22H2 兼容未实测（主验收环境为 Windows 11 x64）。

## 安装/卸载测试记录（本机，2026-09-08/09）

- `/S` 静默安装 → 程序目录、开始菜单入口创建 ✓
- 启动出主窗口、扫描/同步/恢复/历史流程可用 ✓
- 覆盖升级安装（原位升级、数据保留）✓
- `/S` 静默卸载 → 程序文件/注册表/开始菜单清除，用户数据（app.db）校验值不变 ✓

文档：[用户指南](https://github.com/mrye111/SkillDock/blob/main/docs/user-guide.md) · [需求与技术设计](https://github.com/mrye111/SkillDock/blob/main/docs/SkillDock-%E9%9C%80%E6%B1%82%E4%B8%8E%E6%8A%80%E6%9C%AF%E8%AE%BE%E8%AE%A1.md) · [API 契约](https://github.com/mrye111/SkillDock/blob/main/docs/api-contract.md)
