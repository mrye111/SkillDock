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

  const title = done
    ? status === 'cancelled'
      ? '同步任务已取消'
      : status === 'partial'
      ? '同步任务部分完成'
      : '同步任务已完成'
    : '正在同步技能';

  const runningItem = items.find((i) => i.status === 'running') || items[0];
  const message = done
    ? `${counts.success} 成功 · ${counts.skipped} 跳过 · ${counts.conflictPending} 待处理`
    : runningItem
    ? `正在处理：${runningItem.skillName}`
    : '正在准备目标文件，请稍候。';

  const percent =
    counts.total > 0 ? Math.min(100, Math.round((counts.success / counts.total) * 100)) : done ? 100 : 15;

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
        <div className={`task-orbit ${done ? 'done' : ''}`}>
          <Icon name={done ? 'check' : 'layers'} size={28} />
        </div>
        <h2>{title}</h2>
        <p>{message}</p>
      </div>

      <div className="task-progress">
        <span style={{ width: `${percent}%` }} />
      </div>

      <div className="task-phase">
        <span>{done ? '任务结束' : '正在分发与校验内容'}</span>
        <span>
          {counts.success} / {counts.total} 项
        </span>
      </div>
    </Modal>
  );
};
