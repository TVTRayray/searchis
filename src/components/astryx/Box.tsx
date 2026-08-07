import React from 'react';

export interface BoxProps extends React.HTMLAttributes<HTMLDivElement> {
  as?: React.ElementType;
  padding?: 'none' | '3xs' | '2xs' | 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl' | '3xl';
  paddingX?: 'none' | '3xs' | '2xs' | 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl';
  paddingY?: 'none' | '3xs' | '2xs' | 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl';
  margin?: 'none' | '3xs' | '2xs' | 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl';
  background?: 'canvas' | 'surface' | 'elevated' | 'subtle' | 'sunken' | 'titlebar' | 'transparent';
  border?: 'none' | 'all' | 'top' | 'bottom' | 'left' | 'right' | 'subtle';
  radius?: 'none' | 'xs' | 'sm' | 'md' | 'lg' | 'xl' | 'full';
  shadow?: 'none' | 'subtle' | 'window' | 'elevated' | 'accent';
  width?: string;
  height?: string;
  overflow?: 'visible' | 'hidden' | 'auto' | 'scroll';
  flex?: string;
  className?: string;
  children?: React.ReactNode;
}

const spacingMap: Record<string, string> = {
  none: '0px',
  '3xs': 'var(--spacing-3xs)',
  '2xs': 'var(--spacing-2xs)',
  xs: 'var(--spacing-xs)',
  sm: 'var(--spacing-sm)',
  md: 'var(--spacing-md)',
  lg: 'var(--spacing-lg)',
  xl: 'var(--spacing-xl)',
  '2xl': 'var(--spacing-2xl)',
  '3xl': 'var(--spacing-3xl)',
};

const radiusMap: Record<string, string> = {
  none: '0px',
  xs: 'var(--radius-xs)',
  sm: 'var(--radius-sm)',
  md: 'var(--radius-md)',
  lg: 'var(--radius-lg)',
  xl: 'var(--radius-xl)',
  full: 'var(--radius-full)',
};

const backgroundMap: Record<string, string> = {
  canvas: 'var(--color-canvas)',
  surface: 'var(--color-surface)',
  elevated: 'var(--color-surface-elevated)',
  subtle: 'var(--color-surface-subtle)',
  sunken: 'var(--color-surface-sunken)',
  titlebar: 'var(--color-surface-sunken)',
  transparent: 'transparent',
};

const shadowMap: Record<string, string> = {
  none: 'none',
  subtle: 'var(--shadow-subtle)',
  window: 'var(--shadow-window)',
  elevated: 'var(--shadow-elevated)',
  accent: 'var(--shadow-accent)',
};

export const Box: React.FC<BoxProps> = ({
  as: Component = 'div',
  padding,
  paddingX,
  paddingY,
  margin,
  background,
  border = 'none',
  radius,
  shadow = 'none',
  width,
  height,
  overflow,
  flex,
  className = '',
  style = {},
  children,
  ...rest
}) => {
  const customStyles: React.CSSProperties = {
    ...style,
    ...(padding && { padding: spacingMap[padding] }),
    ...(paddingX && { paddingLeft: spacingMap[paddingX], paddingRight: spacingMap[paddingX] }),
    ...(paddingY && { paddingTop: spacingMap[paddingY], paddingBottom: spacingMap[paddingY] }),
    ...(margin && { margin: spacingMap[margin] }),
    ...(background && { backgroundColor: backgroundMap[background] }),
    ...(radius && { borderRadius: radiusMap[radius] }),
    ...(shadow !== 'none' && { boxShadow: shadowMap[shadow] }),
    ...(width && { width }),
    ...(height && { height }),
    ...(overflow && { overflow }),
    ...(flex && { flex }),
  };

  let borderClasses = '';
  if (border === 'all') borderClasses = 'border theme-divider';
  else if (border === 'top') borderClasses = 'border-t theme-divider';
  else if (border === 'bottom') borderClasses = 'border-b theme-divider';
  else if (border === 'left') borderClasses = 'border-l theme-divider';
  else if (border === 'right') borderClasses = 'border-r theme-divider';
  else if (border === 'subtle') borderClasses = 'border theme-divider-subtle';

  return (
    <Component
      style={customStyles}
      className={`${borderClasses} ${className}`.trim()}
      {...rest}
    >
      {children}
    </Component>
  );
};
