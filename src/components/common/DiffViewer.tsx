import React, { useState } from 'react';
import type { FileChange } from '../../lib/backend-contract';
import {
  FilePlus,
  FileEdit,
  FileMinus,
  ChevronDown,
  ChevronRight,
  AlertCircle,
} from 'lucide-react';

interface DiffViewerProps {
  changes: FileChange[];
}

export const DiffViewer: React.FC<DiffViewerProps> = ({ changes }) => {
  const [expandedFiles, setExpandedFiles] = useState<Record<string, boolean>>({
    [changes[0]?.relPath || '']: true,
  });

  const toggleExpand = (relPath: string) => {
    setExpandedFiles((prev) => ({
      ...prev,
      [relPath]: !prev[relPath],
    }));
  };

  const formatBytes = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  };

  if (changes.length === 0) {
    return (
      <div className="p-4 text-center text-xs text-content-muted rounded-lg bg-surface-bg border border-border-subtle">
        无文件内容变更
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {changes.map((file) => {
        const isExpanded = !!expandedFiles[file.relPath];
        const isLargeOrBinary = !file.diffPreviewable || file.bytes > 1024 * 1024;

        let Icon = FileEdit;
        let badgeText = '修改';
        let badgeClass = 'bg-primary-subtle text-primary border-primary-light';

        if (file.kind === 'add') {
          Icon = FilePlus;
          badgeText = '新增';
          badgeClass = 'bg-status-success-bg text-status-success border-status-success-border';
        } else if (file.kind === 'delete') {
          Icon = FileMinus;
          badgeText = '删除';
          badgeClass = 'bg-status-danger-bg text-status-danger border-status-danger-border';
        }

        return (
          <div
            key={file.relPath}
            className="rounded-xl border border-border-subtle bg-surface-panel overflow-hidden"
          >
            {/* 文件头部 */}
            <div
              onClick={() => toggleExpand(file.relPath)}
              className="flex items-center justify-between px-3.5 py-2.5 bg-surface-bg/60 hover:bg-surface-hover cursor-pointer select-none transition-colors"
            >
              <div className="flex items-center gap-2 min-w-0 pr-2">
                <button type="button" className="text-content-muted">
                  {isExpanded ? (
                    <ChevronDown className="w-4 h-4" />
                  ) : (
                    <ChevronRight className="w-4 h-4" />
                  )}
                </button>
                <Icon className="w-4 h-4 text-content-secondary flex-shrink-0" />
                <span className="font-mono text-xs font-semibold text-content-primary truncate">
                  {file.relPath}
                </span>
                <span className={`text-[10px] px-1.5 py-0.5 rounded border font-medium ${badgeClass}`}>
                  {badgeText}
                </span>
              </div>
              <div className="text-[11px] font-mono text-content-muted flex-shrink-0">
                {formatBytes(file.bytes)}
              </div>
            </div>

            {/* 展开的差异内容 */}
            {isExpanded && (
              <div className="p-3 border-t border-border-subtle bg-surface-panel">
                {isLargeOrBinary ? (
                  <div className="flex items-center gap-2 p-3 text-xs text-content-secondary bg-surface-bg rounded-lg border border-border-subtle">
                    <AlertCircle className="w-4 h-4 text-status-warning flex-shrink-0" />
                    <span>文件超过 1MB 或为二进制资源，已降级为概要展示。</span>
                  </div>
                ) : (
                  <div className="font-mono text-xs overflow-x-auto bg-slate-900 text-slate-100 p-3 rounded-lg border border-slate-800 leading-relaxed">
                    {/* 模拟逐行文本对比 */}
                    {file.kind === 'add' ? (
                      <div className="space-y-0.5">
                        <div className="text-emerald-400 bg-emerald-950/40 px-1 rounded">
                          + --- 新增文件 ({file.relPath}) ---
                        </div>
                        <div className="text-emerald-400 bg-emerald-950/40 px-1 rounded">
                          + # {file.relPath.split('/').pop()}
                        </div>
                        <div className="text-emerald-400 bg-emerald-950/40 px-1 rounded">
                          + 新增内容已准备就绪，同步后写入目标。
                        </div>
                      </div>
                    ) : file.kind === 'delete' ? (
                      <div className="space-y-0.5">
                        <div className="text-rose-400 bg-rose-950/40 px-1 rounded">
                          - --- 移除旧文件 ({file.relPath}) ---
                        </div>
                        <div className="text-rose-400 bg-rose-950/40 px-1 rounded">
                          - 该文件在源库中已被删除，更新时将从目标端同步移除。
                        </div>
                      </div>
                    ) : (
                      <div className="space-y-0.5">
                        <div className="text-slate-400">@@ -1,4 +1,5 @@</div>
                        <div className="text-slate-300">  name: {file.relPath.split('/')[0]}</div>
                        <div className="text-rose-400 bg-rose-950/40 px-1 rounded">
                          - description: 旧版本描述内容
                        </div>
                        <div className="text-emerald-400 bg-emerald-950/40 px-1 rounded">
                          + description: 已更新的 Skill 提示词与规范
                        </div>
                        <div className="text-emerald-400 bg-emerald-950/40 px-1 rounded">
                          + version: 1.1.0
                        </div>
                        <div className="text-slate-300">  # 保持原始资源配置与引用</div>
                      </div>
                    )}
                  </div>
                )}
                <div className="mt-2 flex items-center justify-between text-[11px] text-content-muted font-mono">
                  <span>源摘要: {file.sourceDigest?.substring(0, 16) || '无'}</span>
                  <span>目标摘要: {file.targetDigest?.substring(0, 16) || '无'}</span>
                </div>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
};
