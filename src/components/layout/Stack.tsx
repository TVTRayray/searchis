import React from 'react';

type GapSize = 'none' | '3xs' | '2xs' | 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl' | '3xl';

const gapMap: Record<GapSize, string> = {
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

export interface StackProps extends React.HTMLAttributes<HTMLDivElement> {
  direction?: 'row' | 'column' | 'row-reverse' | 'column-reverse';
  gap?: GapSize;
  align?: 'stretch' | 'flex-start' | 'center' | 'flex-end' | 'baseline';
  justify?: 'flex-start' | 'center' | 'flex-end' | 'space-between' | 'space-around';
  wrap?: 'nowrap' | 'wrap' | 'wrap-reverse';
  flex?: string;
  className?: string;
  children?: React.ReactNode;
}

export const Stack: React.FC<StackProps> = ({
  direction = 'column',
  gap = 'md',
  align = 'stretch',
  justify = 'flex-start',
  wrap = 'nowrap',
  flex,
  className = '',
  style = {},
  children,
  ...rest
}) => {
  const stackStyle: React.CSSProperties = {
    display: 'flex',
    flexDirection: direction,
    gap: gapMap[gap],
    alignItems: align,
    justifyContent: justify,
    flexWrap: wrap,
    ...(flex && { flex }),
    ...style,
  };

  return (
    <div style={stackStyle} className={className} {...rest}>
      {children}
    </div>
  );
};

export const VStack: React.FC<Omit<StackProps, 'direction'>> = (props) => (
  <Stack direction="column" {...props} />
);

export const HStack: React.FC<Omit<StackProps, 'direction'>> = (props) => (
  <Stack direction="row" {...props} />
);
