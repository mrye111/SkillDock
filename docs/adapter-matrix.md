# 适配器实测版本矩阵

核实日期：**2026-09-09** · 核实方式：**本机实证（二进制/包字符串提取）+ 官方文档双重核实** · 依据：需求 §5（适配器表）、§16.1（发版锁定测试矩阵：核实日期 + 官方链接 + 验证版本）

本机环境：Windows 11 Pro（10.0.26200），用户目录 `C:\Users\23167`，Git Bash。Codex 已于 2026-09-08 实测（codex-cli 0.147.0），本轮不重复核实；本矩阵覆盖其余三款。

---

## 1. Claude Code

| 项 | 内容 |
| --- | --- |
| 实测版本 | **2.1.141**（npm 全局包 `@anthropic-ai/claude-code`，位于 `D:\node\node_global\node_modules`；`claude --version` 报 `2.1.141 (Claude Code)`；本体为原生二进制 `bin/claude.exe`，约 228 MB） |
| 用户级目录 | `~/.claude/skills/<name>/SKILL.md`；设置 `CLAUDE_CONFIG_DIR` 时为 `<CLAUDE_CONFIG_DIR>/skills/` |
| 项目级目录 | `<项目>/.claude/skills/<name>/SKILL.md` |
| 兼容读取 | 不读取其他 Agent 的目录；插件技能走 `<plugin>/skills/`，属插件机制而非兼容目录 |
| 官方文档 | https://code.claude.com/docs/en/skills （2026-09-09 访问） |

**本机实证**（`claude.exe` 字符串提取）：

- 配置目录解析：`process.env.CLAUDE_CONFIG_DIR ?? path.join(os.homedir(), ".claude")` —— 证实 `CLAUDE_CONFIG_DIR` 覆盖逻辑与注册表 `env_override` 一致。
- 技能加载位置：`join(configDir, ".claude"→configDir, "skills")` 与 `join(homedir, ".claude", "skills")`；帮助字符串：「**Personal** (`~/.claude/skills/<name>/SKILL.md`) — follows you across all repos」「**This repo** (`.claude/skills/<name>/SKILL.md`)」。
- 本机 `CLAUDE_CONFIG_DIR` 未设置，即用户级实际落在 `C:\Users\23167\.claude\skills`（目录存在且有内容）。

**官方文档佐证**：个人技能 `~/.claude/skills/<name>/SKILL.md`、项目技能 `.claude/skills/<name>/SKILL.md`；另有企业策略目录、嵌套目录、`--add-dir` 附加目录、插件、claude.ai 同步（`~/.claude/skills/synced/`）等变体——均属 §5 约定不管理的企业/内置/插件范畴。文档页本身未提 `CLAUDE_CONFIG_DIR`（该变量见配置目录文档），以二进制实证为准。

**与注册表（`src-tauri/src/adapters/mod.rs`）一致性：一致**，无需修正。

## 2. Cursor

| 项 | 内容 |
| --- | --- |
| 实测版本 | **3.13.25**（`%LOCALAPPDATA%\Programs\cursor\resources\app\package.json`） |
| 用户级目录 | `~/.cursor/skills/`；兼容读取 `~/.agents/skills/`、`~/.codex/skills/`、`~/.claude/skills/` |
| 项目级目录 | `<项目>/.cursor/skills/`；兼容读取 `.agents/skills/`、`.codex/skills/`、`.claude/skills/` |
| 官方文档 | https://prod.cursor.com/docs/skills （注册表现有链接，2026-09-09 访问有效）；https://cursor.com/docs/skills 内容相同 |

**本机实证**（`resources/app/out/vs/workbench/workbench.desktop.main.js` 字符串提取）：

- 路径分类函数（原文）：`includes("/.cursor/skills-cursor/") → "builtin"`；`includes("/.cursor/plugins/") || includes("/.claude/plugins/") → "plugin"`；家目录正则 `/^\/(?:Users|home)\/[^/]+\/\.(?:cursor|agents|codex|claude)\/skills\//` 及 Windows 形式 `/^[A-Za-z]:\/Users\/[^/]+\/\.(?:cursor|agents|codex|claude)\/skills\//i → "user"`；包含或以 `.cursor/skills/`、`.agents/skills/`、`.codex/skills/`、`.claude/skills/` 开头 → `"workspace"`。
- 识别的技能子目录列表（原文数组）：`[".cursor/skills/", ".cursor/skills-cursor/", ".cursor/cloud-skills/", ".cursor/plugins/", ".claude/skills/", ".claude/plugins/", ".codex/skills/", ".agents/skills/"]`。
- 本机 `C:\Users\23167\.cursor\skills\` 存在且有内容。

即：四个技能目录（cursor/agents/codex/claude）在**用户级与项目级均被读取**；`.cursor/skills-cursor/`（内置）、`.cursor/plugins/` 与 `.claude/plugins/`（插件）、`.cursor/cloud-skills/`（云同步）为工具自有位置。

**官方文档佐证**：项目级 `.agents/skills/` + `.cursor/skills/`；用户级 `~/.agents/skills/` + `~/.cursor/skills/`；另为兼容读取 `.claude/skills/`、`.codex/skills/`（用户级同理）；`SKILL.md` 递归发现、嵌套目录按子目录作用域生效；仅 `~/.cursor/skills/` 可同步到 Cloud Agents。

**与注册表一致性：一致**。`notes` 中「Cursor 还会读取 .agents、.claude、.codex 下的兼容技能目录，可能存在重复发现」经二进制与文档双重确认。可选补充（非必须）：内置 `skills-cursor/`、插件与云同步目录属 §5 末段约定不管理的内置/插件缓存，无需写入注册表。

## 3. GitHub Copilot（VS Code）

| 项 | 内容 |
| --- | --- |
| 实测版本 | VS Code **1.120.0**（`%LOCALAPPDATA%\Programs\Microsoft VS Code`，构建日期 2026-05-12，commit `0958016b2a`；注意：安装目录处于更新待切换状态，当前仅有 `new_Code.exe`，版本取暂存的 `resources/app/package.json`）+ Copilot Chat 扩展 **0.48.1**（`~/.vscode/extensions/`，另存旧版 0.40.1，VS Code 取最新 0.48.1） |
| 用户级目录 | `~/.copilot/skills/`；兼容读取 `~/.claude/skills/` |
| 项目级目录 | `<项目>/.github/skills/`；兼容读取 `.claude/skills/` |
| 官方文档 | https://code.visualstudio.com/docs/agent-customization/agent-skills （2026-09-09 访问） |

**本机实证**（`github.copilot-chat-0.48.1/dist/extension.js` 字符串提取；0.40.1 结果相同）：

- 默认目录数组（原文）：用户级 `[".copilot/skills",".claude/skills"]`（对 `userHome` 逐一拼接，标记为 `personal`）；工作区 `[".github/skills",".claude/skills"]`（对每个 workspace folder 拼接）。
- 相关设置键：`chat.agentSkillsLocations`（附加技能位置）、`chat.useAgentSkills`（内部开关，0.48.1 未在 package.json 中声明）。
- **未发现 `.agents/skills` 默认目录**：二进制中 `.agents` 字样均指子代理（`agentsDir` 等），与技能目录无关。

**官方文档佐证**：工作区 `.github/skills/`、`.claude/skills/`、**`.agents/skills/`**；用户级 `~/.copilot/skills/`、`~/.claude/skills/`、**`~/.agents/skills/`**；设置 `chat.agentSkillsLocations`（配置附加项目技能位置）、`github.copilot.chat.skillTool.enabled`（子代理上下文运行技能）、`chat.useCustomizationsInParentRepositories`（monorepo 父仓库发现）。

**与注册表一致性：模板一致，备注有一处出入**。

- 一致：`user_template = ~/.copilot/skills`、`project_template = .github/skills` 与实测、文档均符；`.claude/skills` 兼容读取经二进制与文档双重确认。
- **出入**：`also_read_by` 写「VS Code 也支持用户/项目级 .claude 与 .agents 技能目录」——其中 `.agents` 仅见诸官方文档（面向最新版本），**本机 Copilot Chat 0.48.1（及 0.40.1）默认列表未包含 `.agents/skills`**，疑为更新版本新增。
- **建议修正**：将备注改为带版本限定的表述，例如：「VS Code 也读取用户/项目级 .claude/skills（二进制+文档确认）；官方文档另声明 .agents/skills，本机 Copilot Chat 0.48.1 默认未含，按版本差异处理」。

---

## 汇总表

| 工具 | 实测版本 | 用户级目录 | 项目级目录 | 兼容读取（实证） | 官方文档（2026-09-09 访问） | 与注册表 |
| --- | --- | --- | --- | --- | --- | --- |
| Claude Code | 2.1.141 | `~/.claude/skills/`（`CLAUDE_CONFIG_DIR` 可覆盖） | `.claude/skills/` | 无 | https://code.claude.com/docs/en/skills | **一致** |
| Cursor | 3.13.25 | `~/.cursor/skills/` | `.cursor/skills/` | 用户级+项目级均读 `.agents`、`.codex`、`.claude` 的 `skills/` | https://prod.cursor.com/docs/skills | **一致** |
| GitHub Copilot (VS Code) | VS Code 1.120.0 + Copilot Chat 0.48.1 | `~/.copilot/skills/` | `.github/skills/` | 用户级+项目级 `.claude/skills/`；文档另声明 `.agents/skills`（0.48.1 未含） | https://code.visualstudio.com/docs/agent-customization/agent-skills | **模板一致；`.agents` 备注需加版本限定** |
| Codex（2026-09-08 已实测） | codex-cli 0.147.0 | `$CODEX_HOME/skills/`（默认 `~/.codex/skills/`） | `.codex/skills/` | — | https://developers.openai.com/codex/skills | 一致（本轮未复核） |

## 修正建议汇总

1. **（建议）** `copilot_vscode.also_read_by`：`.agents` 一项补充「官方文档声明、本机 Copilot Chat 0.48.1 默认未含」的版本限定；`.claude` 部分保持不变。
2. **（建议）** 采纳本矩阵后将 `ADAPTER_VERSION` 从 `2026-09-08` 更新为 `2026-09-09`（三款均已完成本机实测）。
3. **（可选）** `cursor.notes` 可补一句内置/插件目录（`.cursor/skills-cursor/`、`.cursor/plugins/`、`.claude/plugins/`、`.cursor/cloud-skills/`）存在但不管理，与 §5 末段口径一致；不补充也不影响正确性。
4. 本机 VS Code 安装处于更新待切换状态（仅 `new_Code.exe`），版本号 1.120.0 取自暂存包；重启完成切换后建议复核一次 Copilot Chat 扩展版本与 `.agents/skills` 默认列表。
