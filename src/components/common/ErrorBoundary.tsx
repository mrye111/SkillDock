import { Component, ErrorInfo, ReactNode } from 'react';
import { Icon } from './Icon';

interface Props {
  children: ReactNode;
  fallbackTitle?: string;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
    error: null,
  };

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('[ErrorBoundary caught error]:', error, errorInfo);
  }

  private handleReset = () => {
    this.setState({ hasError: false, error: null });
    window.location.reload();
  };

  public render() {
    if (this.state.hasError) {
      return (
        <div className="min-h-screen flex items-center justify-center p-6 bg-slate-100/90 select-none">
          <div className="max-w-md w-full p-6 rounded-2xl bg-white shadow-xl border border-rose-200 text-center">
            <div className="w-12 h-12 rounded-full bg-rose-50 text-rose-600 flex items-center justify-center mx-auto mb-4 border border-rose-100">
              <Icon name="warning" size={24} />
            </div>
            <h2 className="text-base font-semibold text-slate-800 mb-2">
              {this.props.fallbackTitle || '界面渲染发生异常'}
            </h2>
            <p className="text-xs text-slate-500 mb-4 leading-relaxed">
              底层已安全保护，未发生数据损坏。您可以尝试重置界面或点击刷新。
            </p>
            {this.state.error && (
              <pre className="p-3 mb-4 rounded-lg bg-slate-50 text-[11px] text-rose-600 text-left font-mono overflow-x-auto max-h-32 border border-slate-100">
                {this.state.error.message || String(this.state.error)}
              </pre>
            )}
            <div className="flex gap-2 justify-center">
              <button
                type="button"
                className="button primary small"
                onClick={this.handleReset}
              >
                刷新并重试
              </button>
            </div>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
