## 进展（2026-09-08，部分完成）

已交付配置（已推送 main）：
- `src-tauri/tauri.conf.json`：窗口 1280×820 / 最小 960×640（§9.1）；NSIS 当前用户安装（installMode currentUser，§12.2）；zh-CN；webviewInstallMode = downloadBootstrapper（标准包缺失时引导安装，§12.1）
- `src-tauri/tauri.offline.conf.json`：离线变体覆盖 webviewInstallMode = offlineInstaller（`tauri build --config src-tauri/tauri.offline.conf.json`）
- `src-tauri/capabilities/default.json`：最小权限（core + event + dialog:allow-open + window-state）
- 占位图标（品牌主色 #0F766E），正式图标待设计
- 卸载默认保留用户数据：Tauri NSIS 模板默认行为即「勾选才删除」，不额外配置

**剩余（本票保持开启）**：前端 `dist/` 就绪后联调 `npm run tauri build` 产出标准/离线 EXE；AC-28/29（安装/升级/卸载/离线）需干净虚拟机实测；正式图标替换。
