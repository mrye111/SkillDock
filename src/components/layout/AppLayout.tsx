import React from 'react';
import { useApp } from '../../context/AppContext';
import { Sidebar } from './Sidebar';
import { TopBar } from './TopBar';
import { WorkbenchView } from '../../views/workbench/WorkbenchView';
import { TargetsView } from '../../views/targets/TargetsView';
import { HistoryView } from '../../views/history/HistoryView';
import { SettingsView } from '../../views/settings/SettingsView';
import { SelectionDock } from '../workbench/SelectionDock';
import { SkillDetailDrawer } from '../drawers/SkillDetailDrawer';
import { SyncPlanDrawer } from '../drawers/SyncPlanDrawer';
import { TargetDetailDrawer } from '../drawers/TargetDetailDrawer';
import { HistoryDetailDrawer } from '../drawers/HistoryDetailDrawer';
import { LibrariesModal } from '../modals/LibrariesModal';
import { AddLibraryModal } from '../modals/AddLibraryModal';
import { AddTargetModal } from '../modals/AddTargetModal';
import { TaskProgressModal } from '../modals/TaskProgressModal';
import { RecoveryModal } from '../modals/RecoveryModal';
import { Toast } from '../common/Toast';

export const AppLayout: React.FC = () => {
  const { currentTab, toast, clearToast } = useApp();

  return (
    <>
      {/* 主应用外壳 —— 铺满整个窗口 */}
      <div className="app-shell">
        {/* 左侧导航栏 */}
        <Sidebar />

        {/* 右侧主工作空间 */}
        <div className="workspace">
          {/* 顶部工具栏 */}
          <TopBar />

          {/* 核心主滚动区域 */}
          <main id="main" tabIndex={-1}>
            {currentTab === 'workbench' && <WorkbenchView />}
            {currentTab === 'targets' && <TargetsView />}
            {currentTab === 'history' && <HistoryView />}
            {currentTab === 'settings' && <SettingsView />}
          </main>

          {/* 工作台浮动操作底栏 */}
          {currentTab === 'workbench' && <SelectionDock />}
        </div>
      </div>

      {/* 抽屉组件 */}
      <SkillDetailDrawer />
      <SyncPlanDrawer />
      <TargetDetailDrawer />
      <HistoryDetailDrawer />

      {/* 模态弹窗组件 */}
      <LibrariesModal />
      <AddLibraryModal />
      <AddTargetModal />
      <TaskProgressModal />
      <RecoveryModal />

      {/* 全局 Toast 轻提示 */}
      {toast && (
        <Toast
          message={toast.message}
          type={toast.type}
          onClose={clearToast}
        />
      )}
    </>
  );
};
