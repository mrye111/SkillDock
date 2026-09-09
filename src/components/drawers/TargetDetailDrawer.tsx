import React from 'react';
import { useApp } from '../../context/AppContext';
import { Drawer } from '../common/Drawer';
import { Icon, ToolIcon } from '../common/Icon';
import { invokeCommand } from '../../services/api';
import type { TargetView } from '../../lib/backend-contract';

export const TargetDetailDrawer: React.FC = () => {
  const { drawer, closeDrawer, removeTarget, showToast } = useApp();

  const target: TargetView | undefined =
    drawer.type === 'target_detail' ? drawer.data : undefined;

  if (drawer.type !== 'target_detail' || !target) {
    return null;
  }

  const handleOpenPath = async () => {
    try {
      await invokeCommand('open_registered_path', {
        kind: 'target',
        id: target.targetId,
      });
      showToast('已在文件管理器中打开目标目录', 'info');
    } catch (e: any) {
      showToast(`打开目标目录失败: ${e?.message || e}`, 'error');
    }
  };

  const handleRemove = async () => {
    if (
      window.confirm(
        `确定要移除目标「${target.displayName}」的配置吗？（已同步的文件会保留在磁盘上）`
      )
    ) {
      await removeTarget(target.targetId);
      closeDrawer();
    }
  };

  const availabilityMap: Record<string, string> = {
    exists: '目录可用',
    will_create: '同步时创建',
    no_permission: '需要写入权限',
    invalid_path: '路径不可用',
    unsupported_location: '不支持的位置',
  };

  return (
    <Drawer
      isOpen={drawer.type === 'target_detail'}
      onClose={closeDrawer}
      title={target.displayName}
      subtitle="目录配置与可用状态"
      footer={
        <>
          <span />
          <button
            type="button"
            className="button secondary"
            onClick={handleOpenPath}
          >
            <Icon name="external" size={14} />
            <span>打开目标目录</span>
          </button>
        </>
      }
    >
      <div className="tool-name-line" style={{ marginBottom: 24 }}>
        <ToolIcon adapterId={target.adapterId} large />
        <span className={`availability ${target.availability}`}>
          <span className="status-dot" />
          <span>{availabilityMap[target.availability] || target.availability}</span>
        </span>
      </div>

      <section className="detail-section">
        <dl className="key-values">
          <dt>作用域</dt>
          <dd>
            {target.scope === 'user'
              ? '用户级'
              : target.scope === 'project'
              ? '项目级'
              : '自定义目录'}
          </dd>
          <dt>完整路径</dt>
          <dd>
            <code>{target.resolvedPath}</code>
          </dd>
          <dt>路径来源</dt>
          <dd>{target.resolutionSource}</dd>
          <dt>关联技能</dt>
          <dd>{target.mappedSkillCount} 个</dd>
          <dt>关联工具</dt>
          <dd>{target.sharedWithTools.join('、')}</dd>
        </dl>
      </section>

      <div className="quiet-notice">
        <Icon name="info" size={16} />
        <div>
          目录可用仅表示文件位置可访问。实际分发按物理目录合并，同一目录只写入一次。
        </div>
      </div>

      <div className="danger-zone">
        <h3>移除目标配置</h3>
        <p>停用相关映射，保留已经同步到目标的所有磁盘内容。</p>
        <button
          type="button"
          className="button danger small"
          onClick={handleRemove}
        >
          <Icon name="trash" size={12} />
          <span>移除目标配置</span>
        </button>
      </div>
    </Drawer>
  );
};
