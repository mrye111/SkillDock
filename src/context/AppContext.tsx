import React, { createContext, useContext, useEffect, useState, useCallback, useMemo } from 'react';
import type {
  LibraryDetail,
  LibrarySummary,
  TargetView,
  SyncPlanView,
  TaskSnapshot,
  SyncProgressEvent,
  SyncCompletedEvent,
  RecoveryRequiredEvent,
  ScanProgressEvent,
  AppError,
} from '../lib/backend-contract';
import {
  invokeCommand,
  subscribeEvent,
  pickDirectory,
  isTauri,
} from '../services/api';
import {
  EVENT_SCAN_PROGRESS,
  EVENT_SYNC_PROGRESS,
  EVENT_SYNC_COMPLETED,
  EVENT_RECOVERY_REQUIRED,
} from '../lib/backend-contract';
import { isCellActiveMapped } from '../components/common/Badge';

export type NavTab = 'workbench' | 'targets' | 'history' | 'settings';
export type StatusFilter = 'all' | 'pending' | 'conflict' | 'invalid';

export interface DrawerState {
  type: 'none' | 'skill_detail' | 'sync_plan' | 'target_detail' | 'history_detail';
  data?: any;
}

export interface ToastState {
  message: string;
  type: 'success' | 'warning' | 'error' | 'info';
}

interface AppContextType {
  // 导航
  currentTab: NavTab;
  setCurrentTab: (tab: NavTab) => void;

  // 技能库
  libraries: LibrarySummary[];
  activeLibraryId: string | null;
  activeLibrary: LibraryDetail | null;
  isLoadingLibrary: boolean;
  libraryError: AppError | null;
  switchLibrary: (libId: string) => Promise<void>;
  reloadCurrentLibrary: () => Promise<void>;
  registerNewLibrary: (path?: string, displayName?: string, mode?: 'auto' | 'collection' | 'single') => Promise<void>;
  removeLibraryRecord: (libId: string) => Promise<void>;
  setLibraryPinned: (libId: string, pinned: boolean) => Promise<void>;

  // 目标
  targets: TargetView[];
  isLoadingTargets: boolean;
  reloadTargets: () => Promise<void>;
  addTarget: (adapterId: any, scope: any, path?: string, displayName?: string) => Promise<boolean>;
  removeTarget: (targetId: string) => Promise<void>;
  batchAssignTarget: (skillIds: string[], physicalTargetId: string, enabled?: boolean) => Promise<void>;

  // 选中与筛选
  selectedSkillIds: Set<string>;
  toggleSelectSkill: (skillId: string) => void;
  selectAllSkills: (all: boolean) => void;
  clearSelection: () => void;
  searchQuery: string;
  setSearchQuery: (q: string) => void;
  statusFilter: StatusFilter;
  setStatusFilter: (f: StatusFilter) => void;

  // 抽屉与计划
  drawer: DrawerState;
  openDrawer: (type: DrawerState['type'], data?: any) => void;
  closeDrawer: () => void;
  currentPlan: SyncPlanView | null;
  setCurrentPlan: (plan: SyncPlanView | null) => void;
  requestSyncPreview: (specificMappingIds?: string[], operation?: 'sync' | 'remove' | 'restore') => Promise<void>;

  // 任务执行与恢复
  activeTask: TaskSnapshot | null;
  isTaskModalOpen: boolean;
  setIsTaskModalOpen: (open: boolean) => void;
  executePlan: () => Promise<void>;
  cancelCurrentTask: () => Promise<void>;
  recoveryRequired: RecoveryRequiredEvent | null;
  clearRecoveryRequired: () => void;

  // 模态弹窗与 Toast
  toast: ToastState | null;
  showToast: (message: string, type?: 'success' | 'warning' | 'error' | 'info') => void;
  clearToast: () => void;
  isLibrariesModalOpen: boolean;
  setIsLibrariesModalOpen: (open: boolean) => void;
  isAddLibraryModalOpen: boolean;
  setIsAddLibraryModalOpen: (open: boolean) => void;
  isAddTargetModalOpen: boolean;
  setIsAddTargetModalOpen: (open: boolean) => void;

  // 环境信息
  isTauriApp: boolean;
}

const AppContext = createContext<AppContextType | null>(null);

export const AppProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [currentTab, setCurrentTab] = useState<NavTab>('workbench');

  // 库状态
  const [libraries, setLibraries] = useState<LibrarySummary[]>([]);
  const [activeLibraryId, setActiveLibraryId] = useState<string | null>(null);
  const [activeLibrary, setActiveLibrary] = useState<LibraryDetail | null>(null);
  const [isLoadingLibrary, setIsLoadingLibrary] = useState<boolean>(false);
  const [libraryError, setLibraryError] = useState<AppError | null>(null);

  // 目标状态
  const [targets, setTargets] = useState<TargetView[]>([]);
  const [isLoadingTargets, setIsLoadingTargets] = useState<boolean>(false);

  // 筛选与勾选
  const [selectedSkillIds, setSelectedSkillIds] = useState<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all');

  // 抽屉与计划
  const [drawer, setDrawer] = useState<DrawerState>({ type: 'none' });
  const [currentPlan, setCurrentPlan] = useState<SyncPlanView | null>(null);

  // 任务与恢复
  const [activeTask, setActiveTask] = useState<TaskSnapshot | null>(null);
  const [isTaskModalOpen, setIsTaskModalOpen] = useState<boolean>(false);
  const [recoveryRequired, setRecoveryRequired] = useState<RecoveryRequiredEvent | null>(null);

  // 模态与 Toast
  const [toast, setToast] = useState<ToastState | null>(null);
  const [isLibrariesModalOpen, setIsLibrariesModalOpen] = useState(false);
  const [isAddLibraryModalOpen, setIsAddLibraryModalOpen] = useState(false);
  const [isAddTargetModalOpen, setIsAddTargetModalOpen] = useState(false);

  const showToast = useCallback((message: string, type: 'success' | 'warning' | 'error' | 'info' = 'success') => {
    setToast({ message, type });
  }, []);

  const clearToast = useCallback(() => {
    setToast(null);
  }, []);

  // 加载技能库列表与初始目标
  const loadInitialData = useCallback(async () => {
    try {
      setIsLoadingTargets(true);
      const [libs, tgts] = await Promise.all([
        invokeCommand('list_libraries'),
        invokeCommand('list_targets'),
      ]);
      setLibraries(libs || []);
      setTargets(tgts || []);

      if (libs && libs.length > 0) {
        // 优先激活首个置顶的库，或最近打开的库
        const active = libs.find((l) => l.pinned) || libs[0];
        setActiveLibraryId(active.libraryId);
      }
    } catch (err) {
      console.error('初始化数据获取失败:', err);
    } finally {
      setIsLoadingTargets(false);
    }
  }, []);

  // 等待特定任务发出 sync.completed 完成事件
  const waitForTaskCompletion = useCallback((taskId: string, timeoutMs = 8000): Promise<void> => {
    return new Promise((resolve) => {
      let done = false;
      let unlisten: (() => void) | undefined;

      const timer = setTimeout(() => {
        if (!done) {
          done = true;
          if (unlisten) unlisten();
          resolve();
        }
      }, timeoutMs);

      subscribeEvent(EVENT_SYNC_COMPLETED, (payload) => {
        if (payload.taskId === taskId && !done) {
          done = true;
          clearTimeout(timer);
          if (unlisten) unlisten();
          resolve();
        }
      }).then((fn) => {
        unlisten = fn;
      });
    });
  }, []);

  // 扫描磁盘并载入技能库详情
  const scanAndLoadLibrary = useCallback(async (targetLibraryId: string) => {
    setIsLoadingLibrary(true);
    setLibraryError(null);
    try {
      let libId = targetLibraryId;

      // 1. 获取库列表，判断是否存在 sourceRoot 为 null 的单技能目录
      const currentLibs = await invokeCommand('list_libraries');
      setLibraries(currentLibs || []);
      const targetSummary = currentLibs?.find((l) => l.libraryId === libId);

      if (targetSummary && !targetSummary.sourceRoot) {
        console.warn(`[SkillDock] 库 ${targetSummary.displayName} 的 sourceRoot 为空，尝试用 mode: 'single' 重新登记修复...`);
        try {
          await invokeCommand('remove_library', { libraryId: libId });
          const reReg = await invokeCommand('register_library', {
            input: {
              path: targetSummary.canonicalPath,
              displayName: targetSummary.displayName,
              mode: 'single',
            },
          });
          libId = reReg.libraryId;
          setActiveLibraryId(libId);
          const reLibs = await invokeCommand('list_libraries');
          setLibraries(reLibs || []);
        } catch (repairErr) {
          console.warn('以 single 模式自动重新登记失败:', repairErr);
        }
      }

      // 2. 发起 scan_library 扫描磁盘
      let scanTaskId: string | null = null;
      try {
        const scanRes = await invokeCommand('scan_library', { libraryId: libId });
        scanTaskId = scanRes.taskId;
      } catch (scanErr: any) {
        if (scanErr?.message?.includes('请先选定源根目录') || scanErr?.code === 'validation_failed') {
          const freshLibs = await invokeCommand('list_libraries');
          const sum = freshLibs?.find((l) => l.libraryId === libId);
          if (sum) {
            console.warn(`[SkillDock] scan_library 提示选定源根目录，自动转换为 single 模式重新登记...`);
            await invokeCommand('remove_library', { libraryId: libId });
            const reReg = await invokeCommand('register_library', {
              input: {
                path: sum.canonicalPath,
                displayName: sum.displayName,
                mode: 'single',
              },
            });
            libId = reReg.libraryId;
            setActiveLibraryId(libId);
            const reLibs = await invokeCommand('list_libraries');
            setLibraries(reLibs || []);
            const retryScan = await invokeCommand('scan_library', { libraryId: libId });
            scanTaskId = retryScan.taskId;
          }
        } else {
          throw scanErr;
        }
      }

      // 3. 等待 sync.completed 事件
      if (scanTaskId) {
        await waitForTaskCompletion(scanTaskId);
      }

      // 4. 扫描完成后，读取最新详情
      const detail = await invokeCommand('get_library', { libraryId: libId });
      setActiveLibrary(detail);

      // 5. 保留已勾选的有效技能（刷新或重新扫描时保留用户先前的勾选；若未选择或已取消，则保持未选择状态，避免意外全选）
      setSelectedSkillIds((prevSelected) => {
        if (prevSelected && prevSelected.size > 0) {
          const validSkillIds = new Set(
            detail.skills
              .filter((s) => s.validation.status === 'valid')
              .map((s) => s.skillId)
          );
          const retained = new Set<string>();
          for (const id of prevSelected) {
            if (validSkillIds.has(id)) {
              retained.add(id);
            }
          }
          return retained;
        }
        return new Set();
      });

      // 6. 同步刷新列表
      const latestLibs = await invokeCommand('list_libraries');
      setLibraries(latestLibs || []);
    } catch (err: any) {
      console.error('扫描或获取技能库详情失败:', err);
      setLibraryError(err as AppError);
      setActiveLibrary(null);
    } finally {
      setIsLoadingLibrary(false);
    }
  }, [waitForTaskCompletion]);

  // 切换活动库
  const switchLibrary = useCallback(async (libId: string) => {
    setActiveLibraryId(libId);
    setSelectedSkillIds(new Set());
    await scanAndLoadLibrary(libId);
  }, [scanAndLoadLibrary]);

  // 刷新当前库
  const reloadCurrentLibrary = useCallback(async () => {
    if (activeLibraryId) {
      await scanAndLoadLibrary(activeLibraryId);
    }
  }, [activeLibraryId, scanAndLoadLibrary]);

  // 刷新目标
  const reloadTargets = useCallback(async () => {
    try {
      const tgts = await invokeCommand('list_targets');
      setTargets(tgts || []);
    } catch (e) {
      console.error('刷新目标失败:', e);
    }
  }, []);

  // 登记新技能库
  const registerNewLibrary = useCallback(
    async (
      initialPath?: string,
      displayName?: string,
      mode: 'auto' | 'collection' | 'single' = 'auto'
    ) => {
      let path = initialPath;
      if (!path) {
        const picked = await pickDirectory();
        if (!picked) return;
        path = picked;
      }

      try {
        setIsLoadingLibrary(true);
        let res = await invokeCommand('register_library', {
          input: {
            path,
            displayName,
            mode,
          },
        });

        // 如果 sourceRoot 为空且无候选，改用 single 重新登记
        if (!res.sourceRoot && (!res.candidates || res.candidates.length === 0)) {
          await invokeCommand('remove_library', { libraryId: res.libraryId });
          res = await invokeCommand('register_library', {
            input: {
              path,
              displayName,
              mode: 'single',
            },
          });
        } else if (res.needsRootChoice && res.candidates.length > 0) {
          const chosen = res.candidates[0].path;
          res = await invokeCommand('select_library_root', {
            libraryId: res.libraryId,
            root: chosen,
          });
        }

        const updatedLibs = await invokeCommand('list_libraries');
        setLibraries(updatedLibs || []);
        setActiveLibraryId(res.libraryId);
        setSelectedSkillIds(new Set());
        await scanAndLoadLibrary(res.libraryId);
        showToast('技能库登记成功', 'success');
      } catch (e: any) {
        showToast(`登记技能库失败: ${e?.message || e}`, 'error');
      } finally {
        setIsLoadingLibrary(false);
      }
    },
    [scanAndLoadLibrary, showToast]
  );

  // 移除技能库记录
  const removeLibraryRecord = useCallback(
    async (libId: string) => {
      try {
        await invokeCommand('remove_library', { libraryId: libId });
        const updatedLibs = await invokeCommand('list_libraries');
        setLibraries(updatedLibs || []);
        if (activeLibraryId === libId) {
          if (updatedLibs && updatedLibs.length > 0) {
            switchLibrary(updatedLibs[0].libraryId);
          } else {
            setActiveLibraryId(null);
            setActiveLibrary(null);
          }
        }
        showToast('已移除技能库记录', 'info');
      } catch (e: any) {
        showToast(`移除技能库失败: ${e?.message || e}`, 'error');
      }
    },
    [activeLibraryId, switchLibrary, showToast]
  );

  // 设置技能库置顶
  const setLibraryPinned = useCallback(
    async (libId: string, pinned: boolean) => {
      try {
        await invokeCommand('set_library_pinned', { libraryId: libId, pinned });
        const updatedLibs = await invokeCommand('list_libraries');
        setLibraries(updatedLibs || []);
        showToast(pinned ? '已固定技能库' : '已取消固定', 'success');
      } catch (e: any) {
        showToast(`设置固定状态失败: ${e?.message || e}`, 'error');
      }
    },
    [showToast]
  );

  // 目标添加
  const addTarget = useCallback(
    async (
      adapterId: any,
      scope: any,
      path?: string,
      displayName?: string
    ): Promise<boolean> => {
      try {
        await invokeCommand('save_target', {
          input: {
            adapterId,
            scope,
            path,
            displayName,
          },
        });
        await reloadTargets();
        if (activeLibraryId) {
          await reloadCurrentLibrary();
        }
        showToast('已成功保存同步目标', 'success');
        return true;
      } catch (e: any) {
        showToast(`保存目标失败: ${e?.message || e}`, 'error');
        return false;
      }
    },
    [activeLibraryId, reloadTargets, reloadCurrentLibrary, showToast]
  );

  // 目标移除
  const removeTarget = useCallback(
    async (targetId: string) => {
      try {
        await invokeCommand('remove_target', { targetId });
        await reloadTargets();
        if (activeLibraryId) {
          await reloadCurrentLibrary();
        }
        showToast('已移除目标配置', 'info');
      } catch (e: any) {
        showToast(`移除目标失败: ${e?.message || e}`, 'error');
      }
    },
    [activeLibraryId, reloadTargets, reloadCurrentLibrary, showToast]
  );

  // 批量配置技能与物理目标的映射
  const batchAssignTarget = useCallback(
    async (skillIds: string[], physicalTargetId: string, enabled = true) => {
      if (!activeLibrary) return;
      try {
        const skillsToUpdate = activeLibrary.skills.filter(
          (s) => skillIds.includes(s.skillId) && s.validation.status === 'valid'
        );
        if (skillsToUpdate.length === 0) return;

        showToast(`正在更新 ${skillsToUpdate.length} 个技能的同步目标...`, 'info');

        await Promise.all(
          skillsToUpdate.map((skill) => {
            const currentTargets = new Set(
              Object.entries(skill.targets)
                .filter(([, c]) => isCellActiveMapped(c))
                .map(([ptId]) => ptId)
            );
            if (enabled) {
              currentTargets.add(physicalTargetId);
            } else {
              currentTargets.delete(physicalTargetId);
            }

            return invokeCommand('update_mappings', {
              libraryId: activeLibrary.summary.libraryId,
              skillId: skill.skillId,
              physicalTargetIds: Array.from(currentTargets),
            });
          })
        );

        await reloadCurrentLibrary();
        showToast(
          enabled
            ? `已成功为 ${skillsToUpdate.length} 个技能关联同步目标`
            : `已成功为 ${skillsToUpdate.length} 个技能取消关联目标`,
          'success'
        );
      } catch (err: any) {
        showToast(`配置目标失败: ${err?.message || err}`, 'error');
      }
    },
    [activeLibrary, reloadCurrentLibrary, showToast]
  );

  // 选择逻辑
  const toggleSelectSkill = useCallback((skillId: string) => {
    setSelectedSkillIds((prev) => {
      const next = new Set(prev);
      if (next.has(skillId)) {
        next.delete(skillId);
      } else {
        next.add(skillId);
      }
      return next;
    });
  }, []);

  const selectAllSkills = useCallback(
    (selectAll: boolean) => {
      if (!activeLibrary) return;
      if (selectAll) {
        const all = new Set<string>();
        for (const s of activeLibrary.skills) {
          if (s.validation.status === 'valid') {
            all.add(s.skillId);
          }
        }
        setSelectedSkillIds(all);
      } else {
        setSelectedSkillIds(new Set());
      }
    },
    [activeLibrary]
  );

  const clearSelection = useCallback(() => {
    setSelectedSkillIds(new Set());
  }, []);

  // 抽屉控制
  const openDrawer = useCallback((type: DrawerState['type'], data?: any) => {
    setDrawer({ type, data });
  }, []);

  const closeDrawer = useCallback(() => {
    setDrawer({ type: 'none' });
  }, []);

  // 生成并打开同步预览计划
  const requestSyncPreview = useCallback(
    async (
      specificMappingIds?: string[],
      operation: 'sync' | 'remove' | 'restore' = 'sync'
    ) => {
      if (!activeLibrary) return;

      let mappingIds = specificMappingIds;
      if (!mappingIds) {
        mappingIds = [];
        for (const skill of activeLibrary.skills) {
          if (selectedSkillIds.has(skill.skillId)) {
            for (const cell of Object.values(skill.targets)) {
              if (isCellActiveMapped(cell)) {
                mappingIds.push(cell.mappingId!);
              }
            }
          }
        }
      }

      if (mappingIds.length === 0) {
        showToast('请先为所选技能配置同步目标。', 'warning');
        return;
      }

      try {
        const plan = await invokeCommand('create_sync_plan', {
          input: {
            libraryId: activeLibrary.summary.libraryId,
            operation,
            mappingIds,
          },
        });
        setCurrentPlan(plan);
        openDrawer('sync_plan');
      } catch (e: any) {
        showToast(`生成计划失败: ${e?.message || e}`, 'error');
      }
    },
    [activeLibrary, selectedSkillIds, openDrawer, showToast]
  );

  // 执行同步计划
  const executePlan = useCallback(async () => {
    if (!currentPlan) return;
    try {
      const { taskId } = await invokeCommand('execute_sync_plan', {
        planId: currentPlan.planId,
        planVersion: currentPlan.planVersion,
      });

      const snapshot = await invokeCommand('get_task_snapshot', { taskId });
      setActiveTask(snapshot);
      setIsTaskModalOpen(true);
      closeDrawer();
    } catch (e: any) {
      showToast(`执行失败: ${e?.message || e}`, 'error');
    }
  }, [currentPlan, closeDrawer, showToast]);

  // 取消任务
  const cancelCurrentTask = useCallback(async () => {
    if (!activeTask) return;
    try {
      await invokeCommand('cancel_task', { taskId: activeTask.taskId });
      showToast('已发送取消任务请求', 'info');
    } catch (e: any) {
      showToast(`取消任务失败: ${e?.message || e}`, 'error');
    }
  }, [activeTask, showToast]);

  // 清除恢复提示
  const clearRecoveryRequired = useCallback(() => {
    setRecoveryRequired(null);
  }, []);

  // 初始化加载
  useEffect(() => {
    loadInitialData();
  }, [loadInitialData]);

  // 切换库与启动恢复上次库时，先扫描磁盘再载入详情
  useEffect(() => {
    if (activeLibraryId) {
      scanAndLoadLibrary(activeLibraryId);
    }
  }, [activeLibraryId, scanAndLoadLibrary]);

  // 监听后端事件
  useEffect(() => {
    let unlistenSyncProgress: (() => void) | undefined;
    let unlistenSyncCompleted: (() => void) | undefined;
    let unlistenScanProgress: (() => void) | undefined;
    let unlistenRecovery: (() => void) | undefined;

    async function bindEvents() {
      unlistenSyncProgress = await subscribeEvent(EVENT_SYNC_PROGRESS, (payload: SyncProgressEvent) => {
        setActiveTask((prev) => {
          if (!prev || prev.taskId !== payload.taskId) return prev;
          const updatedItems = [...prev.items];
          if (payload.itemId) {
            const idx = updatedItems.findIndex((it) => it.itemId === payload.itemId);
            if (idx >= 0) {
              updatedItems[idx] = {
                ...updatedItems[idx],
                status: payload.phase === 'item_done' ? 'success' : 'running',
                bytesProcessed: payload.bytesDone ?? updatedItems[idx].bytesProcessed,
                bytesTotal: payload.bytesTotal ?? updatedItems[idx].bytesTotal,
              };
            }
          }
          return {
            ...prev,
            status: 'running',
            items: updatedItems,
          };
        });
      });

      unlistenSyncCompleted = await subscribeEvent(EVENT_SYNC_COMPLETED, (payload: SyncCompletedEvent) => {
        setActiveTask((prev) => {
          if (!prev || prev.taskId !== payload.taskId) return prev;
          return {
            ...prev,
            status: payload.status,
            counts: payload.counts,
            finishedAt: new Date().toISOString(),
          };
        });
        if (activeLibraryId) {
          invokeCommand('get_library', { libraryId: activeLibraryId })
            .then((fresh) => {
              if (fresh) setActiveLibrary(fresh);
            })
            .catch(() => {});
        }
      });

      unlistenScanProgress = await subscribeEvent(EVENT_SCAN_PROGRESS, (_payload: ScanProgressEvent) => {
        // 扫描进度
      });

      unlistenRecovery = await subscribeEvent(EVENT_RECOVERY_REQUIRED, (payload: RecoveryRequiredEvent) => {
        setRecoveryRequired(payload);
      });
    }

    bindEvents();

    return () => {
      if (unlistenSyncProgress) unlistenSyncProgress();
      if (unlistenSyncCompleted) unlistenSyncCompleted();
      if (unlistenScanProgress) unlistenScanProgress();
      if (unlistenRecovery) unlistenRecovery();
    };
  }, [activeLibraryId]);

  // 全局快捷键绑定：Ctrl+O, Ctrl+F, F5, Ctrl+Enter, Esc
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Esc: 关闭抽屉或弹窗
      if (e.key === 'Escape') {
        if (isTaskModalOpen && activeTask?.status !== 'running') {
          setIsTaskModalOpen(false);
        } else if (drawer.type !== 'none') {
          closeDrawer();
        } else if (isLibrariesModalOpen) {
          setIsLibrariesModalOpen(false);
        } else if (isAddLibraryModalOpen) {
          setIsAddLibraryModalOpen(false);
        } else if (isAddTargetModalOpen) {
          setIsAddTargetModalOpen(false);
        }
        return;
      }

      // Ctrl+O: 打开添加技能库弹窗
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'o') {
        e.preventDefault();
        setIsAddLibraryModalOpen(true);
        return;
      }

      // Ctrl+F: 聚焦搜索框
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
        e.preventDefault();
        const searchInput = document.getElementById('skill-search') as HTMLInputElement | null;
        if (searchInput) {
          searchInput.focus();
          searchInput.select();
        }
        return;
      }

      // F5: 刷新当前技能库
      if (e.key === 'F5') {
        e.preventDefault();
        reloadCurrentLibrary();
        return;
      }

      // Ctrl+Enter: 开启同步预览
      if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
        e.preventDefault();
        if (currentTab === 'workbench' && selectedSkillIds.size > 0 && drawer.type === 'none') {
          requestSyncPreview();
        }
        return;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [
    isTaskModalOpen,
    activeTask,
    drawer.type,
    isLibrariesModalOpen,
    isAddLibraryModalOpen,
    isAddTargetModalOpen,
    closeDrawer,
    reloadCurrentLibrary,
    currentTab,
    selectedSkillIds.size,
    requestSyncPreview,
  ]);

  const value = useMemo<AppContextType>(
    () => ({
      currentTab,
      setCurrentTab,
      libraries,
      activeLibraryId,
      activeLibrary,
      isLoadingLibrary,
      libraryError,
      switchLibrary,
      reloadCurrentLibrary,
      registerNewLibrary,
      removeLibraryRecord,
      setLibraryPinned,
      targets,
      isLoadingTargets,
      reloadTargets,
      addTarget,
      removeTarget,
      selectedSkillIds,
      toggleSelectSkill,
      selectAllSkills,
      clearSelection,
      searchQuery,
      setSearchQuery,
      statusFilter,
      setStatusFilter,
      drawer,
      openDrawer,
      closeDrawer,
      currentPlan,
      setCurrentPlan,
      requestSyncPreview,
      activeTask,
      isTaskModalOpen,
      setIsTaskModalOpen,
      executePlan,
      cancelCurrentTask,
      recoveryRequired,
      clearRecoveryRequired,
      toast,
      showToast,
      clearToast,
      isLibrariesModalOpen,
      setIsLibrariesModalOpen,
      isAddLibraryModalOpen,
      setIsAddLibraryModalOpen,
      isAddTargetModalOpen,
      setIsAddTargetModalOpen,
      batchAssignTarget,
      isTauriApp: isTauri,
    }),
    [
      currentTab,
      libraries,
      activeLibraryId,
      activeLibrary,
      isLoadingLibrary,
      libraryError,
      switchLibrary,
      reloadCurrentLibrary,
      registerNewLibrary,
      removeLibraryRecord,
      setLibraryPinned,
      targets,
      isLoadingTargets,
      reloadTargets,
      addTarget,
      removeTarget,
      batchAssignTarget,
      selectedSkillIds,
      toggleSelectSkill,
      selectAllSkills,
      clearSelection,
      searchQuery,
      statusFilter,
      drawer,
      openDrawer,
      closeDrawer,
      currentPlan,
      requestSyncPreview,
      activeTask,
      isTaskModalOpen,
      executePlan,
      cancelCurrentTask,
      recoveryRequired,
      clearRecoveryRequired,
      toast,
      showToast,
      clearToast,
      isLibrariesModalOpen,
      isAddLibraryModalOpen,
      isAddTargetModalOpen,
    ]
  );

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
};

export const useApp = () => {
  const context = useContext(AppContext);
  if (!context) {
    throw new Error('useApp 必须在 AppProvider 内使用');
  }
  return context;
};
