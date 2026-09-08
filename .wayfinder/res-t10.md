## 进展（2026-09-08，部分完成）

已交付配置（已推送 main）：
- `src-tauri/tauri.conf.json`：窗口 1280×820 / 最小 960×640（§9.1）；NSIS 当前用户安装（installMode currentUser，§12.2）；zh-CN；webviewInstallMode = downloadBootstrapper（标准包缺失时引导安装，§12.1）
- `src-tauri/tauri.offline.conf.json`：离线变体覆盖 webviewInstallMode = offlineInstaller（`tauri build --config src-tauri/tauri.offline.conf.json`）
- `src-tauri/capabilities/default.json`：最小权限（core + event + dialog:allow-open + window-state）
- 占位图标（品牌主色 #0F766E），正式图标待设计
- 卸载默认保留用户数据：Tauri NSIS 模板默认行为即「勾选才删除」，不额外配置

**最新进展（2026-09-08）**：
- 前端生产构建 `npm run build` 就绪，TS 类型检查 0 错误；
- 修复了 NSIS 语言配置（由 `zh-CN` 改为 NSIS 规范标识 `SimpChinese`）；
- 成功执行 `npm run tauri build`，标准版 NSIS 安装程序已成功产出：
  - 产物路径：`src-tauri/target/release/bundle/nsis/SkillDock_0.1.0_x64-setup.exe`（2.79 MB）；
  - 独立运行程序：`src-tauri/target/release/skilldock.exe`。

**剩余**：AC-28/29（虚拟机实机安装与升级测试）；离线安装包变体构建；正式图标替换。
