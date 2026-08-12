import React from 'react';
import { Box } from './layout/Box';

export interface AppShellProps {
  className?: string;
  children: React.ReactNode;
}

/**
 * Application shell — ambient canvas only. All chrome (header, nav, actions)
 * lives in HeaderBar; this component owns the background and the body region.
 */
export const AppShell: React.FC<AppShellProps> = ({ className = '', children }) => {
  return (
    <Box
      width="100%"
      height="100vh"
      background="canvas"
      className={`flex flex-col overflow-hidden ambient-glow-bg select-none ${className}`}
    >
      <Box flex="1" overflow="hidden" className="w-full relative">
        {children}
      </Box>
    </Box>
  );
};
