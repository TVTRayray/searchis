import React from 'react';
import { Inline } from './Inline';

export interface StatusDotProps {
  status?: 'active' | 'success' | 'warning' | 'danger' | 'neutral';
  pulse?: boolean;
  size?: 'sm' | 'md' | 'lg';
  className?: string;
  label?: React.ReactNode;
}

const colorMap: Record<string, string> = {
  active: 'var(--color-accent)',
  success: 'var(--color-success)',
  warning: 'var(--color-warning)',
  danger: 'var(--color-danger)',
  neutral: 'var(--color-fg-disabled)',
};

const sizeMap: Record<string, number> = {
  sm: 6,
  md: 8,
  lg: 10,
};

export const StatusDot: React.FC<StatusDotProps> = ({
  status = 'active',
  pulse = false,
  size = 'md',
  className = '',
  label,
}) => {
  const dotSize = sizeMap[size];
  const color = colorMap[status];

  const dot = (
    <span
      className={`relative inline-block rounded-full ${pulse ? 'animate-pulse' : ''} ${className}`}
      style={{
        width: `${dotSize}px`,
        height: `${dotSize}px`,
        backgroundColor: color,
        boxShadow: pulse ? `0 0 8px ${color}` : undefined,
      }}
    />
  );

  if (label) {
    return (
      <Inline gap="xs" align="center">
        {dot}
        <span className="text-xs text-theme-muted font-medium">{label}</span>
      </Inline>
    );
  }

  return dot;
};

export interface StatusTokenProps {
  status: 'active' | 'success' | 'warning' | 'danger' | 'neutral';
  label: React.ReactNode;
  className?: string;
}

export const StatusToken: React.FC<StatusTokenProps> = ({ status, label, className = '' }) => {
  const statusClasses: Record<string, string> = {
    active: 'status-accent',
    success: 'status-success',
    warning: 'status-warning',
    danger: 'status-danger',
    neutral: 'badge-neutral',
  };

  return (
    <span className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md border text-xs font-medium ${statusClasses[status] || 'badge-neutral'} ${className}`}>
      <StatusDot status={status} size="sm" />
      <span>{label}</span>
    </span>
  );
};
