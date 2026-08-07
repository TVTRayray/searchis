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

export interface InlineProps extends React.HTMLAttributes<HTMLDivElement> {
  gap?: GapSize;
  align?: 'flex-start' | 'center' | 'flex-end' | 'baseline';
  justify?: 'flex-start' | 'center' | 'flex-end' | 'space-between';
  className?: string;
  children?: React.ReactNode;
}

export const Inline: React.FC<InlineProps> = ({
  gap = 'sm',
  align = 'center',
  justify = 'flex-start',
  className = '',
  style = {},
  children,
  ...rest
}) => {
  const inlineStyle: React.CSSProperties = {
    display: 'inline-flex',
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: gapMap[gap],
    alignItems: align,
    justifyContent: justify,
    ...style,
  };

  return (
    <div style={inlineStyle} className={className} {...rest}>
      {children}
    </div>
  );
};
