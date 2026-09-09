import React, { useState } from 'react';
import { useApp } from '../../context/AppContext';
import { Modal } from '../common/Modal';
import { Icon } from '../common/Icon';
import { pickDirectory } from '../../services/api';

const TOOL_NAMES: Record<string, string> = {
  codex: 'Codex',
  claude_code: 'Claude Code',
  cursor: 'Cursor',
  copilot_vscode: 'Copilot',
  custom: '自定义目录',
};

export const AddTargetModal: React.FC = () => {
  const { isAddTargetModalOpen, setIsAddTargetModalOpen, addTarget } = useApp();

  const [adapterId, setAdapterId] = useState('cursor');
  const [scope, setScope] = useState<'user' | 'project' | 'custom'>('user');
  const [displayName, setDisplayName] = useState('Cursor');
  const [path, setPath] = useState('');
  const [projectRoot, setProjectRoot] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  if (!isAddTargetModalOpen) return null;

  const handleAdapterChange = (newAdapter: string) => {
    setAdapterId(newAdapter);
    setDisplayName(TOOL_NAMES[newAdapter] || '自定义目标');
    if (newAdapter === 'custom') {
      setScope('custom');
    }
  };

  const handlePickDirectory = async (forProjectRoot = false) => {
    const picked = await pickDirectory();
    if (picked) {
      if (forProjectRoot) {
        setProjectRoot(picked);
      } else {
        setPath(picked);
      }
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    try {
      const finalPath = scope === 'project' ? projectRoot : path;
      const success = await addTarget(
        adapterId,
        scope,
        finalPath.trim() || undefined,
        displayName.trim() || undefined
      );
      if (success) {
        setIsAddTargetModalOpen(false);
        setPath('');
        setProjectRoot('');
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Modal
      isOpen={isAddTargetModalOpen}
      onClose={() => setIsAddTargetModalOpen(false)}
      title="添加同步目标"
      subtitle="选择工具与作用域。保存目录配置后，再为技能选择该目标。"
      footer={
        <>
          <button
            type="button"
            className="button ghost"
            onClick={() => setIsAddTargetModalOpen(false)}
            disabled={isSubmitting}
          >
            取消
          </button>
          <button
            type="button"
            className="button primary"
            onClick={handleSubmit}
            disabled={isSubmitting}
          >
            <span>{isSubmitting ? '正在保存...' : '保存目标'}</span>
            <Icon name="arrow" size={14} />
          </button>
        </>
      }
    >
      <form onSubmit={handleSubmit}>
        <div className="form-row">
          <label htmlFor="target-adapter">Agent 工具</label>
          <select
            id="target-adapter"
            value={adapterId}
            onChange={(e) => handleAdapterChange(e.target.value)}
          >
            {Object.entries(TOOL_NAMES).map(([val, name]) => (
              <option key={val} value={val}>
                {name}
              </option>
            ))}
          </select>
        </div>

        <div className="form-row inline">
          <div>
            <label htmlFor="target-scope">作用域</label>
            <select
              id="target-scope"
              value={scope}
              onChange={(e) => setScope(e.target.value as any)}
              disabled={adapterId === 'custom'}
            >
              <option value="user">用户级 (User)</option>
              <option value="project">项目级 (Project)</option>
              {adapterId === 'custom' && <option value="custom">自定义目录</option>}
            </select>
          </div>
          <div>
            <label htmlFor="target-name">显示名称</label>
            <input
              id="target-name"
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              required
            />
          </div>
        </div>

        {scope === 'project' && (
          <div className="form-row">
            <label htmlFor="project-root">目标项目根目录</label>
            <div className="flex gap-2">
              <input
                id="project-root"
                type="text"
                placeholder="例如 D:\Workspace\my-app"
                value={projectRoot}
                onChange={(e) => setProjectRoot(e.target.value)}
                required
              />
              <button
                type="button"
                className="button secondary flex-none"
                onClick={() => handlePickDirectory(true)}
              >
                <Icon name="folder" size={15} />
                <span>浏览</span>
              </button>
            </div>
            <p>系统将自动根据该项目根目录推导对应的 Agent 技能子目录。</p>
          </div>
        )}

        {(scope === 'custom' || scope === 'user') && (
          <div className="form-row">
            <label htmlFor="target-path">
              {scope === 'custom' ? '自定义技能目录绝对路径' : '自定义目标路径（留空则使用默认配置）'}
            </label>
            <div className="flex gap-2">
              <input
                id="target-path"
                type="text"
                placeholder={
                  scope === 'custom'
                    ? '例如 D:\\CustomAgent\\skills'
                    : '留空将使用系统默认路径'
                }
                value={path}
                onChange={(e) => setPath(e.target.value)}
                required={scope === 'custom'}
              />
              <button
                type="button"
                className="button secondary flex-none"
                onClick={() => handlePickDirectory(false)}
              >
                <Icon name="folder" size={15} />
                <span>浏览</span>
              </button>
            </div>
          </div>
        )}
      </form>
    </Modal>
  );
};
