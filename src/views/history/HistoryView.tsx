import React, { useState, useEffect, useMemo } from 'react';
import { useApp } from '../../context/AppContext';
import { Icon } from '../../components/common/Icon';
import { invokeCommand } from '../../services/api';
import type { HistoryTaskSummary, BackupStats } from '../../lib/backend-contract';

export const HistoryView: React.FC = () => {
  const { openDrawer, targets, setCurrentTab, showToast } = useApp();

  const [historyList, setHistoryList] = useState<HistoryTaskSummary[]>([]);
  const [backupStats, setBackupStats] = useState<BackupStats | null>(null);
  const [filterStatus, setFilterStatus] = useState<string>('');
  const [filterTargetId, setFilterTargetId] = useState<string>('');
  const [loading, setLoading] = useState(false);

  const fetchHistoryAndStats = async () => {
    setLoading(true);
    try {
      const [histRes, statsRes] = await Promise.all([
        invokeCommand('list_history', {
          query: {
            status: filterStatus ? (filterStatus as any) : undefined,
            targetId: filterTargetId ? filterTargetId : undefined,
          },
        }),
        invokeCommand('get_backup_stats'),
      ]);
      setHistoryList(histRes?.items || []);
      setBackupStats(statsRes || null);
    } catch (e: any) {
      showToast(`获取历史记录失败: ${e?.message || e}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchHistoryAndStats();
  }, [filterStatus, filterTargetId]);

  const formatSize = (bytes: number) => {
    if (bytes >= 1073741824) return `${(bytes / 1073741824).toFixed(bytes % 1073741824 ? 1 : 0)} GB`;
    if (bytes >= 1048576) return `${(bytes / 1048576).toFixed(bytes % 1048576 ? 1 : 0)} MB`;
    if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${bytes} B`;
  };

  // 按日期智能分组（今天、昨天、日期）
  const groupedHistory = useMemo(() => {
    const todayStr = new Date().toISOString().slice(0, 10);
    const yesterday = new Date(Date.now() - 86400000).toISOString().slice(0, 10);
    const groups: Record<string, HistoryTaskSummary[]> = {};

    for (const h of historyList) {
      const dateKey = (h.startedAt || '').slice(0, 10);
      let label = dateKey;
      if (dateKey === todayStr || dateKey === '2026-09-08') {
        label = '今天';
      } else if (dateKey === yesterday || dateKey === '2026-09-07') {
        label = '昨天';
      }
      if (!groups[label]) {
        groups[label] = [];
      }
      groups[label].push(h);
    }

    return groups;
  }, [historyList]);

  const getHistoryLabel = (h: HistoryTaskSummary) => {
    if (h.kind === 'restore') return `恢复 ${h.counts.success} 项历史内容`;
    if (h.kind === 'remove') return `移除 ${h.counts.success} 项托管内容`;
    if (h.status === 'partial') {
      return `同步完成，${h.counts.conflictPending || h.counts.failed} 项需处理`;
    }
    return `同步 ${h.counts.success} 个目标技能`;
  };

  const usagePercent = backupStats
    ? Math.min(100, (backupStats.totalBytes / (backupStats.softCapBytes || 1)) * 100)
    : 0;

  return (
    <div className="page">
      {/* 标题区 */}
      <div className="page-heading">
        <div>
          <div className="eyebrow">YOUR SYNC TIMELINE</div>
          <h1>同步历史</h1>
          <p>每一次更新，都有迹可循。</p>
        </div>
      </div>

      <div className="history-layout">
        {/* 左侧时间线 */}
        <section>
          <div className="history-filters">
            <select
              value={filterStatus}
              onChange={(e) => setFilterStatus(e.target.value)}
              aria-label="筛选历史状态"
            >
              <option value="">全部状态</option>
              <option value="completed">已完成</option>
              <option value="partial">部分完成</option>
              <option value="failed">失败</option>
              <option value="cancelled">已取消</option>
            </select>

            <select
              value={filterTargetId}
              onChange={(e) => setFilterTargetId(e.target.value)}
              aria-label="筛选同步目标"
            >
              <option value="">全部目标</option>
              {targets.map((t) => (
                <option key={t.targetId} value={t.targetId}>
                  {t.displayName}
                </option>
              ))}
            </select>

            <span className="spacer" />
            <span className="inline-count">{historyList.length} 次记录</span>
          </div>

          {loading && historyList.length === 0 ? (
            <div className="no-results">正在读取历史记录...</div>
          ) : historyList.length === 0 ? (
            <div className="empty-state">
              <div className="empty-illustration">
                <Icon name="history" size={30} />
              </div>
              <h2>暂无同步记录</h2>
              <p>在技能库完成同步后，这里会记录结果和可用的恢复版本。</p>
            </div>
          ) : (
            Object.entries(groupedHistory).map(([label, rows]) => (
              <React.Fragment key={label}>
                <div className="history-group-label">{label}</div>
                {rows.map((h) => {
                  const markClass =
                    h.status === 'partial'
                      ? 'partial'
                      : h.status === 'failed'
                      ? 'failed'
                      : '';
                  const markIcon =
                    h.status === 'partial'
                      ? 'warning'
                      : h.status === 'failed'
                      ? 'close'
                      : 'check';

                  return (
                    <button
                      key={h.taskId}
                      type="button"
                      className="history-entry"
                      onClick={() => openDrawer('history_detail', { taskId: h.taskId })}
                    >
                      <span className={`history-mark ${markClass}`}>
                        <Icon name={markIcon} size={13} />
                      </span>

                      <span className="history-entry-body">
                        <span className="history-entry-title">
                          <strong>{getHistoryLabel(h)}</strong>
                          <time>
                            {h.startedAt
                              ? new Date(h.startedAt).toLocaleTimeString('zh-CN', {
                                  hour: '2-digit',
                                  minute: '2-digit',
                                  hour12: false,
                                })
                              : ''}
                          </time>
                        </span>

                        <p>
                          <span>{h.libraryName || '已归档技能库'}</span>
                          <span>·</span>
                          <span>{((h.durationMs || 0) / 1000).toFixed(1)} 秒</span>
                          <span>·</span>
                          <span>
                            {h.counts.success} 完成
                            {h.counts.skipped ? ` / ${h.counts.skipped} 跳过` : ''}
                            {h.counts.conflictPending
                              ? ` / ${h.counts.conflictPending} 待处理`
                              : ''}
                          </span>
                        </p>

                        <span className="target-pills">
                          {h.targetPaths.map((p: string, pIdx: number) => {
                            const matchedTgt = targets.find(
                              (t) => t.resolvedPath === p
                            );
                            const name = matchedTgt?.displayName || '目标目录';
                            const adapterId = matchedTgt?.adapterId || 'folder';

                            return (
                              <span key={pIdx} className="target-pill">
                                <Icon name={adapterId} size={11} />
                                <span>{name}</span>
                              </span>
                            );
                          })}
                        </span>
                      </span>

                      <span className="entry-chevron">
                        <Icon name="chevron" size={14} />
                      </span>
                    </button>
                  );
                })}
              </React.Fragment>
            ))
          )}
        </section>

        {/* 右侧备份侧栏看板 */}
        <aside className="backup-aside">
          <div className="backup-heading">
            <Icon name="shield" size={15} />
            <span>备份空间</span>
          </div>

          <div className="backup-amount">
            {backupStats
              ? (backupStats.totalBytes / 1048576).toFixed(0)
              : '0'}
            <small>MB</small>
          </div>

          <div className="capacity">
            容量上限 {backupStats ? formatSize(backupStats.softCapBytes) : '2 GB'}
          </div>

          <div className="usage-track">
            <span style={{ width: `${usagePercent}%` }} />
          </div>

          <div className="backup-row">
            <span>保留时间</span>
            <strong>{backupStats?.retentionDays ?? 30} 天</strong>
          </div>

          <div className="backup-row">
            <span>受保护</span>
            <strong>
              {backupStats ? formatSize(backupStats.protectedBytes) : '0 B'}
            </strong>
          </div>

          <div className="backup-row">
            <span>可释放</span>
            <strong>
              {backupStats ? formatSize(backupStats.reclaimableBytes) : '0 B'}
            </strong>
          </div>

          <button
            type="button"
            className="text-link"
            onClick={() => setCurrentTab('settings')}
          >
            <span>调整备份策略</span>
            <Icon name="arrow" size={12} />
          </button>

          <p className="backup-note">
            恢复前会保存当前版本。
            <br />
            可用的备份与恢复操作，
            <br />
            可以在每次任务详情中查看。
          </p>
        </aside>
      </div>
    </div>
  );
};
