import React, { useState } from 'react';
import { useApp } from '../../context/AppContext';
import { Modal } from '../common/Modal';
import { Icon } from '../common/Icon';
import { pickDirectory } from '../../services/api';

export const AddLibraryModal: React.FC = () => {
  const {
    isAddLibraryModalOpen,
    setIsAddLibraryModalOpen,
    registerNewLibrary,
  } = useApp();

  const [path, setPath] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [mode, setMode] = useState<'auto' | 'collection' | 'single'>('auto');
  const [isSubmitting, setIsSubmitting] = useState(false);

  if (!isAddLibraryModalOpen) return null;

  const handlePickDirectory = async () => {
    const picked = await pickDirectory(path || undefined);
    if (picked) {
      setPath(picked);
      if (!displayName) {
        const folderName = picked.split(/[\\/]/).filter(Boolean).pop() || '';
        setDisplayName(folderName);
      }
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!path.trim()) return;

    setIsSubmitting(true);
    try {
      await registerNewLibrary(path.trim(), displayName.trim() || undefined, mode);
      setIsAddLibraryModalOpen(false);
      setPath('');
      setDisplayName('');
      setMode('auto');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Modal
      isOpen={isAddLibraryModalOpen}
      onClose={() => setIsAddLibraryModalOpen(false)}
      title="添加技能库"
      subtitle="登记源目录后，先扫描再读取技能列表。"
      footer={
        <>
          <button
            type="button"
            className="button ghost"
            onClick={() => setIsAddLibraryModalOpen(false)}
            disabled={isSubmitting}
          >
            取消
          </button>
          <button
            type="button"
            className="button primary"
            onClick={handleSubmit}
            disabled={!path.trim() || isSubmitting}
          >
            <span>{isSubmitting ? '正在登记与扫描...' : '登记并扫描'}</span>
            <Icon name="arrow" size={14} />
          </button>
        </>
      }
    >
      <form onSubmit={handleSubmit}>
        <div className="form-row">
          <label htmlFor="library-path">源目录绝对路径</label>
          <div className="flex gap-2">
            <input
              id="library-path"
              type="text"
              placeholder="例如 D:\Workspace\skills"
              value={path}
              onChange={(e) => setPath(e.target.value)}
              required
            />
            <button
              type="button"
              className="button secondary flex-none"
              onClick={handlePickDirectory}
              title="浏览本地目录"
            >
              <Icon name="folder" size={15} />
              <span>浏览</span>
            </button>
          </div>
          <p>选择包含一个或多个技能的本地文件夹，登记后系统将自动索引 SKILL.md。</p>
        </div>

        <div className="form-row">
          <label htmlFor="library-name">技能库显示名称（可选）</label>
          <input
            id="library-name"
            type="text"
            placeholder="例如 Personal Skills"
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
          />
        </div>

        <div className="form-row">
          <label htmlFor="library-mode">目录类型</label>
          <select
            id="library-mode"
            value={mode}
            onChange={(e) => setMode(e.target.value as any)}
          >
            <option value="auto">自动识别 (Auto)</option>
            <option value="collection">技能集合目录 (Collection)</option>
            <option value="single">单个技能根目录 (Single)</option>
          </select>
          <p>如果不确定，请保持「自动识别」即可。</p>
        </div>
      </form>
    </Modal>
  );
};
