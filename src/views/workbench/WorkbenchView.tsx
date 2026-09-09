import React, { useMemo } from 'react';
import { useApp } from '../../context/AppContext';
import { SyncMatrixTable } from './SyncMatrixTable';
import { Icon } from '../../components/common/Icon';
import { invokeCommand } from '../../services/api';

export const WorkbenchView: React.FC = () => {
  const {
    activeLibrary,
    targets,
    statusFilter,
    setStatusFilter,
    searchQuery,
    setSearchQuery,
    setIsAddLibraryModalOpen,
    setIsLibrariesModalOpen,
    reloadCurrentLibrary,
    showToast,
  } = useApp();

  const physicalTargetCount = useMemo(() => {
    return new Set(targets.map((t) => t.physicalTargetId)).size;
  }, [targets]);

  // 各状态计数
  const counts = useMemo(() => {
    if (!activeLibrary) return { all: 0, pending: 0, conflict: 0, invalid: 0 };
    const skills = activeLibrary.skills;
    return {
      all: skills.length,
      pending: skills.filter((s) =>
        Object.values(s.targets).some((c) =>
          ['source_updated', 'to_add'].includes(c.state)
        )
      ).length,
      conflict: skills.filter((s) =>
        Object.values(s.targets).some((c) =>
          ['target_modified', 'both_modified', 'unmanaged_conflict', 'ownership_conflict'].includes(
            c.state
          )
        )
      ).length,
      invalid: skills.filter((s) => s.validation.status !== 'valid').length,
    };
  }, [activeLibrary]);

  const handleOpenSourceFolder = async () => {
    if (!activeLibrary) return;
    try {
      await invokeCommand('open_registered_path', {
        kind: 'library',
        id: activeLibrary.summary.libraryId,
      });
      showToast('已在文件管理器中定位技能库目录', 'info');
    } catch (e: any) {
      showToast(`打开目录失败: ${e?.message || e}`, 'error');
    }
  };

  return (
    <div className="page">
      {/* 顶部标题区 */}
      <div className="page-heading">
        <div>
          <div className="eyebrow">YOUR SKILLS, IN SYNC</div>
          <h1>技能库</h1>
          <p>一处维护，让每个工具保持一致。</p>
        </div>
        <button
          type="button"
          className="button secondary"
          onClick={() => setIsAddLibraryModalOpen(true)}
        >
          <Icon name="plus" size={14} />
          <span>添加技能库</span>
        </button>
      </div>

      {/* 无技能库时的空状态 */}
      {!activeLibrary ? (
        <div className="empty-state">
          <div className="empty-illustration">
            <Icon name="folder" size={30} />
          </div>
          <h2>从一个技能库开始</h2>
          <p>选择你的本地技能目录，然后将它同步到常用的 Agent 工具。</p>
          <button
            type="button"
            className="button primary mt-6"
            onClick={() => setIsAddLibraryModalOpen(true)}
          >
            <Icon name="plus" size={14} />
            <span>添加技能库</span>
          </button>
        </div>
      ) : (
        <>
          {/* 源库信息条 */}
          <div className="source-strip">
            <div className="source-glyph">
              <Icon name="folder" size={21} />
            </div>
            <div className="source-info">
              <strong>
                <span>{activeLibrary.summary.displayName}</span>
                {activeLibrary.summary.pinned && (
                  <span className="subtle-badge">已固定</span>
                )}
              </strong>
              <code title={activeLibrary.summary.sourceRoot || activeLibrary.summary.canonicalPath}>
                {activeLibrary.summary.sourceRoot || activeLibrary.summary.canonicalPath}
              </code>
            </div>
            <div className="strip-meta select-none">
              <span className="inline-stat">
                <b>{counts.all}</b> 个技能
              </span>
              <span className="tiny-separator" />
              <span className="inline-stat">
                <b>{physicalTargetCount}</b> 个目标
              </span>
            </div>
            <button
              type="button"
              className="icon-button"
              onClick={handleOpenSourceFolder}
              title="打开技能库本地目录"
              aria-label="打开技能库本地目录"
            >
              <Icon name="external" size={14} />
            </button>
          </div>

          {/* 库内无技能时的空状态 */}
          {activeLibrary.skills.length === 0 ? (
            <div className="empty-state">
              <div className="empty-illustration">
                <Icon name="folder" size={30} />
              </div>
              <h2>这里还没有找到技能</h2>
              <p>源目录的直接子文件夹需要包含 SKILL.md。添加或修正文件后，重新扫描即可。</p>
              <div className="empty-action-row">
                <button
                  type="button"
                  className="button secondary"
                  onClick={reloadCurrentLibrary}
                >
                  <Icon name="refresh" size={14} />
                  <span>重新扫描</span>
                </button>
                <button
                  type="button"
                  className="button ghost"
                  onClick={() => setIsLibrariesModalOpen(true)}
                >
                  更换技能库
                </button>
              </div>
            </div>
          ) : (
            <>
              {/* 筛选与搜索工具栏 */}
              <div className="section-toolbar">
                <div className="tabs" role="tablist" aria-label="技能状态筛选">
                  <button
                    type="button"
                    role="tab"
                    aria-selected={statusFilter === 'all'}
                    className={`tab ${statusFilter === 'all' ? 'active' : ''}`}
                    onClick={() => setStatusFilter('all')}
                  >
                    <span>全部</span>
                    <span className="count">{counts.all}</span>
                  </button>

                  <button
                    type="button"
                    role="tab"
                    aria-selected={statusFilter === 'pending'}
                    className={`tab ${statusFilter === 'pending' ? 'active' : ''}`}
                    onClick={() => setStatusFilter('pending')}
                  >
                    <span>待同步</span>
                    <span className="count">{counts.pending}</span>
                  </button>

                  <button
                    type="button"
                    role="tab"
                    aria-selected={statusFilter === 'conflict'}
                    className={`tab ${statusFilter === 'conflict' ? 'active' : ''}`}
                    onClick={() => setStatusFilter('conflict')}
                  >
                    <span>需处理</span>
                    <span className="count">{counts.conflict}</span>
                  </button>

                  <button
                    type="button"
                    role="tab"
                    aria-selected={statusFilter === 'invalid'}
                    className={`tab ${statusFilter === 'invalid' ? 'active' : ''}`}
                    onClick={() => setStatusFilter('invalid')}
                  >
                    <span>校验异常</span>
                    <span className="count">{counts.invalid}</span>
                  </button>
                </div>

                <label className="search-box">
                  <Icon name="search" size={14} />
                  <input
                    id="skill-search"
                    type="text"
                    placeholder="搜索技能名称或描述"
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    aria-label="搜索技能名称或描述"
                  />
                  <kbd className="select-none">Ctrl F</kbd>
                </label>
              </div>

              {/* 矩阵表格 */}
              <SyncMatrixTable />
            </>
          )}
        </>
      )}
    </div>
  );
};
