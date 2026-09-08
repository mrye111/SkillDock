# Seedance 2.5 参考路由器（Reference Router）

> 各路由目标的详细内容见 `reference-details.md` 的同名小节。
> 当多个族别竞争时，先读 `reference-details.md` 的「00-index」选择规则。

## 路由表（需求 → 参考条目）

| 需求 | 参考条目 |
|------|---------|
| 能力与运行时边界 | 01-capabilities-and-boundaries |
| 大包、R2V、故事板、白模/绿幕、参考冲突 | 02-reference-orchestration |
| 端点插值（首末帧） | 03-first-last-frame |
| 提示词结构、时间码、镜头、音频、文案 | 10-prompt-architecture |
| 故事、动画、一镜到底、动作、表演、游戏过场 | 20-story-previs-performance |
| 广告、电商、社交、产品、时尚、美食 | 30-commercial-growth |
| 讲解、培训、数字人、本地化 | 31-knowledge-enterprise-localization |
| 纪录片、医疗、科学、野生动物、监管声明 | 32-factual-regulated |
| 建筑、地产、汽车、路线、旅行 | 33-spaces-mobility-travel |
| 本地编辑、源编辑、扩展、诊断、重试 | 40-local-editing-and-repair |
| 来源与维护 | 98-source-ledger |
| 正向测试用例 | 99-eval-cases |

## 证据层级（来源可信度）

- **O（官方）**：ByteDance Seed / Volcengine Ark —— 用于模型方向
- **F（第一方）**：Dreamina 产品面 —— 用于工作流可能性（上限非目标）
- **P（流行可复现）**：社区高信号实践 —— 用于可检查启发式
- **D（领域标准）**：TikTok Creative Center、Google ABCD、FDA、C2PA 等 —— 用于验收、真相、版权、安全和披露
- **S（二级）**：其他 —— 仅作线索

## 模式准入规则（00-index 摘要）

保留模式需同时满足 6 个条件：独特用户任务、官方/可检查高信号支持、特定参考包或提示词更改、可见验收测试、最小修复动作、无未验证运行时声明。
