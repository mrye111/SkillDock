import React, { useState, useEffect } from 'react';
import { useApp } from '../../context/AppContext';
import { Drawer } from '../common/Drawer';
import { MatrixBadge, isCellActiveMapped } from '../common/Badge';
import { Icon, ToolIcon } from '../common/Icon';
import { invokeCommand } from '../../services/api';
import type { SkillView } from '../../lib/backend-contract';

export const SkillDetailDrawer: React.FC = () => {
  const {
    drawer,
    closeDrawer,
    activeLibrary,
    targets,
    reloadCurrentLibrary,
    requestSyncPreview,
    showToast,
  } = useApp();

  const skill: SkillView | undefined =
    drawer.type === 'skill_detail' ? drawer.data : undefined;

  // 目标勾选状态
  const [selectedTargets, setSelectedTargets] = useState<Set<string>>(new Set());
  const [isSaving, setIsSaving] = useState(false);

  // 物理去重目标
  const physicalTargets = React.useMemo(() => {
    return targets.filter(
      (t, i, arr) => arr.findIndex((x) => x.physicalTargetId === t.physicalTargetId) === i
    );
  }, [targets]);

  useEffect(() => {
    if (skill) {
      const active = new Set<string>();
      for (const [ptId, cell] of Object.entries(skill.targets)) {
        if (cell?.mappingId && cell.state !== 'no_mapping') {
          active.add(ptId);
        }
      }
      setSelectedTargets(active);
    }
  }, [skill]);

  if (drawer.type !== 'skill_detail' || !skill || !activeLibrary) {
    return null;
  }

  const formatBytes = (bytes: number) => {
    if (bytes >= 1073741824) return `${(bytes / 1073741824).toFixed(1)} GB`;
    if (bytes >= 1048576) return `${(bytes / 1048576).toFixed(1)} MB`;
    if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${bytes} B`;
  };

  const handleToggleTarget = (ptId: string) => {
    setSelectedTargets((prev) => {
      const next = new Set(prev);
      if (next.has(ptId)) {
        next.delete(ptId);
      } else {
        next.add(ptId);
      }
      return next;
    });
  };

  // 保存目标选择
  const handleSaveMappings = async () => {
    setIsSaving(true);
    try {
      await invokeCommand('update_mappings', {
        libraryId: activeLibrary.summary.libraryId,
        skillId: skill.skillId,
        physicalTargetIds: Array.from(selectedTargets),
      });
      await reloadCurrentLibrary();
      showToast('已更新技能的目标同步配置', 'success');
    } catch (e: any) {
      showToast(`保存目标失败: ${e?.message || e}`, 'error');
    } finally {
      setIsSaving(false);
    }
  };

  // 预览单个技能同步
  const handlePreviewSkill = async (operation: 'sync' | 'remove' = 'sync') => {
    const mappingIds = Object.values(skill.targets)
      .filter((c) => (operation === 'sync' ? isCellActiveMapped(c) : Boolean(c.mappingId)))
      .map((c) => c.mappingId)
      .filter((id): id is string => Boolean(id));

    if (mappingIds.length === 0) {
      showToast(
        operation === 'sync'
          ? '请先为该技能勾选并保存至少一个同步目标'
          : '该技能没有可供移除的映射记录',
        'warning'
      );
      return;
    }

    await requestSyncPreview(mappingIds, operation);
  };

  // 打开源目录
  const handleOpenSourceDir = async () => {
    try {
      await invokeCommand('open_registered_path', {
        kind: 'library',
        id: activeLibrary.summary.libraryId,
      });
      showToast('已在文件管理器中定位技能目录', 'info');
    } catch (e: any) {
      showToast(`打开目录失败: ${e?.message || e}`, 'error');
    }
  };

  const hasActiveMappings = Object.values(skill.targets).some((c) => isCellActiveMapped(c));
  const hasAnyMappings = Object.values(skill.targets).some((c) => Boolean(c.mappingId));
  const isValid = skill.validation.status === 'valid';

  return (
    <Drawer
      isOpen={drawer.type === 'skill_detail'}
      onClose={closeDrawer}
      title={skill.name || skill.relPath}
      subtitle={`${skill.fileCount} 个文件 · ${formatBytes(skill.totalBytes)}`}
      footer={
        <>
          <button
            type="button"
            className="button ghost"
            onClick={handleOpenSourceDir}
          >
            <Icon name="external" size={12} />
            <span>打开源目录</span>
          </button>
          <button
            type="button"
            className="button primary"
            disabled={!isValid || !hasActiveMappings}
            onClick={() => handlePreviewSkill('sync')}
          >
            <span>预览此技能</span>
            <Icon name="arrow" size={14} />
          </button>
        </>
      }
    >
      <p className="detail-description">{skill.description || '未填写描述。'}</p>

      {/* 校验错误提示 */}
      {skill.validation.issues.length > 0 && (
        <div className="error-box">
          <strong>校验异常：</strong>
          {skill.validation.issues.map((issue, idx) => (
            <div key={idx} className="mt-1">
              {issue.line ? `第 ${issue.line} 行 · ` : ''}
              {issue.message}
            </div>
          ))}
        </div>
      )}

      {/* 技能信息 */}
      <section className="detail-section">
        <h3>技能信息</h3>
        <dl className="key-values">
          <dt>相对路径</dt>
          <dd>
            <code>{skill.relPath}</code>
          </dd>
          <dt>校验状态</dt>
          <dd>{isValid ? '已通过' : '未通过'}</dd>
          <dt>内容更新时间</dt>
          <dd>
            {skill.lastContentChangedAt
              ? new Date(skill.lastContentChangedAt).toLocaleString('zh-CN', {
                  hour12: false,
                })
              : '暂无'}
          </dd>
          <dt>内容摘要</dt>
          <dd>
            <code>{skill.digest || '校验完成后生成'}</code>
          </dd>
        </dl>
      </section>

      {/* 同步到这些目标 */}
      <section className="detail-section">
        <h3>同步到这些目标</h3>
        <div>
          {physicalTargets.map((t) => {
            const cell = skill.targets[t.physicalTargetId];
            const isChecked = selectedTargets.has(t.physicalTargetId);
            return (
              <div key={t.physicalTargetId} className="mapping-row">
                <label>
                  <input
                    type="checkbox"
                    checked={isChecked}
                    disabled={!isValid}
                    onChange={() => handleToggleTarget(t.physicalTargetId)}
                  />
                  <ToolIcon adapterId={t.adapterId} />
                  <span>
                    <strong>{t.displayName}</strong>
                    <small>
                      {t.scope === 'user'
                        ? '用户级'
                        : t.scope === 'project'
                        ? '项目级'
                        : '自定义'}
                    </small>
                  </span>
                </label>
                <MatrixBadge
                  state={cell?.state || 'no_mapping'}
                  detail={cell?.detail}
                  asButton={false}
                />
              </div>
            );
          })}
        </div>

        <div className="row-end">
          <button
            type="button"
            className="button secondary small"
            disabled={!isValid || isSaving}
            onClick={handleSaveMappings}
          >
            {isSaving ? '正在保存...' : '保存目标选择'}
          </button>
        </div>
        <p className="info-foot">取消目标选择只停止映射，不删除目标中的文件。</p>
      </section>

      {/* 危险区：移除托管内容 */}
      <div className="danger-zone">
        <h3>移除托管内容</h3>
        <p>单独生成移除计划，查看具体目标路径和备份要求后再决定。</p>
        <button
          type="button"
          className="button danger small"
          disabled={!hasAnyMappings}
          onClick={() => handlePreviewSkill('remove')}
        >
          预览移除
        </button>
      </div>
    </Drawer>
  );
};
