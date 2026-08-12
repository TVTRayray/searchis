import React from 'react';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'danger' | 'success' | 'ghost';
  size?: 'sm' | 'md' | 'lg';
  icon?: React.ReactNode;
  loading?: boolean;
  className?: string;
  children?: React.ReactNode;
}

export const Button: React.FC<ButtonProps> = ({
  variant = 'secondary',
  size = 'md',
  icon,
  loading = false,
  disabled,
  className = '',
  children,
  ...rest
}) => {
  const variantClassMap: Record<string, string> = {
    primary: 'btn-primary',
    secondary: 'btn-secondary',
    danger: 'btn-danger',
    success: 'btn-success',
    ghost: 'interactive-muted hover:bg-[color:var(--color-surface-hover)]',
  };

  const sizeClassMap: Record<string, string> = {
    sm: 'px-2.5 py-1 text-xs rounded-md',
    md: 'px-3.5 py-1.5 text-xs font-semibold rounded-lg',
    lg: 'px-4 py-2 text-sm font-semibold rounded-xl',
  };

  return (
    <button
      type="button"
      disabled={disabled || loading}
      className={`inline-flex items-center justify-center gap-1.5 cursor-pointer transition-all ${variantClassMap[variant]} ${sizeClassMap[size]} ${(disabled || loading) ? 'opacity-50 cursor-not-allowed' : ''} ${className}`.trim()}
      {...rest}
    >
      {loading ? (
        <span className="w-3.5 h-3.5 border-2 border-current border-t-transparent rounded-full animate-spin" />
      ) : (
        icon
      )}
      {children}
    </button>
  );
};

export interface IconButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  icon: React.ReactNode;
  ariaLabel: string;
  variant?: 'ghost' | 'secondary' | 'primary' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}

export const IconButton: React.FC<IconButtonProps> = ({
  icon,
  ariaLabel,
  variant = 'ghost',
  size = 'md',
  className = '',
  ...rest
}) => {
  const sizeClassMap: Record<string, string> = {
    sm: 'p-1 rounded-md',
    md: 'p-1.5 rounded-lg',
    lg: 'p-2 rounded-xl',
  };

  const variantClassMap: Record<string, string> = {
    ghost: 'interactive-muted',
    secondary: 'btn-secondary',
    primary: 'btn-primary',
    danger: 'btn-danger',
  };

  return (
    <button
      type="button"
      aria-label={ariaLabel}
      title={ariaLabel}
      className={`inline-flex items-center justify-center cursor-pointer transition-all ${variantClassMap[variant]} ${sizeClassMap[size]} ${className}`.trim()}
      {...rest}
    >
      {icon}
    </button>
  );
};
