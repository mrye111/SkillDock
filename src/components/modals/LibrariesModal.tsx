import React from 'react';
import { useApp } from '../../context/AppContext';
import { Modal } from '../common/Modal';
import { Icon } from '../common/Icon';

export const LibrariesModal: React.FC = () => {
  const {
    isLibrariesModalOpen,
    setIsLibrariesModalOpen,
    setIsAddLibraryModalOpen,
    libraries,
    activeLibraryId,
    switchLibrary,
    setLibraryPinned,
    removeLibraryRecord,
  } = useApp();

  if (!isLibrariesModalOpen) return null;

  return (
    <Modal
      isOpen={isLibrariesModalOpen}
      onClose={() => setIsLibrariesModalOpen(false)}
      title="技能库"
      subtitle="切换和管理已登记的技能目录。"
      wide
      footer={
        <button
          type="button"
          className="button primary"
          onClick={() => {
            setIsLibrariesModalOpen(false);
            setIsAddLibraryModalOpen(true);
          }}
        >
          <Icon name="plus" size={14} />
          <span>添加技能库</span>
        </button>
      }
    >
      <div className="space-y-2">
        {libraries.length === 0 ? (
          <p className="info-foot text-center py-6">尚未登记技能库。</p>
        ) : (
          libraries.map((lib) => {
            const isSelected = lib.libraryId === activeLibraryId;
            return (
              <div
                key={lib.libraryId}
                className={`library-option ${isSelected ? 'selected' : ''}`}
              >
                <button
                  type="button"
                  onClick={async () => {
                    await switchLibrary(lib.libraryId);
                    setIsLibrariesModalOpen(false);
                  }}
                >
                  <span className="source-glyph">
                    <Icon name="folder" size={21} />
                  </span>
                  <span>
                    <strong>
                      {lib.displayName}
                      {lib.pinned && (
                        <span className="subtle-badge ml-2 text-clear-blue font-normal">
                          已固定
                        </span>
                      )}
                    </strong>
                    <code title={lib.canonicalPath}>{lib.canonicalPath}</code>
                    <small>
                      {lib.skillCount} 个技能
                      {!lib.pathValid ? ' · 路径已失效' : ''}
                    </small>
                  </span>
                </button>

                {/* 固定操作 */}
                <button
                  type="button"
                  className={`icon-button ${lib.pinned ? 'pinned' : ''}`}
                  onClick={() => setLibraryPinned(lib.libraryId, !lib.pinned)}
                  aria-label={lib.pinned ? '取消固定' : '固定技能库'}
                  title={lib.pinned ? '取消固定' : '固定技能库'}
                >
                  <Icon name="pin" size={14} />
                </button>

                {/* 移除操作 */}
                <button
                  type="button"
                  className="icon-button hover:text-clear-red"
                  onClick={() => {
                    if (
                      window.confirm(
                        `确定要从 SkillDock 移除技能库「${lib.displayName}」的登记吗？（不会删除本地文件夹）`
                      )
                    ) {
                      removeLibraryRecord(lib.libraryId);
                    }
                  }}
                  aria-label={`移除技能库 ${lib.displayName}`}
                  title="从列表移除"
                >
                  <Icon name="trash" size={14} />
                </button>
              </div>
            );
          })
        )}
      </div>
    </Modal>
  );
};
