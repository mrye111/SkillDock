import React from 'react';
import type { MatrixCellState, ValidationStatus } from '../../lib/backend-contract';
import { Icon } from './Icon';

export const STATE_LABELS: Record<MatrixCellState, string> = {
  no_mapping: '未选择',
  to_add: '待新增',
  same_content_unmanaged: '可接管',
  unmanaged_conflict: '同名冲突',
  synced: '已同步',
  source_updated: '待更新',
  target_modified: '本地修改',
  both_modified: '双方修改',
  aligned_externally: '内容一致',
  target_deleted: '目标已删除',
  source_removed: '源已移除',
  ownership_conflict: '来源冲突',
  invalid: '校验异常',
  unsupported: '不支持',
  paused: '已暂停',
};

export const CONFLICT_STATES: MatrixCellState[] = [
  'target_modified',
  'both_modified',
  'unmanaged_conflict',
  'ownership_conflict',
];

/**
 * 判定一个目标单元格是否处于真正活动的同步映射状态（已开启且未暂停）
 * 排除未选择 (no_mapping) 与已取消关联/暂停 (paused)
 */
export const isCellActiveMapped = (
  cell?: { mappingId?: string | null; state?: MatrixCellState } | null
): boolean => {
  if (!cell || !cell.mappingId) return false;
  return cell.state !== 'paused' && cell.state !== 'no_mapping';
};

export interface MatrixBadgeProps {
  state: MatrixCellState;
  detail?: string | null;
  className?: string;
  onClick?: (e: React.MouseEvent) => void;
  asButton?: boolean;
}

export const MatrixBadge: React.FC<MatrixBadgeProps> = ({
  state,
  detail,
  className = '',
  onClick,
  asButton = true,
}) => {
  const label = STATE_LABELS[state] || state;
  const isConflict = CONFLICT_STATES.includes(state);

  const iconName =
    state === 'synced' || state === 'aligned_externally'
      ? 'check'
      : state === 'source_updated'
      ? 'upload'
      : state === 'to_add'
      ? 'plus'
      : state === 'invalid' || state === 'unsupported'
      ? 'info'
      : isConflict
      ? 'warning'
      : 'minus';

  const content = (
    <>
      <Icon name={iconName} size={12} />
      <span>{label}</span>
    </>
  );

  const defaultTooltips: Partial<Record<MatrixCellState, string>> = {
    paused: '已停止自动同步：目标端仍保留上次同步的文件，不再接收更新。如需彻底清理目标文件，可在抽屉中执行“预览移除”。',
    no_mapping: '未选择此目标：目标目录尚未配置该技能。',
    synced: '已同步：源库与目标目录文件内容完全一致。',
    to_add: '待同步：目标目录尚未存在该技能，待同步写入。',
    source_updated: '待更新：源库有新变动，待更新到目标目录。',
  };

  const title = detail || defaultTooltips[state] || label;

  if (asButton || onClick) {
    return (
      <button
        type="button"
        className={`cell-state ${state} ${className}`}
        onClick={onClick}
        title={title}
      >
        {content}
      </button>
    );
  }

  return (
    <span className={`cell-state ${state} ${className}`} title={title}>
      {content}
    </span>
  );
};

export const ValidationBadge: React.FC<{
  status: ValidationStatus;
  issueCount?: number;
  className?: string;
}> = ({ status, issueCount = 0, className = '' }) => {
  if (status === 'valid') {
    return (
      <span className={`subtle-badge text-status-success ${className}`}>
        校验通过
      </span>
    );
  }
  return (
    <span className={`subtle-badge text-status-danger ${className}`}>
      校验异常{issueCount > 0 ? ` (${issueCount})` : ''}
    </span>
  );
};

export const ActionBadge: React.FC<{
  action: string;
  className?: string;
}> = ({ action, className = '' }) => {
  const labels: Record<string, string> = {
    create: '新增',
    update: '更新',
    adopt: '接管',
    overwrite: '覆盖',
    take_over: '接管并覆盖',
    reinstall: '重新安装',
    remove: '移除',
    restore: '恢复',
    skip: '跳过',
    blocked: '不可执行',
  };

  return (
    <span className={`action-badge ${action} ${className}`}>
      {labels[action] || action}
    </span>
  );
};
