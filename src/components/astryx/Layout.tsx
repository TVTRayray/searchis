import React from 'react';
import { Box } from './Box';

export interface LayoutProps {
  direction?: 'row' | 'column';
  width?: string;
  height?: string;
  className?: string;
  children: React.ReactNode;
}

export const Layout: React.FC<LayoutProps> = ({
  direction = 'row',
  width = '100%',
  height = '100%',
  className = '',
  children,
}) => {
  return (
    <Box
      style={{
        display: 'flex',
        flexDirection: direction,
        width,
        height,
        minHeight: 0,
      }}
      className={className}
    >
      {children}
    </Box>
  );
};

export interface LayoutPanelProps {
  width?: string;
  minWidth?: string;
  maxWidth?: string;
  height?: string;
  flex?: string;
  background?: 'canvas' | 'surface' | 'elevated' | 'subtle' | 'sunken' | 'titlebar' | 'transparent';
  border?: 'none' | 'all' | 'top' | 'bottom' | 'left' | 'right' | 'subtle';
  padding?: 'none' | '3xs' | '2xs' | 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl' | '3xl';
  overflow?: 'visible' | 'hidden' | 'auto' | 'scroll';
  className?: string;
  children?: React.ReactNode;
}

export const LayoutPanel: React.FC<LayoutPanelProps> = ({
  width,
  minWidth,
  maxWidth,
  height,
  flex,
  background,
  border = 'none',
  padding = 'none',
  overflow = 'auto',
  className = '',
  children,
}) => {
  return (
    <Box
      width={width}
      height={height}
      background={background}
      border={border}
      padding={padding}
      overflow={overflow}
      style={{
        ...(minWidth && { minWidth }),
        ...(maxWidth && { maxWidth }),
        ...(flex && { flex }),
      }}
      className={`min-h-0 min-w-0 ${className}`}
    >
      {children}
    </Box>
  );
};
