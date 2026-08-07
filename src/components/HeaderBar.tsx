import React from 'react';
import { ViewMode, SettingsConfig } from '../types/snippet';
import { Zap, LayoutGrid, Settings, Moon, Sun, HelpCircle, Keyboard } from 'lucide-react';
import {
  Box,
  VStack,
  HStack,
  Inline,
  Button,
  IconButton,
  useTheme,
  StatusDot,
  Badge,
} from './astryx';

interface HeaderBarProps {
  currentView: ViewMode;
  onSelectView: (view: ViewMode) => void;
  config: SettingsConfig;
  onUpdateConfig: (updater: (prev: SettingsConfig) => SettingsConfig) => void;
  snippetCount: number;
  onOpenKeyboardHelp?: () => void;
  /** 呼出独立检索窗口（双窗口形态） */
  onOpenQuickPicker?: () => void;
}

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
    if (config.theme === 'dark') {
      onUpdateConfig(prev => ({ ...prev, theme: 'light' }));
      setTheme('light');
    } else {
      onUpdateConfig(prev => ({ ...prev, theme: 'dark' }));
      setTheme('dark');
    }
  };

  return (
    <Box
      as="header"
      background="surface"
      border="bottom"
      paddingX="lg"
      paddingY="sm"
      shadow="subtle"
      className="sticky top-0 z-40 w-full backdrop-blur-xl select-none"
    >
      <HStack align="center" justify="space-between" className="max-w-7xl mx-auto">
        {/* Product Brand Title */}
        <HStack align="center" gap="sm">
          <Box className="brand-mark flex items-center justify-center w-8 h-8 rounded-lg font-bold text-sm">
            S
          </Box>
          <VStack gap="2xs">
            <HStack align="center" gap="xs">
              <span className="font-bold text-sm tracking-tight text-theme">
                Searchis
              </span>
              <Badge variant="accent" size="sm">{snippetCount} 条片段</Badge>
              <StatusDot status="active" size="sm" />
            </HStack>
            <p className="text-xs text-theme-muted font-normal">
              Arch Linux 本机文本片段检索与粘贴工具
            </p>
          </VStack>
        </HStack>

        {/* View Navigation Switcher */}
        <Inline gap="2xs" className="p-1 rounded-xl theme-surface-subtle border theme-divider">
          <button
            type="button"
            onClick={() => onOpenQuickPicker?.()}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
              currentView === 'quick-picker'
                ? 'nav-active shadow-xs'
                : 'interactive-muted'
            }`}
          >
            <Zap className="w-3.5 h-3.5 icon-accent" />
            <span>快速检索</span>
          </button>

          <button
            type="button"
            onClick={() => onSelectView('manager')}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
              currentView === 'manager'
                ? 'nav-active shadow-xs'
                : 'interactive-muted'
            }`}
          >
            <LayoutGrid className="w-3.5 h-3.5" />
            <span>片段管理</span>
          </button>

          <button
            type="button"
            onClick={() => onSelectView('onboarding')}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
              currentView === 'onboarding'
                ? 'nav-active shadow-xs'
                : 'interactive-muted'
            }`}
          >
            <HelpCircle className="w-3.5 h-3.5" />
            <span>使用引导</span>
          </button>

          <button
            type="button"
            onClick={() => onSelectView('settings')}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
              currentView === 'settings'
                ? 'nav-active shadow-xs'
                : 'interactive-muted'
            }`}
          >
            <Settings className="w-3.5 h-3.5" />
            <span>设置</span>
          </button>
        </Inline>

        {/* Theme & Keyboard Help Actions */}
        <HStack align="center" gap="xs">
          {onOpenKeyboardHelp && (
            <IconButton
              icon={<Keyboard className="w-4 h-4" />}
              ariaLabel="键盘快捷键速查"
              onClick={onOpenKeyboardHelp}
              variant="ghost"
            />
          )}

          <IconButton
            icon={effectiveTheme === 'dark' ? <Sun className="w-4 h-4 icon-accent" /> : <Moon className="w-4 h-4 icon-accent" />}
            ariaLabel={`切换深浅色主题 (当前: ${effectiveTheme === 'dark' ? '深色' : '浅色'})`}
            onClick={toggleTheme}
            variant="ghost"
          />
        </HStack>
      </HStack>
    </Box>
  );
};
