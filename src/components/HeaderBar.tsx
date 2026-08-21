import React from 'react';
import { Keyboard, Moon, Settings, Sun } from 'lucide-react';
import { ViewMode } from '../types/snippet';
import { SettingsConfig } from '../api/snippets';
import { useTheme } from './ThemeProvider';

interface HeaderBarProps {
  currentView: ViewMode;
  onSelectView: (view: ViewMode) => void;
  config: SettingsConfig;
  onUpdateConfig: (key: string, value: unknown) => void;
  onOpenKeyboardHelp?: () => void;
}

export const HeaderBar: React.FC<HeaderBarProps> = ({
  currentView,
  onSelectView,
  config,
  onUpdateConfig,
  onOpenKeyboardHelp,
}) => {
  const { effectiveTheme, setTheme } = useTheme();
  const themeLabel = config.theme === 'system'
    ? `跟随系统 · ${effectiveTheme === 'dark' ? '深色' : '浅色'}`
    : config.theme === 'dark' ? '深色' : '浅色';

  const toggleTheme = () => {
    const nextTheme = effectiveTheme === 'dark' ? 'light' : 'dark';
    setTheme(nextTheme);
    onUpdateConfig('theme', nextTheme);
  };

  return (
    <header className="manager-header">
      <div className="manager-header-inner">
        <div className="manager-brand">
          <div className="manager-brand-mark" aria-label="Searchis" title="Searchis">S</div>
        </div>

        <div className="manager-header-actions">
          {onOpenKeyboardHelp && (
            <button
              type="button"
              className="manager-icon-button"
              aria-label="键盘快捷键速查"
              title="键盘快捷键速查"
              onClick={onOpenKeyboardHelp}
            >
              <Keyboard className="size-4" aria-hidden />
            </button>
          )}
          <button
            type="button"
            className="manager-icon-button"
            aria-label={`切换主题（当前：${themeLabel}）`}
            title={`切换主题（当前：${themeLabel}）`}
            onClick={toggleTheme}
          >
            {effectiveTheme === 'dark' ? <Sun className="size-4" aria-hidden /> : <Moon className="size-4" aria-hidden />}
          </button>
          <button
            type="button"
            className={`manager-icon-button ${currentView === 'settings' ? 'is-active' : ''}`}
            aria-label={currentView === 'settings' ? '返回片段管理' : '打开设置'}
            title={currentView === 'settings' ? '返回片段管理' : '打开设置'}
            aria-pressed={currentView === 'settings'}
            onClick={() => onSelectView(currentView === 'settings' ? 'manager' : 'settings')}
          >
            <Settings className="size-4" aria-hidden />
          </button>
        </div>
      </div>
    </header>
  );
};
