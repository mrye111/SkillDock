import React from 'react';
import { useApp } from '../../context/AppContext';
import { Icon, BrandLogo } from '../common/Icon';

export const Sidebar: React.FC = () => {
  const {
    currentTab,
    setCurrentTab,
    activeLibrary,
    setIsLibrariesModalOpen,
  } = useApp();

  const skillCount = activeLibrary?.skills?.length ?? 0;
  const libraryDisplayName = activeLibrary?.summary.displayName || '选择技能库';

  return (
    <aside className="sidebar">
      {/* 品牌 Logo 浮动微质感磁贴 */}
      <div
        className="brand cursor-pointer select-none"
        onClick={() => setCurrentTab('workbench')}
        aria-label="SkillDock 技能库"
      >
        <div className="brand-tile">
          <BrandLogo size={22} />
        </div>
        <div className="brand-copy">
          <span className="brand-name">SkillDock</span>
          <span className="brand-caption">Desktop Sync</span>
        </div>
      </div>

      {/* 工作空间切换器 */}
      <button
        type="button"
        className="workspace-switch"
        onClick={() => setIsLibrariesModalOpen(true)}
        title="点击管理与切换技能库"
      >
        <span className="workspace-icon">
          <Icon name="folder" size={17} />
        </span>
        <span>
          <strong title={libraryDisplayName}>{libraryDisplayName}</strong>
          <small>本地技能库</small>
        </span>
        <span className="muted">
          <Icon name="chevrons" size={13} />
        </span>
      </button>

      {/* 导航标签 */}
      <div className="nav-label">工作空间</div>

      {/* 主要导航项 */}
      <nav className="main-nav" aria-label="主要导航">
        <button
          type="button"
          className={currentTab === 'workbench' ? 'active' : ''}
          onClick={() => setCurrentTab('workbench')}
        >
          {currentTab === 'workbench' && <span className="ribbon-indicator" />}
          <Icon name="layers" size={17} />
          <span>技能库</span>
          <small>{skillCount}</small>
        </button>

        <button
          type="button"
          className={currentTab === 'targets' ? 'active' : ''}
          onClick={() => setCurrentTab('targets')}
        >
          {currentTab === 'targets' && <span className="ribbon-indicator" />}
          <Icon name="target" size={17} />
          <span>同步目标</span>
        </button>

        <button
          type="button"
          className={currentTab === 'history' ? 'active' : ''}
          onClick={() => setCurrentTab('history')}
        >
          {currentTab === 'history' && <span className="ribbon-indicator" />}
          <Icon name="history" size={17} />
          <span>同步历史</span>
        </button>
      </nav>

      {/* 底部导航与状态 */}
      <div className="sidebar-bottom">
        <nav className="main-nav">
          <button
            type="button"
            className={currentTab === 'settings' ? 'active' : ''}
            onClick={() => setCurrentTab('settings')}
          >
            {currentTab === 'settings' && <span className="ribbon-indicator" />}
            <Icon name="settings" size={17} />
            <span>设置</span>
          </button>
        </nav>

        <div className="sidebar-note select-none">
          <span className="status-dot green" />
          <span>一处维护，随处可用</span>
        </div>
      </div>
    </aside>
  );
};
