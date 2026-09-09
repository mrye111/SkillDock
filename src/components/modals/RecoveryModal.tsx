import React from 'react';
import { useApp } from '../../context/AppContext';
import { Modal } from '../common/Modal';
import { Icon } from '../common/Icon';

export const RecoveryModal: React.FC = () => {
  const { recoveryRequired, clearRecoveryRequired, reloadCurrentLibrary } = useApp();

  if (!recoveryRequired) return null;

  const handleDone = () => {
    clearRecoveryRequired();
    reloadCurrentLibrary();
  };

  return (
    <Modal
      isOpen={!!recoveryRequired}
      onClose={clearRecoveryRequired}
      title="检测到未完成的同步事务"
      subtitle="前置备份已完备保存，原内容未丢失。"
      footer={
        <button
          type="button"
          onClick={handleDone}
          className="button primary"
        >
          <Icon name="check" size={14} />
          <span>已知悉，刷新并继续使用</span>
        </button>
      }
    >
      <div className="error-box">
        <strong>目标路径存在事务残留</strong>
        <p className="mt-1">原因：{recoveryRequired.reason}</p>
        <p className="mt-1 mono">目标位置: {recoveryRequired.targetPath}</p>
        {recoveryRequired.skillName && (
          <p className="mt-1">关联技能: {recoveryRequired.skillName}</p>
        )}
      </div>

      <p className="text-xs text-muted leading-relaxed mt-3">
        SkillDock 在写入前已保存修改前的内容备份，不会导致文件损坏。点击下方按钮后，将自动重置状态并刷新技能状态。
      </p>
    </Modal>
  );
};
