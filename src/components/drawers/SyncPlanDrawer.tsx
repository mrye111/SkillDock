import React, { useState } from 'react';
import { useApp } from '../../context/AppContext';
import { Drawer } from '../common/Drawer';
import { Icon, ToolIcon } from '../common/Icon';
import { ActionBadge } from '../common/Badge';
import { invokeCommand } from '../../services/api';
import type { ConflictChoice, ConflictInfo, PlanItemView } from '../../lib/backend-contract';

const CHOICE_LABELS: Record<ConflictChoice, string> = {
  keep_target: '本次保留目标',
  overwrite_with_source: '备份后用源覆盖',
  adopt_existing: '接管相同内容',
  take_over: '接管并用源覆盖',
  pause_mapping: '暂停此映射',
  reinstall: '重新安装',
  keep_deleted: '维持删除状态',
  transfer_ownership: '转移归属',
  remove_with_backup: '备份后移除',
};

export const SyncPlanDrawer: React.FC = () => {
  const {
    drawer,
    closeDrawer,
    currentPlan,
    setCurrentPlan,
    executePlan,
    resolveConflictsBulk,
    targets,
    showToast,
  } = useApp();

  const [resolvingItemId, setResolvingItemId] = useState<string | null>(null);
  const [isBulkResolving, setIsBulkResolving] = useState(false);

  if (drawer.type !== 'sync_plan' || !currentPlan) {
    return null;
  }

  const { summary, groups, operation, status } = currentPlan;
  const isRestore = operation === 'restore';
  const isRemove = operation === 'remove';

  const title = isRestore ? '恢复预览' : isRemove ? '移除预览' : '同步预览';
  const subtitle = '检查具体变化，再决定执行。';

  const formatBytes = (bytes: number) => {
    if (bytes >= 1073741824) return `${(bytes / 1073741824).toFixed(1)} GB`;
    if (bytes >= 1048576) return `${(bytes / 1048576).toFixed(1)} MB`;
    if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${bytes} B`;
  };

  // 解决冲突
  const handleResolveConflict = async (item: PlanItemView, choice: ConflictChoice) => {
    setResolvingItemId(item.itemId);
    try {
      const updatedPlan = await invokeCommand('resolve_conflict', {
        planId: currentPlan.planId,
        planVersion: currentPlan.planVersion,
        itemId: item.itemId,
        choice,
      });
      setCurrentPlan(updatedPlan);
      showToast('已更新冲突决策', 'success');
    } catch (e: any) {
      showToast(`冲突决策失败: ${e?.message || e}`, 'error');
    } finally {
      setResolvingItemId(null);
    }
  };

  // 待决冲突项统计（直接计算，严禁在 early return 后使用 React Hook 以免违反 Rules of Hooks 导致白屏）
  const allConflictItems = groups.flatMap((g) => g.items.filter((it) => Boolean(it.conflict)));

  // 相同内容冲突（可零风险一键接管）
  const sameContentConflicts = allConflictItems.filter(
    (it) =>
      it.conflict?.kind === 'same_content' &&
      it.conflict?.availableChoices.includes('adopt_existing')
  );

  // 可保留目标冲突
  const keepTargetConflicts = allConflictItems.filter((it) =>
    it.conflict?.availableChoices.includes('keep_target')
  );

  // 一键接管全部内容一致项
  const handleAdoptAllSameContent = async () => {
    setIsBulkResolving(true);
    try {
      await resolveConflictsBulk(['same_content'], 'adopt_existing');
    } finally {
      setIsBulkResolving(false);
    }
  };

  // 批量保留目标
  const handleKeepAllTarget = async () => {
    setIsBulkResolving(true);
    try {
      const kinds = Array.from(
        new Set(
          keepTargetConflicts
            .map((it) => it.conflict?.kind)
            .filter((k): k is ConflictInfo['kind'] => Boolean(k))
        )
      );
      if (kinds.length > 0) {
        await resolveConflictsBulk(kinds, 'keep_target');
      }
    } finally {
      setIsBulkResolving(false);
    }
  };

  const firstStatLabel = isRestore ? '恢复' : isRemove ? '移除' : '新增';
  const firstStatCount = isRestore ? summary.restoreCount : isRemove ? summary.removeCount : summary.createCount;

  const middleStatLabel = isRestore || isRemove ? '可执行' : '更新 / 接管';
  const middleStatCount = isRestore || isRemove ? summary.executableCount : summary.updateCount;

  return (
    <Drawer
      isOpen={drawer.type === 'sync_plan'}
      onClose={closeDrawer}
      title={title}
      subtitle={subtitle}
      footer={
        <>
          <span className="plan-footer-info">
            <small>
              {summary.executableCount} 项可执行 · {formatBytes(summary.totalBytes)}
            </small>
            <small>执行前自动创建备份，可在历史记录中恢复</small>
          </span>
          <button
            type="button"
            className="button primary"
            disabled={summary.executableCount === 0 || status !== 'active'}
            onClick={executePlan}
          >
            <span>
              {isRestore ? '执行恢复' : isRemove ? '执行移除' : '执行同步'}{' '}
              {summary.executableCount} 项
            </span>
            <Icon name="arrow" size={14} />
          </button>
        </>
      }
    >
      {status === 'stale' && (
        <div className="error-box">
          计划已失效（内容或忽略规则已变化），请回到技能库重新生成预览。
        </div>
      )}

      {/* 三栏数字看板 */}
      <div className="drawer-summary">
        <div>
          <strong>{firstStatCount}</strong>
          <small>{firstStatLabel}</small>
        </div>
        <div>
          <strong>{middleStatCount}</strong>
          <small>{middleStatLabel}</small>
        </div>
        <div>
          <strong>{summary.conflictCount + summary.blockedCount}</strong>
          <small>需要处理</small>
        </div>
      </div>

      {/* 冲突批量快速处理工具条 */}
      {summary.conflictCount > 0 && (sameContentConflicts.length > 0 || keepTargetConflicts.length > 0) && (
        <div className="p-3.5 mb-4 rounded-xl border border-amber-200/80 bg-gradient-to-r from-amber-50/90 to-orange-50/80 backdrop-blur shadow-sm">
          <div className="flex items-center justify-between gap-2 mb-2">
            <div className="flex items-center gap-2">
              <span className="flex h-5 w-5 items-center justify-center rounded-full bg-amber-100 text-amber-700">
                <Icon name="shield" size={12} />
              </span>
              <strong className="text-xs font-semibold text-amber-900">
                检测到 {summary.conflictCount} 项待决策冲突
              </strong>
            </div>
            <span className="text-[11px] text-amber-700">
              推荐批量快速决策
            </span>
          </div>

          <div className="flex items-center gap-2 flex-wrap pt-2 border-t border-amber-200/50">
            {sameContentConflicts.length > 0 && (
              <button
                type="button"
                className="button primary small inline-flex items-center gap-1.5 shadow-sm text-xs"
                disabled={isBulkResolving}
                onClick={handleAdoptAllSameContent}
                title="源与目标文件逐字节完全一致，安全接管并建立管理基线"
              >
                <Icon name="check" size={13} />
                <span>
                  {isBulkResolving ? '正在批量处理...' : `全部接管内容一致项 (${sameContentConflicts.length})`}
                </span>
              </button>
            )}

            {keepTargetConflicts.length > 0 && (
              <button
                type="button"
                className="button secondary small inline-flex items-center gap-1.5 text-xs"
                disabled={isBulkResolving}
                onClick={handleKeepAllTarget}
                title="本次不覆盖目标文件，保留目标端当前内容"
              >
                <Icon name="history" size={13} />
                <span>
                  {isBulkResolving ? '正在批量处理...' : `全部保留目标 (${keepTargetConflicts.length})`}
                </span>
              </button>
            )}
          </div>
        </div>
      )}

      {/* 按目标分组展示 */}
      {groups.map((group) => {
        const target = targets.find((t) => t.physicalTargetId === group.physicalTargetId);
        const adapterId = target?.adapterId || 'custom';
        const displayName = target?.displayName || '目标目录';

        return (
          <section key={group.physicalTargetId} className="plan-group">
            <div className="plan-group-title">
              <ToolIcon adapterId={adapterId} />
              <strong>{displayName}</strong>
              <span className="spacer" />
              <small>{group.items.length} 项</small>
            </div>
            <p className="plan-group-path mono">{group.targetPath}</p>

            {target?.availability === 'will_create' && (
              <p className="info-foot" style={{ margin: '0 0 10px' }}>
                目标目录将在执行时自动创建。
              </p>
            )}

            {/* 各技能项目折叠 */}
            {group.items.map((item) => {
              const isOpenDefault = item.action !== 'skip' || Boolean(item.conflict);

              return (
                <details
                  key={item.itemId}
                  className="plan-item"
                  open={isOpenDefault}
                >
                  <summary>
                    <Icon name="chevron" size={13} />
                    <strong>{item.skillName}</strong>
                    {item.conflict ? (
                      <span className="action-badge update">需处理</span>
                    ) : (
                      <ActionBadge action={item.action} />
                    )}
                  </summary>

                  <div className="plan-files">
                    {/* 文件变更列表 */}
                    {item.fileChanges.map((f, idx) => (
                      <div key={idx} className="file-change">
                        <span className={`change-mark ${f.kind}`}>
                          {f.kind === 'add' ? '+' : f.kind === 'delete' ? '−' : '~'}
                        </span>
                        <code>{f.relPath}</code>
                        <small>
                          {f.kind === 'add'
                            ? '新增'
                            : f.kind === 'delete'
                            ? '删除'
                            : '更新'}
                        </small>
                        <span>{formatBytes(f.bytes)}</span>
                      </div>
                    ))}

                    {item.needsBackup && (
                      <p className="backup-inline">
                        <Icon name="shield" size={11} />
                        <span>先保存当前内容备份，再执行此项操作</span>
                      </p>
                    )}

                    {item.excludedByIgnore.length > 0 && (
                      <span className="file-list-label">
                        已按排除规则忽略：
                        {item.excludedByIgnore.map((f) => f.relPath).join('、')}
                      </span>
                    )}

                    <span className="file-list-label mono">{item.targetPath}</span>
                  </div>

                  {/* 冲突决策卡片 */}
                  {item.conflict && (
                    <div className="conflict-box">
                      <p>{item.conflict.message}</p>
                      <div className="conflict-options">
                        {item.conflict.availableChoices.map((choice) => (
                          <button
                            key={choice}
                            type="button"
                            className={`button small ${
                              choice === 'keep_target' ? 'secondary' : 'ghost'
                            }`}
                            disabled={resolvingItemId === item.itemId}
                            onClick={() => handleResolveConflict(item, choice)}
                          >
                            {CHOICE_LABELS[choice] || choice}
                          </button>
                        ))}
                      </div>
                    </div>
                  )}

                  {item.decision && (
                    <div className="conflict-box resolved">
                      已选择：{CHOICE_LABELS[item.decision] || item.decision}
                    </div>
                  )}

                  {item.blockedReason && (
                    <div className="blocked-info">{item.blockedReason}</div>
                  )}
                </details>
              );
            })}

            {group.sharedWithTools.length > 1 && (
              <p className="info-foot">
                此目录也可能被{' '}
                {group.sharedWithTools.filter((n) => n !== displayName).join('、')}{' '}
                读取。
              </p>
            )}
          </section>
        );
      })}

      {isRestore && (
        <div className="quiet-notice">
          <Icon name="history" size={16} />
          <div>恢复只改变目标端内容，源库保持不变。恢复后的相关映射将自动暂停。</div>
        </div>
      )}

      <p className="info-foot">
        仅展示文件变化清单。{summary.skipCount} 项跳过，{summary.conflictCount}{' '}
        项冲突未纳入，{summary.blockedCount} 项阻塞。
      </p>
    </Drawer>
  );
};
