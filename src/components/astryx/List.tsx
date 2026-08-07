import React, { forwardRef } from 'react';
import { Box } from './Box';

export interface ListProps extends React.HTMLAttributes<HTMLUListElement> {
  bordered?: boolean;
  divided?: boolean;
  className?: string;
  children: React.ReactNode;
}

export const List = forwardRef<HTMLUListElement, ListProps>(({
  bordered = false,
  divided = true,
  className = '',
  children,
  ...rest
}, ref) => {
  return (
    <ul
      ref={ref}
      className={`w-full divide-y ${divided ? 'theme-divider-subtle' : ''} ${
        bordered ? 'border theme-divider rounded-lg' : ''
      } ${className}`.trim()}
      style={{ margin: 0, padding: 0, listStyle: 'none' }}
      {...rest}
    >
      {children}
    </ul>
  );
});

List.displayName = 'List';

export interface ListItemProps {
  active?: boolean;
  disabled?: boolean;
  onClick?: () => void;
  icon?: React.ReactNode;
  title: React.ReactNode;
  subtitle?: React.ReactNode;
  meta?: React.ReactNode;
  extra?: React.ReactNode;
  className?: string;
}

export const ListItem: React.FC<ListItemProps> = ({
  active = false,
  disabled = false,
  onClick,
  icon,
  title,
  subtitle,
  meta,
  extra,
  className = '',
}) => {
  return (
    <li
      onClick={disabled ? undefined : onClick}
      className={`w-full flex items-center justify-between px-3 py-2.5 transition-all text-xs ${
        onClick && !disabled ? 'cursor-pointer' : ''
      } ${
        active
          ? 'raycast-item-active font-semibold'
          : 'interactive-muted'
      } ${disabled ? 'opacity-50 cursor-not-allowed' : ''} ${className}`}
    >
      <div className="flex items-center gap-3 min-w-0 flex-1 pr-2">
        {icon && <div className="shrink-0 flex items-center justify-center">{icon}</div>}
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="font-medium text-theme text-sm truncate">{title}</span>
            {meta}
          </div>
          {subtitle && <div className="text-xs text-theme-muted truncate mt-0.5">{subtitle}</div>}
        </div>
      </div>
      {extra && <div className="shrink-0 flex items-center gap-2">{extra}</div>}
    </li>
  );
};
