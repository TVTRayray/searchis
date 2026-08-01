import React from 'react';
import { ViewMode, SettingsConfig } from '../types/snippet';
import { Zap, LayoutGrid, Settings, Moon, Sun, Search } from 'lucide-react';

interface HeaderBarProps {
  currentView: ViewMode;
  onSelectView: (view: ViewMode) => void;
  config: SettingsConfig;
  onUpdateConfig: (updater: (prev: SettingsConfig) => SettingsConfig) => void;
  snippetCount: number;
}

export const HeaderBar: React.FC<HeaderBarProps> = ({
  currentView,
  onSelectView,
  config,
  onUpdateConfig,
  snippetCount,
}) => {
  const toggleTheme = () => {
    onUpdateConfig(prev => ({
      ...prev,
      theme: prev.theme === 'dark' ? 'light' : 'dark'
    }));
  };

  return (
    <header className="sticky top-0 z-40 w-full border-b theme-divider bg-[color:var(--surface)] backdrop-blur-xl px-4 py-3">
      <div className="max-w-7xl mx-auto flex items-center justify-between gap-4">
        
        {/* Brand Logo & Title */}
        <div className="flex items-center gap-3">
          <div className="brand-mark flex items-center justify-center w-8 h-8 rounded-lg font-bold text-sm">
            <Search className="w-4 h-4" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <span className="font-semibold text-sm tracking-tight text-theme">
                Searchis
              </span>
            </div>
            <p className="text-xs text-theme-muted font-normal">
              本地文本片段检索工具
            </p>
          </div>
        </div>

        {/* Navigation Mode Switcher */}
        <nav className="flex items-center p-1 rounded-lg theme-surface-subtle border theme-divider">
          <button
            onClick={() => onSelectView('quick-picker')}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-all ${
              currentView === 'quick-picker'
                ? 'nav-active shadow-xs'
                : 'interactive-muted'
            }`}
          >
            <Zap className="w-3.5 h-3.5 icon-accent" />
            <span>快速检索</span>
          </button>

          <button
            onClick={() => onSelectView('manager')}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-all ${
              currentView === 'manager'
                ? 'nav-active shadow-xs'
                : 'interactive-muted'
            }`}
          >
            <LayoutGrid className="w-3.5 h-3.5" />
            <span>片段管理</span>
          </button>

          <button
            onClick={() => onSelectView('onboarding')}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-all ${
              currentView === 'onboarding'
                ? 'nav-active shadow-xs'
                : 'interactive-muted'
            }`}
          >
            <span>使用引导</span>
          </button>

          <button
            onClick={() => onSelectView('settings')}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-all ${
              currentView === 'settings'
                ? 'nav-active shadow-xs'
                : 'interactive-muted'
            }`}
          >
            <Settings className="w-3.5 h-3.5" />
            <span>设置</span>
          </button>
        </nav>

        {/* Right Tools & Theme Toggle */}
        <div className="flex items-center gap-2">
          <button
            onClick={toggleTheme}
            title="切换深色/浅色模式"
            className="p-2 rounded-lg interactive-muted"
          >
            {config.theme === 'dark' ? (
              <Sun className="w-4 h-4 icon-accent" />
            ) : (
              <Moon className="w-4 h-4 icon-accent" />
            )}
          </button>
        </div>

      </div>
    </header>
  );
};
