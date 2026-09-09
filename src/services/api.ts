/**
 * SkillDock 前端统一 API 客户端
 *
 * 包装真实 Tauri 后端调用（call, onContractEvent）与目录选择对话框，
 * 并在非桌面环境下提供清晰隔离的模拟测试层。
 */

import {
  call,
  onContractEvent,
  type ContractCommands,
  type ContractCommandName,
  type ContractEventMap,
  type AppError,
} from '../lib/backend-contract';
import { isTauriEnvironment } from './environment';
import {
  MOCK_LIBRARIES,
  MOCK_LIBRARY_DETAIL,
  MOCK_TARGETS,
  MOCK_ADAPTERS,
  MOCK_SYNC_PLAN,
  MOCK_TASK_SNAPSHOT,
  MOCK_HISTORY,
  MOCK_BACKUP_STATS,
  MOCK_SNAPSHOTS,
} from './mock-data';
import { open as openDialog } from '@tauri-apps/plugin-dialog';

// 检查是否可以使用真实后端
export const isTauri = isTauriEnvironment();

/**
 * 弹出系统原生目录选择对话框
 */
export async function pickDirectory(defaultPath?: string): Promise<string | null> {
  if (isTauri) {
    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
        defaultPath: defaultPath || undefined,
        title: '选择技能库或目标文件夹',
      });
      if (typeof selected === 'string') {
        return selected;
      }
      return null;
    } catch (e) {
      console.error('目录选择器调用失败:', e);
      return null;
    }
  } else {
    // 浏览器调试模式下通过 prompt 交互
    const input = window.prompt('【浏览器模拟】请输入本地文件夹绝对路径：', defaultPath || 'D:\\Work\\my-skills');
    return input ? input.trim() : null;
  }
}

/**
 * 统一命令调用网关
 */
export async function invokeCommand<N extends ContractCommandName>(
  name: N,
  args?: ContractCommands[N]['args']
): Promise<ContractCommands[N]['result']> {
  if (isTauri) {
    // 真实 Tauri IPC 调用，类型安全
    return (call as any)(name, args ?? {});
  }

  // 以下为纯浏览器开发模式下的可替换 Mock 实现
  console.warn(`[SkillDock Mock] 模拟命令调用: ${name}`, args);

  // 模拟轻微网络/IO延迟 (120ms)
  await new Promise((r) => setTimeout(r, 120));

  switch (name) {
    case 'list_libraries':
      return MOCK_LIBRARIES as ContractCommands[N]['result'];
    case 'get_library':
      return MOCK_LIBRARY_DETAIL as ContractCommands[N]['result'];
    case 'list_targets':
      return MOCK_TARGETS as ContractCommands[N]['result'];
    case 'list_adapters':
      return MOCK_ADAPTERS as ContractCommands[N]['result'];
    case 'create_sync_plan':
      return MOCK_SYNC_PLAN as ContractCommands[N]['result'];
    case 'resolve_conflict': {
      const arg = args as ContractCommands['resolve_conflict']['args'];
      const updatedPlan = JSON.parse(JSON.stringify(MOCK_SYNC_PLAN));
      updatedPlan.planVersion += 1;
      for (const group of updatedPlan.groups) {
        for (const item of group.items) {
          if (item.itemId === arg.itemId) {
            item.decision = arg.choice;
            if (arg.choice === 'overwrite_with_source') {
              item.action = 'overwrite';
              item.conflict = null;
              item.selected = true;
            } else if (arg.choice === 'keep_target') {
              item.action = 'skip';
              item.selected = false;
            }
          }
        }
      }
      return updatedPlan as ContractCommands[N]['result'];
    }
    case 'execute_sync_plan':
      return { taskId: 'task-sync-999' } as ContractCommands[N]['result'];
    case 'get_task_snapshot':
      return MOCK_TASK_SNAPSHOT as ContractCommands[N]['result'];
    case 'list_tasks':
      return [MOCK_TASK_SNAPSHOT] as ContractCommands[N]['result'];
    case 'cancel_task':
      return { taskId: (args as any)?.taskId, state: 'requested' } as ContractCommands[N]['result'];
    case 'list_history':
      return MOCK_HISTORY as ContractCommands[N]['result'];
    case 'get_backup_stats':
      return MOCK_BACKUP_STATS as ContractCommands[N]['result'];
    case 'list_snapshots':
      return MOCK_SNAPSHOTS as ContractCommands[N]['result'];
    case 'update_backup_settings': {
      const arg = args as ContractCommands['update_backup_settings']['args'];
      return {
        ...MOCK_BACKUP_STATS,
        retentionDays: arg?.retentionDays ?? MOCK_BACKUP_STATS.retentionDays,
        softCapBytes: arg?.softCapBytes ?? MOCK_BACKUP_STATS.softCapBytes,
      } as ContractCommands[N]['result'];
    }
    case 'update_library_settings':
      return { configVersion: 4 } as ContractCommands[N]['result'];
    case 'open_registered_path':
      alert(`[浏览器模拟] 打开已登记路径: ${(args as any)?.kind} ${(args as any)?.id}`);
      return null as ContractCommands[N]['result'];
    case 'register_library': {
      const arg = args as ContractCommands['register_library']['args'];
      return {
        libraryId: 'lib-new-' + Date.now(),
        canonicalPath: arg.input.path,
        sourceRoot: arg.input.path,
        needsRootChoice: false,
        candidates: [],
        diagnostics: [],
        recentLibraries: MOCK_LIBRARIES,
      } as ContractCommands[N]['result'];
    }
    case 'save_target': {
      const arg = args as ContractCommands['save_target']['args'];
      return {
        targetId: 'tgt-custom-' + Date.now(),
        physicalTargetId: 'ptgt-custom-' + Date.now(),
        resolvedPath: arg.input.path || 'C:\\Users\\Custom\\Skills',
        resolutionSource: '用户自定义目录',
        availability: 'exists',
        sharedWithTools: ['自定义'],
        warnings: [],
      } as ContractCommands[N]['result'];
    }
    case 'scan_library':
      return { taskId: 'task-scan-' + Date.now() } as ContractCommands[N]['result'];
    case 'select_library_root': {
      const arg = args as ContractCommands['select_library_root']['args'];
      return {
        libraryId: arg.libraryId,
        canonicalPath: arg.root,
        sourceRoot: arg.root,
        needsRootChoice: false,
        candidates: [],
        diagnostics: [],
        recentLibraries: MOCK_LIBRARIES,
      } as ContractCommands[N]['result'];
    }
    case 'remove_target':
    case 'remove_library':
    case 'set_library_pinned':
      return null as ContractCommands[N]['result'];
    default:
      return null as ContractCommands[N]['result'];
  }
}

/**
 * 订阅契约事件监听器
 */
export async function subscribeEvent<N extends keyof ContractEventMap>(
  name: N,
  handler: (payload: ContractEventMap[N]) => void
): Promise<() => void> {
  if (isTauri) {
    return onContractEvent(name, handler);
  }

  // 纯浏览器环境下模拟
  console.log(`[SkillDock Mock] 注册事件监听: ${name}`);
  return () => {
    console.log(`[SkillDock Mock] 取消事件监听: ${name}`);
  };
}

export type { AppError };
