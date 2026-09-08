## 修正记录（2026-09-08）：Codex 技能目录按实测客户端校正

**用户报告 + 本机实证**：需求文档 §5 表中 Codex 的 `.agents/skills` 与本机安装的 codex-cli 0.147.0 不符。

证据（本机 codex-cli 0.147.0，npm 包 @openai/codex@0.147.0 的原生二进制字符串）：
- 存在 `.codex/skills`、`CODEX_HOME/skills`、`${CODEX_HOME:-$HOME/.codex}` 路径字符串
- **不存在** `.agents/skills` 路径字符串（`.agents` 仅出现于 `~/.agents/plugins/marketplace`，是插件市场，与技能无关）
- 本机 `~/.codex/skills/` 目录真实存在

处置（commit `fix: Codex 适配器目录按本机 codex-cli 0.147.0 实测校正为 .codex/skills`）：
- 用户级模板 → `%USERPROFILE%\.codex\skills`，支持 CODEX_HOME 覆盖
- 项目级模板 → `{PROJECT}\.codex\skills`
- docsUrl → https://developers.openai.com/codex/skills
- 适配器 notes 记录实测版本；docs 与实测不符时以实测为准（§16.1 原则的兑现）
- 新增单元测试：CODEX_HOME 覆盖、项目级模板

**遗留**：需求文档 §5 适配器表需同步更正（规格文件由需求方维护）；已在应用内保存过 Codex 目标的用户需在目标页删除后重新添加（已存的 Target 记录不会自动迁移）。
