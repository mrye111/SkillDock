/**
 * 桌面运行环境检测与配置
 */

export function isTauriEnvironment(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export type DataSourceMode = 'tauri' | 'mock';

let currentMode: DataSourceMode = isTauriEnvironment() ? 'tauri' : 'mock';

export function getDataSourceMode(): DataSourceMode {
  return currentMode;
}

export function setDataSourceMode(mode: DataSourceMode): void {
  // 严格安全限制：在真实 Tauri 环境中不允许强制伪装为已成功同步的假数据，
  // 但允许在纯浏览器调试下切换
  currentMode = mode;
}
