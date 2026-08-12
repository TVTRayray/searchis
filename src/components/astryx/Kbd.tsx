import React from 'react';

export interface KbdProps {
  className?: string;
  children: React.ReactNode;
}

export const Kbd: React.FC<KbdProps> = ({ className = '', children }) => {
  return (
    <kbd className={`raycast-kbd ${className}`.trim()}>
      {children}
    </kbd>
  );
};
