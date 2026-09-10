import React from 'react';
import { useApp } from '../../context/AppContext';
import { Modal } from '../common/Modal';
import { Icon } from '../common/Icon';

export const TaskProgressModal: React.FC = () => {
  const {
    isTaskModalOpen,
    setIsTaskModalOpen,
    activeTask,
    cancelCurrentTask,
    setCurrentTab,
    openDrawer,
  } = useApp();

  if (!isTaskModalOpen || !activeTask) {
    return null;
  }

  const { status, counts, items } = activeTask;
  const isRunning = status === 'running' || status === 'queued';
  const done = !isRunning;
  const isFailed = status === 'failed';
  const isCancelled = status === 'cancelled';
  const isPartial = status === 'partial';

  const title = done
    ? isCancelled
      ? '同步任务已取消'
      : isFailed
      ? '同步任务执行失败'
      : isPartial
      ? '同步任务部分完成'
      : '同步任务已完成'
    : '正在同步技能';

  const runningItem = items.find((i) => i.status === 'running') || items[0];
  const summaryParts: string[] = [];
  if (counts.success > 0) summaryParts.push(`${counts.success} 成功`);
  if (counts.failed > 0) summaryParts.push(`${counts.failed} 失败`);
  if (counts.skipped > 0) summaryParts.push(`${counts.skipped} 跳过`);
  if (counts.conflictPending > 0) summaryParts.push(`${counts.conflictPending} 待决策`);
  if (counts.cancelled > 0) summaryParts.push(`${counts.cancelled} 已取消`);

  const message = done
    ? summaryParts.join(' · ') || '任务已完成'
    : runningItem
    ? `正在处理：${runningItem.skillName}`
    : '正在准备目标文件，请稍候。';

  const percent =
    counts.total > 0 ? Math.min(100, Math.round((counts.success / counts.total) * 100)) : done ? 100 : 15;

  const failedItems = items.filter((it) => it.status === 'failed' || Boolean(it.error));

  const handleViewRecord = () => {
    setIsTaskModalOpen(false);
    setCurrentTab('history');
    if (activeTask.taskId) {
      openDrawer('history_detail', { taskId: activeTask.taskId });
    }
  };

  return (
    <Modal
      isOpen={isTaskModalOpen}
      onClose={() => {
        if (done) setIsTaskModalOpen(false);
      }}
      title="任务进度"
      subtitle="同步任务执行中，已自动保存修改前的前置备份。"
      showCloseButton={done}
      footer={
        done ? (
          <>
            <button
              type="button"
              className="button secondary"
              onClick={handleViewRecord}
            >
              <Icon name="history" size={14} />
              <span>查看记录</span>
            </button>
            <button
              type="button"
              className="button primary"
              onClick={() => setIsTaskModalOpen(false)}
            >
              完成
            </button>
          </>
        ) : (
          <button
            type="button"
            className="button secondary"
            onClick={cancelCurrentTask}
          >
            取消任务
          </button>
        )
      }
    >
      <div className="task-center">
        <div
          className={`task-orbit ${
            done
              ? isFailed
                ? 'failed'
                : isCancelled
                ? 'cancelled'
                : 'done'
              : ''
          }`}
        >
          <Icon
            name={
              done
                ? isFailed
                  ? 'warning'
                  : isCancelled
                  ? 'close'
                  : 'check'
                : 'layers'
            }
            size={28}
          />
        </div>
        <h2>{title}</h2>
        <p>{message}</p>
      </div>

      <div className="task-progress">
        <span
          style={{
            width: `${percent}%`,
            backgroundColor: isFailed ? '#ef4444' : isCancelled ? '#94a3b8' : undefined,
          }}
        />
      </div>

      <div className="task-phase">
        <span>{done ? '任务结束' : '正在分发与校验内容'}</span>
        <span>
          {counts.success} / {counts.total} 项
        </span>
      </div>

      {done && failedItems.length > 0 && (
        <div className="mt-4 p-3 rounded-xl border border-rose-200/80 bg-rose-50/70 text-left max-h-36 overflow-y-auto">
          <div className="text-xs font-semibold text-rose-800 mb-1 flex items-center gap-1.5">
            <Icon name="warning" size={13} />
            <span>失败项目 ({failedItems.length})</span>
          </div>
          <div className="space-y-1.5">
            {failedItems.map((it) => (
              <div key={it.itemId} className="text-xs text-rose-700 leading-snug">
                <span className="font-medium text-rose-900">{it.skillName}</span>
                {it.targetPath && <span className="text-[11px] text-rose-500 ml-1.5">({it.targetPath})</span>}:{' '}
                <span>{it.error?.message || '执行失败'}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </Modal>
  );
};
