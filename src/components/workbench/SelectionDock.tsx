import React, { useState } from 'react';
import { useApp } from '../../context/AppContext';
import { Icon } from '../common/Icon';
import { isCellActiveMapped } from '../common/Badge';

export const SelectionDock: React.FC = () => {
  const {
    activeLibrary,
    targets,
    selectedSkillIds,
    clearSelection,
    requestSyncPreview,
    batchAssignTarget,
    setIsAddTargetModalOpen,
  } = useApp();

  const [isAssigning, setIsAssigning] = useState(false);

  if (!activeLibrary || activeLibrary.skills.length === 0) {
    return null;
  }

  const selectedSkills = activeLibrary.skills.filter((s) =>
    selectedSkillIds.has(s.skillId)
  );

  const activeMappings = selectedSkills.flatMap((s) =>
    Object.values(s.targets).filter((c) => isCellActiveMapped(c))
  );

  const physicalTargetSet = new Set(
    selectedSkills.flatMap((s) =>
      Object.entries(s.targets)
        .filter(([, c]) => isCellActiveMapped(c))
        .map(([ptId]) => ptId)
    )
  );

  const physicalTargets = targets.filter(
    (t, i, arr) => arr.findIndex((x) => x.physicalTargetId === t.physicalTargetId) === i
  );

  const hasSelection = selectedSkills.length > 0;

  const handleToggleTargetForSelection = async (ptId: string, shouldEnable: boolean) => {
    setIsAssigning(true);
    try {
      await batchAssignTarget(Array.from(selectedSkillIds), ptId, shouldEnable);
    } finally {
      setIsAssigning(false);
    }
  };

  if (hasSelection) {
    return (
      <div className="selection-dock">
        <span className="selection-count">{selectedSkills.length}</span>
        <div className="selection-copy">
          <strong>已选择 {selectedSkills.length} 个技能</strong>
          <small>
            {physicalTargetSet.size > 0
              ? `${physicalTargetSet.size} 个物理目标已开启 · 执行前先查看具体变化`
              : '所选技能尚未关联目标，请点击右侧目标一键开启'}
          </small>
        </div>

        {/* 目标快捷开关标签组：方便用户直接管理所选技能关联的目标 */}
        {physicalTargets.length > 0 && (
          <div className="flex items-center gap-1.5 flex-wrap max-w-[480px]">
            <span className="text-[11px] text-slate-400 font-medium mr-1 select-none">关联目标:</span>
            {physicalTargets.map((t) => {
              const count = selectedSkills.filter((s) =>
                isCellActiveMapped(s.targets[t.physicalTargetId])
              ).length;
              const isAll = count === selectedSkills.length;
              const isSome = count > 0 && !isAll;

              return (
                <button
                  key={t.physicalTargetId}
                  type="button"
                  className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md text-[11px] font-medium border transition ${
                    isAll
                      ? 'bg-blue-50 border-blue-200 text-blue-700 hover:bg-blue-100 hover:border-blue-300'
                      : isSome
                      ? 'bg-slate-50 border-blue-200 text-blue-600 hover:bg-blue-50'
                      : 'bg-white border-slate-200 text-slate-500 hover:border-slate-300 hover:text-slate-800'
                  }`}
                  disabled={isAssigning}
                  onClick={() => handleToggleTargetForSelection(t.physicalTargetId, !isAll)}
                  title={
                    isAll
                      ? `点击取消所选技能对 ${t.displayName} 的关联`
                      : `点击将所选技能关联到 ${t.displayName} (${count}/${selectedSkills.length})`
                  }
                >
                  <span
                    className={`w-1.5 h-1.5 rounded-full ${
                      isAll ? 'bg-blue-600' : isSome ? 'bg-blue-400' : 'bg-slate-300'
                    }`}
                  />
                  <span>{t.displayName}</span>
                  {count > 0 && (
                    <span className="text-[10px] opacity-75 font-mono">
                      {isAll ? '✓' : `${count}`}
                    </span>
                  )}
                </button>
              );
            })}
          </div>
        )}

        {/* 如果连目标都未添加 */}
        {physicalTargets.length === 0 && (
          <button
            type="button"
            className="button secondary small"
            onClick={() => setIsAddTargetModalOpen(true)}
          >
            <Icon name="plus" size={12} />
            <span>添加同步目标</span>
          </button>
        )}

        <button
          type="button"
          className="button ghost"
          onClick={clearSelection}
        >
          取消选择
        </button>

        <button
          type="button"
          className="button primary"
          disabled={activeMappings.length === 0}
          onClick={() => requestSyncPreview()}
          title={
            activeMappings.length === 0
              ? '请先为所选技能开启至少一个同步目标'
              : '预览所选技能的同步操作 (Ctrl+Enter)'
          }
        >
          <span>预览同步</span>
          <Icon name="arrow" size={14} />
        </button>
      </div>
    );
  }

  return (
    <div className="selection-dock selection-empty">
      <div className="selection-copy">
        <strong>选择技能，查看同步到各目标的变化。</strong>
      </div>
      <kbd>Ctrl + Enter</kbd>
      <button
        type="button"
        className="button primary"
        disabled
      >
        <span>预览同步</span>
        <Icon name="arrow" size={14} />
      </button>
    </div>
  );
};
