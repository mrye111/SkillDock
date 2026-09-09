import React, { useState } from 'react';
import { useApp } from '../../context/AppContext';
import { Icon } from '../common/Icon';

const TAB_TITLES: Record<string, string> = {
  workbench: '技能库',
  targets: '同步目标',
  history: '同步历史',
  settings: '设置',
};

export const TopBar: React.FC = () => {
  const { currentTab, reloadCurrentLibrary, reloadTargets, isLoadingLibrary, isTauriApp, showToast } =
    useApp();
  const [isRefreshing, setIsRefreshing] = useState(false);

  const handleRefresh = async () => {
    if (isRefreshing || isLoadingLibrary) return;
    setIsRefreshing(true);
    try {
      if (currentTab === 'workbench') {
        await reloadCurrentLibrary();
        showToast('已重新扫描磁盘并更新技能状态', 'success');
      } else if (currentTab === 'targets') {
        await reloadTargets();
        showToast('已刷新同步目标列表', 'success');
      } else {
        await new Promise((r) => setTimeout(r, 280));
        showToast('已刷新当前页面', 'success');
      }
    } finally {
      setIsRefreshing(false);
    }
  };

  const currentTitle = TAB_TITLES[currentTab] || '工作空间';

  return (
    <header className="topbar">
      {/* 面包屑导航 */}
      <div className="breadcrumb select-none">
        <span>工作空间</span>
        <span className="crumb-separator">/</span>
        <strong>{currentTitle}</strong>
      </div>

      {/* 右侧系统信息与刷新操作 */}
      <div className="topbar-right">
        <span className="quiet-label select-none">
          {isTauriApp ? '桌面容器已就绪' : '本地文件同步'}
        </span>
        <span className="topbar-divider" />
        <button
          type="button"
          className={`icon-button ${isRefreshing || isLoadingLibrary ? 'spinning' : ''}`}
          onClick={handleRefresh}
          aria-label="刷新当前页面（F5）"
          title="刷新（F5）"
        >
          <Icon name="refresh" size={16} />
        </button>
      </div>
    </header>
  );
};
