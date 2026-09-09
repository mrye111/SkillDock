import React, { useState, useEffect } from 'react';
import { useApp } from '../../context/AppContext';
import { Drawer } from '../common/Drawer';
import { Icon } from '../common/Icon';
import { invokeCommand } from '../../services/api';
import type { TaskSnapshot, TaskItemView } from '../../lib/backend-contract';

export const HistoryDetailDrawer: React.FC = () => {
  const {
    drawer,
    closeDrawer,
    openDrawer,
    setCurrentPlan,
    showToast,
  } = useApp();

  const [taskDetail, setTaskDetail] = useState<TaskSnapshot | null>(null);
  const [selectedItems, setSelectedItems] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(false);

  const taskId = drawer.type === 'history_detail' ? drawer.data?.taskId : null;

  useEffect(() => {
    if (!taskId) {
      setTaskDetail(null);
      setSelectedItems(new Set());
      return;
    }

    setLoading(true);
    invokeCommand('get_task_detail', { taskId })
      .then((detail) => {
        setTaskDetail(detail);
        // 默认勾选所有成功且有 snapshotId 的项
        const initial = new Set<string>();
        if (detail?.items) {
          for (const item of detail.items) {
            if (item.status === 'success') {
              initial.add(item.itemId);
            }
          }
        }
        setSelectedItems(initial);
      })
      .catch((e) => {
        showToast(`加载任务详情失败: ${e?.message || e}`, 'error');
      })
      .finally(() => {
        setLoading(false);
      });
  }, [taskId, showToast]);

  if (drawer.type !== 'history_detail' || !taskId) {
    return null;
  }

  const toggleItem = (itemId: string) => {
    setSelectedItems((prev) => {
      const next = new Set(prev);
      if (next.has(itemId)) {
        next.delete(itemId);
      } else {
        next.add(itemId);
      }
      return next;
    });
  };

  const handlePreviewRestore = async () => {
    if (selectedItems.size === 0) {
      showToast('请至少选择一项要恢复的目标内容', 'warning');
      return;
    }

    try {
      const restorePlan = await invokeCommand('create_restore_plan', {
        input: {
          taskId,
          itemIds: Array.from(selectedItems),
        },
      });
      setCurrentPlan(restorePlan);
      openDrawer('sync_plan');
    } catch (e: any) {
      showToast(`生成恢复计划失败: ${e?.message || e}`, 'error');
    }
  };

  const statusLabel =
    taskDetail?.status === 'completed'
      ? '已完成'
      : taskDetail?.status === 'partial'
      ? '部分完成'
      : taskDetail?.status === 'failed'
      ? '失败'
      : taskDetail?.status || '进行中';

  return (
    <Drawer
      isOpen={drawer.type === 'history_detail'}
      onClose={closeDrawer}
      title="任务详情"
      subtitle={
        taskDetail?.startedAt
          ? `${new Date(taskDetail.startedAt).toLocaleString('zh-CN', {
              hour12: false,
            })} · ${statusLabel}`
          : '尚未开始'
      }
      footer={
        <>
          <small>只恢复目标端，源库保持不变</small>
          <button
            type="button"
            className="button primary"
            disabled={selectedItems.size === 0}
            onClick={handlePreviewRestore}
          >
            <Icon name="history" size={14} />
            <span>预览恢复 ({selectedItems.size})</span>
          </button>
        </>
      }
    >
      {loading || !taskDetail ? (
        <div className="no-results">正在读取任务快照明细...</div>
      ) : (
        <>
          {/* 三栏计数 */}
          <div className="drawer-summary">
            <div>
              <strong>{taskDetail.counts.success}</strong>
              <small>成功</small>
            </div>
            <div>
              <strong>{taskDetail.counts.skipped}</strong>
              <small>跳过</small>
            </div>
            <div>
              <strong>
                {taskDetail.counts.conflictPending + taskDetail.counts.failed}
              </strong>
              <small>需处理</small>
            </div>
          </div>

          <section className="detail-section">
            <h3>选择要恢复的目标内容</h3>
            <div>
              {taskDetail.items.map((item: TaskItemView) => {
                const isSelected = selectedItems.has(item.itemId);
                const canRestore = item.status === 'success' || Boolean(item.snapshotId);

                return (
                  <label key={item.itemId} className="history-detail-item">
                    <input
                      type="checkbox"
                      checked={isSelected}
                      disabled={!canRestore}
                      onChange={() => toggleItem(item.itemId)}
                    />
                    <div>
                      <strong>{item.skillName}</strong>
                      <p className="mono">{item.targetPath}</p>
                      <p>
                        {item.status === 'success'
                          ? '已完成'
                          : item.status === 'conflict_pending'
                          ? '冲突未处理'
                          : item.status === 'cancelled'
                          ? '已取消'
                          : item.status === 'skipped'
                          ? '已跳过'
                          : item.status}
                        {item.snapshotId ? ' · 备份可恢复' : ' · 无可用快照'}
                      </p>
                      {item.error && (
                        <p className="error-text">{item.error.message}</p>
                      )}
                    </div>
                    <Icon
                      name={item.status === 'success' ? 'check' : 'info'}
                      size={13}
                    />
                  </label>
                );
              })}
            </div>
          </section>

          <div className="quiet-notice">
            <Icon name="shield" size={16} />
            <div>
              恢复之前先预览。目标中的后续修改会作为冲突处理，恢复时会保存当前版本备份。
            </div>
          </div>
        </>
      )}
    </Drawer>
  );
};
