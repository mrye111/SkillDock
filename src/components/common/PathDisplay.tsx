import React, { useState } from 'react';
import { Folder, Copy, Check, ExternalLink } from 'lucide-react';
import { invokeCommand } from '../../services/api';

interface PathDisplayProps {
  path: string;
  kind?: 'library' | 'target' | 'task_item' | 'snapshot';
  id?: string;
  className?: string;
  showOpenButton?: boolean;
}

export const PathDisplay: React.FC<PathDisplayProps> = ({
  path,
  kind,
  id,
  className = '',
  showOpenButton = true,
}) => {
  const [copied, setCopied] = useState(false);

  const handleCopy = async (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await navigator.clipboard.writeText(path);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('复制路径失败:', err);
    }
  };

  const handleOpenFolder = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (kind && id) {
      try {
        await invokeCommand('open_registered_path', { kind, id });
      } catch (err: any) {
        alert(`打开文件夹失败: ${err?.message || err}`);
      }
    }
  };

  return (
    <div
      className={`inline-flex items-center gap-1.5 text-xs text-content-secondary font-mono max-w-full group ${className}`}
      title={path}
    >
      <Folder className="w-3.5 h-3.5 flex-shrink-0 text-content-muted group-hover:text-primary" />
      <span className="truncate flex-1 select-all">{path}</span>

      <button
        type="button"
        onClick={handleCopy}
        title="复制完整路径"
        className="p-1 hover:bg-surface-hover rounded text-content-muted hover:text-content-primary transition-colors flex-shrink-0"
      >
        {copied ? (
          <Check className="w-3.5 h-3.5 text-status-success" />
        ) : (
          <Copy className="w-3.5 h-3.5" />
        )}
      </button>

      {showOpenButton && kind && id && (
        <button
          type="button"
          onClick={handleOpenFolder}
          title="在资源管理器中打开"
          className="p-1 hover:bg-surface-hover rounded text-content-muted hover:text-primary transition-colors flex-shrink-0"
        >
          <ExternalLink className="w-3.5 h-3.5" />
        </button>
      )}
    </div>
  );
};
