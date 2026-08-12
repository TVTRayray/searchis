import React from 'react';

export interface BadgeProps {
  variant?: 'neutral' | 'accent' | 'success' | 'warning' | 'danger';
  size?: 'sm' | 'md';
  mono?: boolean;
  className?: string;
  children: React.ReactNode;
}

const variantClassMap: Record<string, string> = {
  neutral: 'badge-neutral',
  accent: 'badge-accent',
  success: 'badge-success',
  warning: 'badge-warning',
  danger: 'badge-danger',
};

export const Badge: React.FC<BadgeProps> = ({
  variant = 'neutral',
  size = 'md',
  mono = false,
  className = '',
  children,
}) => {
  const sizeClasses = size === 'sm' ? 'px-1.5 py-0.5 text-[10px]' : 'px-2 py-0.5 text-xs';
  const monoClass = mono ? 'font-mono' : '';

  return (
    <span className={`inline-flex items-center font-medium rounded-md tracking-tight ${variantClassMap[variant]} ${sizeClasses} ${monoClass} ${className}`.trim()}>
      {children}
    </span>
  );
};

export const Token: React.FC<Omit<BadgeProps, 'variant'>> = (props) => (
  <Badge variant="accent" mono {...props} />
);
