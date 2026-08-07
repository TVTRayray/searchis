import React, { useState, useEffect } from 'react';
import { Snippet, ViewMode, SettingsConfig } from './types/snippet';
import { initialSnippets } from './data/initialSnippets';
import {
  ThemeProvider,
  AppShell,
  Box,
  VStack,
  HStack,
  Button,
  Badge,
  Kbd,
  useTheme,
} from './components/astryx';
import { HeaderBar } from './components/HeaderBar';
import { QuickSearchWindow } from './components/QuickSearchWindow';
import { ManagerWindow } from './components/ManagerWindow';
import { SettingsModal } from './components/SettingsModal';
import { OnboardingWizard } from './components/OnboardingWizard';
import { KeyboardShortcutsModal } from './components/KeyboardShortcutsModal';
import { CheckCircle2, Zap } from 'lucide-react';

const DEFAULT_CONFIG: SettingsConfig = {
  globalShortcut: 'Option + Space',
  copyShortcut: 'Cmd + Enter',
  autoPaste: true,
  restoreClipboard: true,
  launchAtLogin: true,
  theme: 'system',
  playAudioFeedback: true,
  maxResultsCount: 20,
};

const MainContent: React.FC = () => {
  const [snippets, setSnippets] = useState<Snippet[]>(() => {
    try {
      const saved = localStorage.getItem('searchis_snippets_v1');
      return saved ? JSON.parse(saved) : initialSnippets;
    } catch {
      return initialSnippets;
    }
  });

  const [config, setConfig] = useState<SettingsConfig>(() => {
    try {
      const saved = localStorage.getItem('searchis_config_v1');
      return saved ? JSON.parse(saved) : DEFAULT_CONFIG;
    } catch {
      return DEFAULT_CONFIG;
    }
  });

  const [currentView, setCurrentView] = useState<ViewMode>('manager');
  const [isQuickPickerOpen, setIsQuickPickerOpen] = useState(false);
  const [isKeyboardHelpOpen, setIsKeyboardHelpOpen] = useState(false);
  const [toastMessage, setToastMessage] = useState<string | null>(null);
  const [prefillCreateKey, setPrefillCreateKey] = useState<string | undefined>();

  const { effectiveTheme } = useTheme();

  // Persist snippets
  useEffect(() => {
    try {
      localStorage.setItem('searchis_snippets_v1', JSON.stringify(snippets));
    } catch {
      // ignore
    }
  }, [snippets]);

  // Persist config
  useEffect(() => {
    try {
      localStorage.setItem('searchis_config_v1', JSON.stringify(config));
    } catch {
      // ignore
    }
  }, [config]);

  // Global Option+Space keyboard shortcut to toggle quick picker overlay
  useEffect(() => {
    const handleGlobalKeyDown = (e: KeyboardEvent) => {
      if (e.altKey && e.code === 'Space') {
        e.preventDefault();
        setIsQuickPickerOpen(prev => !prev);
      }
    };
    window.addEventListener('keydown', handleGlobalKeyDown);
    return () => window.removeEventListener('keydown', handleGlobalKeyDown);
  }, []);

  const showToast = (msg: string) => {
    setToastMessage(msg);
    setTimeout(() => setToastMessage(null), 2500);
  };

  const handleSaveSnippet = (snippet: Snippet) => {
    setSnippets(prev => {
      const exists = prev.some(s => s.id === snippet.id);
      if (exists) {
        return prev.map(s => s.id === snippet.id ? snippet : s);
      }
      return [snippet, ...prev];
    });
    showToast(`片段 "${snippet.title}" 已保存`);
  };

  const handleDeleteSnippet = (id: string) => {
    const target = snippets.find(s => s.id === id);
    if (!target) return;
    setSnippets(prev => prev.map(s => s.id === id ? { ...s, deletedAt: new Date().toISOString() } : s));
    showToast(`片段 "${target.title}" 已移入回收站`);
  };

  const handleRestoreSnippet = (id: string) => {
    const target = snippets.find(s => s.id === id);
    if (!target) return;
    setSnippets(prev => prev.map(s => s.id === id ? { ...s, deletedAt: undefined } : s));
    showToast(`片段 "${target.title}" 已恢复`);
  };

  const handlePermanentDeleteSnippet = (id: string) => {
    const target = snippets.find(s => s.id === id);
    if (!target) return;
    setSnippets(prev => prev.filter(s => s.id !== id));
    showToast(`片段 "${target.title}" 已彻底删除`);
  };

  const handleCopySnippet = (snippet: Snippet) => {
    navigator.clipboard.writeText(snippet.content).catch(() => {});
    setSnippets(prev => prev.map(s => s.id === snippet.id ? { ...s, usageCount: s.usageCount + 1, lastUsedAt: new Date().toISOString() } : s));
    showToast(`已复制 "${snippet.title}" 到剪贴板`);
  };

  const handlePasteSnippet = (snippet: Snippet) => {
    navigator.clipboard.writeText(snippet.content).catch(() => {});
    setSnippets(prev => prev.map(s => s.id === snippet.id ? { ...s, usageCount: s.usageCount + 1, lastUsedAt: new Date().toISOString() } : s));
    showToast(`已将 "${snippet.title}" 自动粘贴到目标窗口`);
    setIsQuickPickerOpen(false);
  };

  const handleEditSnippetFromQuickPicker = (snippet: Snippet) => {
    setIsQuickPickerOpen(false);
    setCurrentView('manager');
  };

  const handleCreateNewSnippetFromQuickPicker = (prefillKey?: string) => {
    setIsQuickPickerOpen(false);
    setPrefillCreateKey(prefillKey);
    setCurrentView('manager');
  };

  const handleExportData = () => {
    const dataStr = 'data:text/json;charset=utf-8,' + encodeURIComponent(JSON.stringify(snippets, null, 2));
    const downloadAnchor = document.createElement('a');
    downloadAnchor.setAttribute('href', dataStr);
    downloadAnchor.setAttribute('download', `searchis_snippets_${Date.now()}.json`);
    document.body.appendChild(downloadAnchor);
    downloadAnchor.click();
    downloadAnchor.remove();
    showToast('JSON 备份文件已导出');
  };

  const handleImportData = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (event) => {
      try {
        const imported = JSON.parse(event.target?.result as string);
        if (Array.isArray(imported)) {
          setSnippets(imported);
          showToast(`成功恢复 ${imported.length} 条片段数据`);
        }
      } catch {
        showToast('导入失败：JSON 格式不正确');
      }
    };
    reader.readAsText(file);
  };

  const handleResetSampleData = () => {
    setSnippets(initialSnippets);
    showToast('已恢复内置演示片段数据');
  };

  return (
    <AppShell
      title="Searchis"
      subtitle={`Arch Linux 本机文本片段工具 (${effectiveTheme === 'dark' ? '深色' : '浅色'}主题)`}
      nav={
        <HeaderBar
          currentView={currentView}
          onSelectView={setCurrentView}
          config={config}
          onUpdateConfig={setConfig}
          snippetCount={snippets.filter(s => !s.deletedAt).length}
          onOpenKeyboardHelp={() => setIsKeyboardHelpOpen(true)}
        />
      }
      actions={
        <Button
          variant="primary"
          size="sm"
          icon={<Zap className="w-3.5 h-3.5" />}
          onClick={() => setIsQuickPickerOpen(true)}
        >
          呼出快速窗口 <Kbd>⌥Space</Kbd>
        </Button>
      }
    >
      {/* Toast Notification */}
      {toastMessage && (
        <div className="fixed bottom-6 right-6 z-50 animate-pop-in">
          <Box paddingX="lg" paddingY="md" radius="lg" shadow="elevated" background="elevated" border="all" className="status-success flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4" />
            <span className="text-xs font-semibold">{toastMessage}</span>
          </Box>
        </div>
      )}

      {/* Main View Renderer */}
      {currentView === 'manager' && (
        <ManagerWindow
          snippets={snippets}
          onSaveSnippet={handleSaveSnippet}
          onDeleteSnippet={handleDeleteSnippet}
          onRestoreSnippet={handleRestoreSnippet}
          onPermanentDeleteSnippet={handlePermanentDeleteSnippet}
          onCopySnippet={handleCopySnippet}
          onPasteSnippet={handlePasteSnippet}
          onOpenSettings={() => setCurrentView('settings')}
        />
      )}

      {currentView === 'onboarding' && (
        <OnboardingWizard
          onComplete={() => setCurrentView('manager')}
          onCreateSnippet={(key, title, content) => {
            handleSaveSnippet({
              id: `snip-${Date.now()}`,
              key,
              title,
              content,
              aliases: [],
              tags: ['向导新建'],
              pinned: false,
              usageCount: 0,
              createdAt: new Date().toISOString(),
              updatedAt: new Date().toISOString(),
            });
          }}
        />
      )}

      {currentView === 'settings' && (
        <SettingsModal
          config={config}
          onUpdateConfig={setConfig}
          onExportData={handleExportData}
          onImportData={handleImportData}
          onResetSampleData={handleResetSampleData}
          onClose={() => setCurrentView('manager')}
        />
      )}

      {currentView === 'quick-picker' && (
        <QuickSearchWindow
          snippets={snippets.filter(s => !s.deletedAt)}
          onPasteSnippet={handlePasteSnippet}
          onCopySnippet={handleCopySnippet}
          onEditSnippet={handleEditSnippetFromQuickPicker}
          onCreateNewSnippet={handleCreateNewSnippetFromQuickPicker}
          onClose={() => setCurrentView('manager')}
        />
      )}

      {/* Floating Quick Search Overlay (Triggerable from any screen via ⌥Space button or hotkey) */}
      {isQuickPickerOpen && (
        <QuickSearchWindow
          snippets={snippets.filter(s => !s.deletedAt)}
          onPasteSnippet={handlePasteSnippet}
          onCopySnippet={handleCopySnippet}
          onEditSnippet={handleEditSnippetFromQuickPicker}
          onCreateNewSnippet={handleCreateNewSnippetFromQuickPicker}
          onClose={() => setIsQuickPickerOpen(false)}
        />
      )}

      {/* Keyboard Shortcuts Help Modal */}
      <KeyboardShortcutsModal
        isOpen={isKeyboardHelpOpen}
        onClose={() => setIsKeyboardHelpOpen(false)}
      />
    </AppShell>
  );
};

export const App: React.FC = () => {
  return (
    <ThemeProvider>
      <MainContent />
    </ThemeProvider>
  );
};
