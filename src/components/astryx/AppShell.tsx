import React from 'react';
import { Box } from './Box';
import { HStack, VStack } from './Stack';
import { StatusDot } from './StatusDot';
import { useTheme } from './ThemeProvider';
import { Sun, Moon, Database, ShieldCheck } from 'lucide-react';
import { IconButton } from './Button';
import { Badge } from './Badge';

export interface AppShellProps {
  title?: React.ReactNode;
  subtitle?: React.ReactNode;
  statusText?: React.ReactNode;
  nav?: React.ReactNode;
  actions?: React.ReactNode;
  className?: string;
  children: React.ReactNode;
}

export const AppShell: React.FC<AppShellProps> = ({
  title = 'Searchis',
  subtitle = 'Arch Linux 本机文本片段检索与粘贴工具',
  statusText = 'SQLCipher 加密数据库就绪',
  nav,
  actions,
  className = '',
  children,
}) => {
  const { theme, effectiveTheme, setTheme } = useTheme();

  const toggleTheme = () => {
    if (theme === 'dark') setTheme('light');
    else if (theme === 'light') setTheme('dark');
    else setTheme(effectiveTheme === 'dark' ? 'light' : 'dark');
  };

  return (
    <Box
      width="100%"
      height="100vh"
      background="canvas"
      className={`flex flex-col overflow-hidden ambient-glow-bg select-none ${className}`}
    >
      {/* Top Application Bar */}
      <Box
        as="header"
        background="surface"
        border="bottom"
        paddingX="lg"
        paddingY="sm"
        shadow="subtle"
        className="flex-none backdrop-blur-xl z-40"
      >
        <HStack align="center" justify="space-between" gap="md">
          {/* Logo & Product Title */}
          <HStack align="center" gap="sm">
            <Box
              width="32px"
              height="32px"
              radius="md"
              className="brand-mark flex items-center justify-center font-bold text-sm"
            >
              S
            </Box>
            <VStack gap="none">
              <HStack align="center" gap="xs">
                <span className="font-bold text-sm tracking-tight text-theme">{title}</span>
                <StatusDot status="success" pulse size="sm" />
              </HStack>
              <HStack align="center" gap="xs">
                {subtitle && <p className="text-[11px] text-theme-muted font-normal m-0">{subtitle}</p>}
                <Badge variant="neutral" size="sm">{effectiveTheme === 'dark' ? 'Dark' : 'Light'}</Badge>
              </HStack>
            </VStack>
          </HStack>

          {/* Navigation Items */}
          {nav && <Box className="shrink-0">{nav}</Box>}

          {/* Right Action Tools & Theme Switcher */}
          <HStack align="center" gap="sm">
            {statusText && (
              <Box paddingX="xs" paddingY="3xs" radius="xs" background="subtle" border="subtle" className="hidden md:flex items-center gap-1.5 text-[11px] text-theme-muted">
                <ShieldCheck className="w-3.5 h-3.5 icon-success" />
                <span>{statusText}</span>
              </Box>
            )}

            {actions}

            {/* Theme Toggle Button */}
            <IconButton
              icon={effectiveTheme === 'dark' ? <Sun className="w-4 h-4 icon-accent" /> : <Moon className="w-4 h-4 icon-accent" />}
              ariaLabel={`切换主题 (当前: ${theme === 'system' ? '跟随系统 (' + effectiveTheme + ')' : theme === 'dark' ? '深色' : '浅色'})`}
              onClick={toggleTheme}
              variant="secondary"
              size="md"
            />
          </HStack>
        </HStack>
      </Box>

      {/* Main Application Body Container */}
      <Box flex="1" overflow="hidden" className="w-full relative">
        {children}
      </Box>
    </Box>
  );
};
