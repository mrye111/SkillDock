import React, { useEffect } from 'react';
import { Icon } from './Icon';

export interface ToastProps {
  message: string;
  type?: 'success' | 'warning' | 'error' | 'info';
  onClose: () => void;
  duration?: number;
}

export const Toast: React.FC<ToastProps> = ({
  message,
  type = 'success',
  onClose,
  duration = 3400,
}) => {
  useEffect(() => {
    const timer = setTimeout(() => {
      onClose();
    }, duration);
    return () => clearTimeout(timer);
  }, [onClose, duration]);

  const iconName =
    type === 'success' ? 'circleCheck' : type === 'error' ? 'warning' : 'info';

  return (
    <div className={`toast ${type}`} role="status" aria-live="polite">
      <Icon name={iconName} size={15} />
      <span>{message}</span>
    </div>
  );
};
