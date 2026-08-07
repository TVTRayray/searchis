import React from 'react';
import { Box } from './Box';
import { VStack } from './Stack';

export interface SideNavProps {
  width?: string;
  className?: string;
  header?: React.ReactNode;
  footer?: React.ReactNode;
  children: React.ReactNode;
}

export const SideNav: React.FC<SideNavProps> = ({
  width = '240px',
  className = '',
  header,
  footer,
  children,
}) => {
  return (
    <Box
      as="nav"
      width={width}
      height="100%"
      background="sunken"
      border="right"
      padding="sm"
      className={`flex flex-col justify-between select-none ${className}`}
    >
      <VStack gap="sm" style={{ height: '100%', minHeight: 0 }}>
        {header && <Box paddingY="xs" paddingX="xs">{header}</Box>}
        <Box flex="1" overflow="auto" className="w-full space-y-1">
          {children}
        </Box>
        {footer && <Box paddingY="xs" border="top">{footer}</Box>}
      </VStack>
    </Box>
  );
};

export interface SideNavItemProps {
  active?: boolean;
  disabled?: boolean;
  icon?: React.ReactNode;
  badge?: React.ReactNode;
  onClick?: () => void;
  className?: string;
  children: React.ReactNode;
}

export const SideNavItem: React.FC<SideNavItemProps> = ({
  active = false,
  disabled = false,
  icon,
  badge,
  onClick,
  className = '',
  children,
}) => {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className={`w-full flex items-center justify-between px-3 py-2 rounded-xl text-xs font-medium transition-all cursor-pointer ${
        active
          ? 'nav-active font-semibold shadow-xs'
          : 'interactive-muted'
      } ${disabled ? 'opacity-50 cursor-not-allowed' : ''} ${className}`}
    >
      <div className="flex items-center gap-2 min-w-0">
        {icon && <span className="shrink-0 flex items-center">{icon}</span>}
        <span className="truncate">{children}</span>
      </div>
      {badge && <span className="shrink-0 ml-2">{badge}</span>}
    </button>
  );
};
