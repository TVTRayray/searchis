import React from 'react';
import { Box } from './Box';

export interface CardProps {
  title?: React.ReactNode;
  subtitle?: React.ReactNode;
  headerAction?: React.ReactNode;
  footer?: React.ReactNode;
  padding?: 'none' | 'xs' | 'sm' | 'md' | 'lg' | 'xl';
  className?: string;
  children: React.ReactNode;
}

export const Card: React.FC<CardProps> = ({
  title,
  subtitle,
  headerAction,
  footer,
  padding = 'lg',
  className = '',
  children,
}) => {
  return (
    <Box
      background="elevated"
      border="all"
      radius="md"
      shadow="subtle"
      className={`overflow-hidden ${className}`}
    >
      {(title || headerAction) && (
        <Box paddingX="lg" paddingY="md" border="bottom" background="subtle" className="flex items-center justify-between gap-3">
          <div>
            {title && <h3 className="text-sm font-bold text-theme leading-tight">{title}</h3>}
            {subtitle && <p className="text-xs text-theme-muted mt-0.5">{subtitle}</p>}
          </div>
          {headerAction && <div className="shrink-0">{headerAction}</div>}
        </Box>
      )}
      <Box padding={padding}>
        {children}
      </Box>
      {footer && (
        <Box paddingX="lg" paddingY="md" border="top" background="subtle">
          {footer}
        </Box>
      )}
    </Box>
  );
};
