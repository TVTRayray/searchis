import React from 'react';

type GapSize = 'none' | '3xs' | '2xs' | 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl';

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
};

export interface GridProps extends React.HTMLAttributes<HTMLDivElement> {
  columns?: number | string;
  gap?: GapSize;
  rowGap?: GapSize;
  columnGap?: GapSize;
  align?: 'stretch' | 'flex-start' | 'center' | 'flex-end';
  className?: string;
  children?: React.ReactNode;
}

export const Grid: React.FC<GridProps> = ({
  columns = 1,
  gap = 'md',
  rowGap,
  columnGap,
  align,
  className = '',
  style = {},
  children,
  ...rest
}) => {
  const templateColumns = typeof columns === 'number' ? `repeat(${columns}, minmax(0, 1fr))` : columns;

  const gridStyle: React.CSSProperties = {
    display: 'grid',
    gridTemplateColumns: templateColumns,
    gap: gapMap[gap],
    ...(rowGap && { rowGap: gapMap[rowGap] }),
    ...(columnGap && { columnGap: gapMap[columnGap] }),
    ...(align && { alignItems: align }),
    ...style,
  };

  return (
    <div style={gridStyle} className={className} {...rest}>
      {children}
    </div>
  );
};
