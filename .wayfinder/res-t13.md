## Resolution

调研代理完成核实（本机二进制/包字符串提取 + 官方文档双重核实），已复核、采纳并推送（commit 见 main）：

- **`docs/adapter-matrix.md`**：三款工具全节实证 + 汇总表。
- **Claude Code 2.1.141**：用户级 `~/.claude/skills/`（`CLAUDE_CONFIG_DIR` 覆盖经二进制实证）、项目级 `.claude/skills/` —— 与注册表一致。
- **Cursor 3.13.25**：用户级与项目级均读 `.cursor/.agents/.codex/.claude/skills/`（workbench 分类函数实证）；内置 skills-cursor/、插件、cloud-skills/ 为工具自有位置 —— 注册表一致，notes 补实证说明。
- **Copilot（VS Code 1.120.0 + Copilot Chat 0.48.1）**：默认列表用户级 `[.copilot/skills, .claude/skills]`、项目级 `[.github/skills, .claude/skills]`（extension.js 实证）—— 模板一致；**注册表 also_read_by 中 `.agents` 一项按建议改为版本限定表述**（官方文档称支持，本机 0.48.1 未含）。
- `ADAPTER_VERSION` → 2026-09-09；需求文档同步 v1.4（§5 Copilot 行加版本限定）。
- 遗留提示：本机 VS Code 处于更新待切换状态（仅 new_Code.exe），切换后建议复核一次 Copilot 扩展版本与 .agents 默认列表——已记入地图雾区。
