import React, { useMemo } from 'react';
import { useApp } from '../../context/AppContext';
import { Icon, ToolIcon } from '../../components/common/Icon';
import { invokeCommand } from '../../services/api';
import type { TargetView } from '../../lib/backend-contract';

export const TargetsView: React.FC = () => {
  const {
    targets,
    openDrawer,
    setIsAddTargetModalOpen,
    showToast,
  } = useApp();

  const readyCount = useMemo(() => {
    return targets.filter((t) => t.availability === 'exists').length;
  }, [targets]);

  const physicalTargetCount = useMemo(() => {
    return new Set(targets.map((t) => t.physicalTargetId)).size;
  }, [targets]);

  const handleOpenTargetPath = async (target: TargetView) => {
    try {
      await invokeCommand('open_registered_path', {
        kind: 'target',
        id: target.targetId,
      });
      showToast(`已在文件管理器中打开 ${target.displayName} 目录`, 'info');
    } catch (e: any) {
      showToast(`打开目录失败: ${e?.message || e}`, 'error');
    }
  };

  const availabilityLabels: Record<string, string> = {
    exists: '目录可用',
    will_create: '同步时创建',
    no_permission: '需要写入权限',
    invalid_path: '路径不可用',
    unsupported_location: '不支持的位置',
  };

  return (
    <div className="page">
      {/* 标题区 */}
      <div className="page-heading">
        <div>
          <div className="eyebrow">CONNECTED DESTINATIONS</div>
          <h1>同步目标</h1>
          <p>把技能放到工具能找到的地方。</p>
        </div>
        <button
          type="button"
          className="button secondary"
          onClick={() => setIsAddTargetModalOpen(true)}
        >
          <Icon name="plus" size={14} />
          <span>添加目标</span>
        </button>
      </div>

      {targets.length === 0 ? (
        <div className="empty-state">
          <div className="empty-illustration">
            <Icon name="folder" size={30} />
          </div>
          <h2>连接你的第一个工具</h2>
          <p>选择工具和目录，预览后再同步技能。</p>
          <button
            type="button"
            className="button primary mt-6"
            onClick={() => setIsAddTargetModalOpen(true)}
          >
            <Icon name="plus" size={14} />
            <span>添加目标</span>
          </button>
        </div>
      ) : (
        <>
          {/* 统计横条 */}
          <div className="page-kicker select-none">
            <b>{physicalTargetCount}</b> 个物理目标 <span>·</span> {readyCount} 个目录可用
          </div>

          {/* 目标卡片网格 */}
          <div className="target-grid">
            {targets.map((target) => (
              <article key={target.targetId} className="target-card">
                <div className="target-card-top">
                  <ToolIcon adapterId={target.adapterId} large />
                  <div>
                    <h2>{target.displayName}</h2>
                    <small>
                      {target.scope === 'project'
                        ? '项目级'
                        : target.scope === 'custom'
                        ? '自定义目录'
                        : '用户级'}{' '}
                      <span>·</span> {target.enabled ? '已配置' : '未启用'}
                    </small>
                  </div>
                  <button
                    type="button"
                    className="icon-button"
                    onClick={() => openDrawer('target_detail', target)}
                    aria-label={`${target.displayName} 目录详情`}
                    title="目录详情"
                  >
                    <Icon name="more" size={16} />
                  </button>
                </div>

                {/* 路径条 */}
                <div className="target-card-path">
                  <Icon name="folder" size={13} />
                  <code title={target.resolvedPath}>{target.resolvedPath}</code>
                  <button
                    type="button"
                    className="icon-button"
                    onClick={() => handleOpenTargetPath(target)}
                    aria-label={`打开 ${target.displayName} 目录`}
                    title="打开本地目录"
                  >
                    <Icon name="external" size={12} />
                  </button>
                </div>

                {/* 状态与统计 */}
                <div className="target-detail-line">
                  <span className={`availability ${target.availability}`}>
                    <span className="status-dot" />
                    <span>
                      {availabilityLabels[target.availability] || target.availability}
                    </span>
                  </span>
                  <span>{target.mappedSkillCount} 个关联技能</span>
                </div>

                {/* 卡片底栏 */}
                <div className="target-card-foot">
                  <span>
                    {target.sharedWithTools.length > 1
                      ? `共享读取 · ${target.sharedWithTools.length} 个工具`
                      : '独立目录配置'}
                  </span>
                  <button
                    type="button"
                    className="text-link"
                    onClick={() => openDrawer('target_detail', target)}
                  >
                    <span>目录详情</span>
                    <Icon name="arrow" size={11} />
                  </button>
                </div>
              </article>
            ))}
          </div>

          {/* 底部说明 */}
          <div className="quiet-notice">
            <Icon name="link" size={16} />
            <div>
              <strong>同一目录，只同步一次。</strong>
              <br />
              部分工具会读取兼容目录。实际分发按物理目标合并，目录详情中可查看关联工具。
            </div>
          </div>
        </>
      )}
    </div>
  );
};
