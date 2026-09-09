import React, { useMemo } from 'react';
import { useApp } from '../../context/AppContext';
import { MatrixBadge, isCellActiveMapped } from '../../components/common/Badge';
import { Icon, ToolIcon } from '../../components/common/Icon';
import type { SkillView, TargetView } from '../../lib/backend-contract';

const GLYPHS = ['code', 'document', 'rocket', 'pen', 'brackets', 'flask', 'grid'];
const COLORS = ['', 'sand', 'mint', 'lilac', '', 'mint', 'lilac', 'rose'];

export const SyncMatrixTable: React.FC = () => {
  const {
    activeLibrary,
    targets,
    selectedSkillIds,
    toggleSelectSkill,
    selectAllSkills,
    searchQuery,
    statusFilter,
    openDrawer,
    requestSyncPreview,
    batchAssignTarget,
  } = useApp();

  // 物理目标去重
  const physicalTargets: TargetView[] = useMemo(() => {
    return targets.filter(
      (t, i, arr) => arr.findIndex((x) => x.physicalTargetId === t.physicalTargetId) === i
    );
  }, [targets]);

  // 筛选技能列表
  const filteredSkills: SkillView[] = useMemo(() => {
    if (!activeLibrary) return [];
    const q = searchQuery.trim().toLowerCase();

    return activeLibrary.skills.filter((s) => {
      // 搜索匹配
      if (q) {
        const text = `${s.name || ''} ${s.description || ''} ${s.relPath}`.toLowerCase();
        if (!text.includes(q)) return false;
      }

      // 状态筛选
      if (statusFilter === 'pending') {
        return Object.values(s.targets).some((c) =>
          ['source_updated', 'to_add'].includes(c.state)
        );
      }
      if (statusFilter === 'conflict') {
        return Object.values(s.targets).some((c) =>
          ['target_modified', 'both_modified', 'unmanaged_conflict', 'ownership_conflict'].includes(
            c.state
          )
        );
      }
      if (statusFilter === 'invalid') {
        return s.validation.status !== 'valid';
      }

      return true;
    });
  }, [activeLibrary, searchQuery, statusFilter]);

  const selectableSkills = filteredSkills.filter((s) => s.validation.status === 'valid');
  const isAllSelected =
    selectableSkills.length > 0 &&
    selectableSkills.every((s) => selectedSkillIds.has(s.skillId));

  const handleCellClick = (skill: SkillView, target: TargetView) => {
    const cell = skill.targets[target.physicalTargetId];
    if (isCellActiveMapped(cell) && skill.validation.status === 'valid') {
      requestSyncPreview([cell!.mappingId!]);
    } else {
      openDrawer('skill_detail', skill);
    }
  };

  return (
    <div>
      <div className="table-container">
        <table className="skills-table">
          <thead>
            <tr>
              <th className="check">
                <input
                  type="checkbox"
                  aria-label="全选当前有效技能"
                  checked={isAllSelected}
                  disabled={selectableSkills.length === 0}
                  onChange={(e) => selectAllSkills(e.target.checked)}
                />
              </th>
              <th className="skill-col">技能名称</th>
              {physicalTargets.map((t) => {
                // 如果用户有勾选特定技能，则以所选技能为准；未勾选特定技能时以全部有效技能为准
                const targetSkills =
                  selectedSkillIds.size > 0
                    ? selectableSkills.filter((s) => selectedSkillIds.has(s.skillId))
                    : selectableSkills;

                // 统计已处于有效活动映射状态的技能数量
                const activeCount = targetSkills.filter((s) =>
                  isCellActiveMapped(s.targets[t.physicalTargetId])
                ).length;
                const totalCount = targetSkills.length;
                const isAllMapped = totalCount > 0 && activeCount === totalCount;
                const isIndeterminate = activeCount > 0 && activeCount < totalCount;

                const countScope = selectedSkillIds.size > 0 ? `选中的 ${selectedSkillIds.size} 项` : '全部有效';
                const headerTitle = isAllMapped
                  ? `${t.displayName} · 已全部关联 (${activeCount}/${totalCount}) · 点击取消关联`
                  : isIndeterminate
                  ? `${t.displayName} · 部分关联 (${activeCount}/${totalCount}) · 点击关联${countScope}`
                  : `${t.displayName} · 未关联 (0/${totalCount}) · 点击为${countScope}开启关联`;

                return (
                  <th key={t.physicalTargetId} className="agent-col" title={headerTitle}>
                    <div className="flex items-center gap-2 pr-1 min-w-0">
                      <input
                        type="checkbox"
                        ref={(el) => {
                          if (el) el.indeterminate = isIndeterminate;
                        }}
                        checked={isAllMapped}
                        disabled={totalCount === 0}
                        aria-label={`切换 ${t.displayName} 的关联状态 (${activeCount}/${totalCount})`}
                        onChange={() => {
                          const shouldEnable = !isAllMapped;
                          const targetSkillIds = targetSkills.map((s) => s.skillId);
                          batchAssignTarget(targetSkillIds, t.physicalTargetId, shouldEnable);
                        }}
                        className="cursor-pointer flex-none"
                      />
                      <span className="agent-heading min-w-0 truncate">
                        <ToolIcon adapterId={t.adapterId} />
                        <span className="truncate">{t.displayName}</span>
                      </span>
                    </div>
                  </th>
                );
              })}
            </tr>
          </thead>
          <tbody>
            {filteredSkills.map((skill, idx) => {
              const glyph = GLYPHS[idx % GLYPHS.length];
              const colorClass = COLORS[idx % COLORS.length];
              const isSelected = selectedSkillIds.has(skill.skillId);
              const isValid = skill.validation.status === 'valid';

              return (
                <tr
                  key={skill.skillId}
                  className={isSelected ? 'selected' : ''}
                >
                  <td className="check">
                    <input
                      type="checkbox"
                      aria-label={`选择技能 ${skill.name || skill.relPath}`}
                      checked={isSelected}
                      disabled={!isValid}
                      onChange={() => toggleSelectSkill(skill.skillId)}
                    />
                  </td>

                  <td>
                    <button
                      type="button"
                      className="skill-identity"
                      onClick={() => openDrawer('skill_detail', skill)}
                    >
                      <span className={`skill-symbol ${colorClass}`}>
                        <Icon name={glyph} size={17} />
                      </span>
                      <span>
                        <span className="skill-name">
                          {skill.name || skill.relPath}
                        </span>
                        <span className="skill-desc">
                          {skill.description || '元数据尚未完善'}
                        </span>
                      </span>
                    </button>
                  </td>

                  {physicalTargets.map((t) => {
                    const cell = skill.targets[t.physicalTargetId];
                    return (
                      <td key={t.physicalTargetId}>
                        <MatrixBadge
                          state={cell?.state || 'no_mapping'}
                          detail={cell?.detail}
                          onClick={() => handleCellClick(skill, t)}
                        />
                      </td>
                    );
                  })}
                </tr>
              );
            })}
          </tbody>
        </table>

        {filteredSkills.length === 0 && (
          <div className="no-results">
            没有符合条件的技能。试试其他关键词或筛选条件。
          </div>
        )}
      </div>

      {/* 表格底部信息 */}
      <div className="table-caption select-none">
        <span>
          <Icon name="info" size={12} />
          <span>状态根据文件内容比较；文件已同步不代表 Agent 已加载。</span>
        </span>
        <span>
          {filteredSkills.length} / {activeLibrary?.skills.length || 0} 个技能
        </span>
      </div>
    </div>
  );
};
