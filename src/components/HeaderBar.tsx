import React from 'react';
import { ViewMode } from '../types/snippet';
import type { SettingsConfig } from '../api/snippets';
import { Zap, LayoutGrid, Settings, HelpCircle, Keyboard, ShieldCheck, Sun, Moon } from 'lucide-react';
import {
  Box,
  VStack,
  HStack,
  Inline,
} from './layout';
import { useTheme } from '../components/ThemeProvider';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Kbd } from '@/components/ui/kbd';

interface HeaderBarProps {
  currentView: ViewMode;
  onSelectView: (view: ViewMode) => void;
  config: SettingsConfig;
  onUpdateConfig: (key: string, value: unknown) => void;
  snippetCount: number;
  onOpenKeyboardHelp?: () => void;
  onOpenQuickPicker?: () => void;
}

const NAV_ITEMS: { view: ViewMode; label: string; icon: React.ReactNode }[] = [
  { view: 'quick-picker', label: '快速检索', icon: <Zap className="size-3.5" /> },
  { view: 'manager', label: '片段管理', icon: <LayoutGrid className="size-3.5" /> },
  { view: 'onboarding', label: '使用引导', icon: <HelpCircle className="size-3.5" /> },
  { view: 'settings', label: '设置', icon: <Settings className="size-3.5" /> },
];

export const HeaderBar: React.FC<HeaderBarProps> = ({
  currentView,
  onSelectView,
  config,
  onUpdateConfig,
  snippetCount,
  onOpenKeyboardHelp,
  onOpenQuickPicker,
}) => {
  const { theme, effectiveTheme, setTheme } = useTheme();

  const toggleTheme = () => {
    const newTheme = config.theme === 'dark' ? 'light' : 'dark';
    onUpdateConfig('theme', newTheme);
    setTheme(newTheme);
  };

  return (
    <header className="sticky top-0 z-40 w-full backdrop-blur-xl select-none bg-background/80 border-b">
      <HStack align="center" justify="space-between" className="max-w-7xl mx-auto px-4 h-14 gap-4">
        <HStack align="center" gap="sm" className="shrink-0">
          <Box className="brand-mark flex items-center justify-center w-8 h-8 rounded-lg font-bold text-sm">
            S
          </Box>
          <VStack gap="2xs">
            <HStack align="center" gap="xs">
              <span className="font-bold text-sm tracking-tight text-theme">Searchis</span>
              <Badge variant="secondary" className="rounded-full">{snippetCount} 条片段</Badge>
              <span className="h-1.5 w-1.5 rounded-full bg-[color:var(--color-success)]" aria-hidden />
            </HStack>
            <p className="text-xs text-theme-muted font-normal">Arch Linux 本机文本片段检索与粘贴工具</p>
          </VStack>
        </HStack>

        <Inline gap="2xs" className="p-1 rounded-lg bg-muted">
          {NAV_ITEMS.map(item => (
            <Button
              key={item.view}
              type="button"
              size="sm"
              variant={currentView === item.view ? 'secondary' : 'ghost'}
              className="h-8 px-3 gap-1.5 rounded-md text-xs"
              onClick={() => onSelectView(item.view)}
            >
              {item.icon}
              <span>{item.label}</span>
            </Button>
          ))}
        </Inline>

        <HStack align="center" gap="xs" className="shrink-0">
          <Badge variant="outline" className="hidden md:inline-flex gap-1.5 text-[11px] font-normal text-theme-muted rounded-full">
            <ShieldCheck className="size-3.5 text-[color:var(--color-success)]" />
            SQLCipher 加密数据库就绪
          </Badge>
          {onOpenKeyboardHelp && (
            <Button variant="ghost" size="icon" aria-label="键盘快捷键速查" onClick={onOpenKeyboardHelp}>
              <Keyboard className="size-4" />
            </Button>
          )}
          <Button variant="ghost" size="icon" aria-label={`切换主题`} onClick={toggleTheme}>
            {effectiveTheme === 'dark' ? <Sun className="size-4" /> : <Moon className="size-4" />}
          </Button>
          {onOpenQuickPicker && (
            <Button variant="default" size="sm" className="gap-1.5" onClick={onOpenQuickPicker}>
              <Zap className="size-3.5" />
              呼出快速窗口
              <Kbd className="ml-0.5">Ctrl+O</Kbd>
            </Button>
          )}
        </HStack>
      </HStack>
    </header>
  );
};
