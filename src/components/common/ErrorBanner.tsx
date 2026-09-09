import React, { useState } from 'react';
import type { AppError } from '../../lib/backend-contract';
import { AlertTriangle, ChevronDown, ChevronRight, RefreshCw } from 'lucide-react';

interface ErrorBannerProps {
  error: AppError;
  onRetry?: () => void;
  className?: string;
}

export const ErrorBanner: React.FC<ErrorBannerProps> = ({
  error,
  onRetry,
  className = '',
}) => {
  const [expanded, setExpanded] = useState(false);

  return (
    <div
      className={`p-4 rounded-xl bg-status-danger-bg border border-status-danger-border text-content-primary ${className}`}
    >
      <div className="flex items-start gap-3">
        <AlertTriangle className="w-5 h-5 text-status-danger flex-shrink-0 mt-0.5" />
        <div className="flex-1 min-w-0">
          <div className="flex items-center justify-between gap-2">
            <h4 className="text-sm font-semibold text-status-danger">
              操作失败（错误码：{error.code}）
            </h4>
            {error.retryable && onRetry && (
              <button
                type="button"
                onClick={onRetry}
                className="inline-flex items-center gap-1 px-2.5 py-1 text-xs font-medium text-white bg-status-danger hover:bg-red-700 rounded-md transition-colors"
              >
                <RefreshCw className="w-3 h-3" />
                重试
              </button>
            )}
          </div>
          <p className="text-sm mt-1 text-content-primary leading-relaxed">{error.message}</p>

          <div className="mt-2">
            <button
              type="button"
              onClick={() => setExpanded(!expanded)}
              className="inline-flex items-center gap-1 text-xs text-content-secondary hover:text-content-primary font-medium"
            >
              {expanded ? <ChevronDown className="w-3.5 h-3.5" /> : <ChevronRight className="w-3.5 h-3.5" />}
              诊断详情与上下文 (ID: {error.diagnosticId})
            </button>

            {expanded && (
              <div className="mt-2 p-2.5 rounded-lg bg-surface-panel border border-border-subtle font-mono text-xs overflow-x-auto select-text">
                <div className="text-content-secondary mb-1">
                  诊断标识：<span className="text-content-primary">{error.diagnosticId}</span>
                </div>
                {error.context && (
                  <pre className="text-content-secondary whitespace-pre-wrap break-all">
                    {JSON.stringify(error.context, null, 2)}
                  </pre>
                )}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
