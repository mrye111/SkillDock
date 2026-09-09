## Resolution

已完成并推送：

- **`docs/THIRD-PARTY-NOTICES.md`**（572 行，同步复制到 `release/` 随发布物分发）：许可族概览 + 246 个 Rust crate + 199 个 npm 包完整清单（名称/版本/许可证）+ 各许可族文本或要点 + Microsoft WebView2 Runtime 条款说明。
- 分布：MIT 217 / Apache-2.0 188 / ISC 9 / Unicode-3.0（ICU4X）/ MPL-2.0（cssparser 等）/ BSD-3 / BSL-1.0 / Zlib / Unlicense / CC0 / CC-BY-4.0（caniuse-lite）/ 0BSD / MIT-0——**无 GPL/LGPL/AGPL 强 copyleft 依赖**。
- 生成脚本 `scripts/gen_notices.py` 已入库，发布前可复跑保证与锁文件一致。
