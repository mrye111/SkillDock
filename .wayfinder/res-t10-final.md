## Resolution（2026-09-08 完成）

前后端联调打包已完成，双发行物产出并通过本机安装验收。

**产出物**（`release/`，已 gitignore，不入库）：
- `SkillDock-0.1.0-windows-x64-setup.exe`（标准包，2.67 MiB ≤ 40 MiB 预算；WebView2 downloadBootstrapper 引导）
- `SkillDock-0.1.0-windows-x64-offline-setup.exe`（离线包，252.5 MiB，内嵌 WebView2 离线安装组件，体积单独披露）
- `SHA256SUMS.txt` 校验值

**AC-28 本机实测通过**：/S 静默安装 → `%LOCALAPPDATA%\SkillDock`（当前用户、免管理员）→ 开始菜单入口创建 → 启动出现主窗口 → /S 静默卸载 → 程序文件/注册表/开始菜单全清且用户数据（app.db）哈希不变。过程中发现并记录两个环境教训：① Tauri NSIS currentUser 默认是 `%LOCALAPPDATA%\<产品名>`（无 Programs 层级），需求文档已更正；② 从 MSYS/Git Bash 上下文启动安装程序会导致 NSIS 壳文件夹解析异常装到 `D:\SkillDock`，从 cmd/Explorer 启动则正常。

**验收残留（不阻塞本票关闭，已列入地图 Not yet specified）**：
- AC-29 离线包需在无 WebView2 的干净系统实测（本机已有运行时，无法真实验证引导路径）
- AC-28 的干净虚拟机复测、升级覆盖安装测试
- 正式图标（当前为品牌色占位）
- 「安装更新前检测正在执行的同步任务」非 Tauri 默认能力，需要自定义 NSIS 钩子（P1 再议）
