import React, { useState, useEffect } from 'react';
import { Snippet, ViewMode, SettingsConfig } from './types/snippet';
import { initialSnippets } from './data/initialSnippets';
import { HeaderBar } from './components/HeaderBar';
import { QuickSearchWindow } from './components/QuickSearchWindow';
import { ManagerWindow } from './components/ManagerWindow';
import { OnboardingWizard } from './components/OnboardingWizard';
import { SettingsModal } from './components/SettingsModal';
import { CheckCircle2, Copy, AlertCircle, X } from 'lucide-react';

export const App: React.FC = () => {
  // Load snippets from localStorage or fallback to initialSnippets
  const [snippets, setSnippets] = useState<Snippet[]>(() => {
    const saved = localStorage.getItem('searchis_snippets_v1');
    if (saved) {
      try { return JSON.parse(saved); } catch { return initialSnippets; }
    }
    return initialSnippets;
  });

  // Settings Configuration
  const [config, setConfig] = useState<SettingsConfig>(() => {
    const saved = localStorage.getItem('searchis_config_v1');
    if (saved) {
      try { return JSON.parse(saved); } catch { /* ignore */ }
    }
    return {
      globalShortcut: 'Option + Space',
      copyShortcut: 'Cmd + Enter',
      autoPaste: true,
      restoreClipboard: false,
      launchAtLogin: true,
      theme: 'dark',
      accentColor: 'blue',
      playAudioFeedback: true,
      maxResultsCount: 20
    };
  });

  // View state
  const [currentView, setCurrentView] = useState<ViewMode>('quick-picker');
  const [editingSnippet, setEditingSnippet] = useState<Snippet | null>(null);

  // Toast Notification state
  const [toast, setToast] = useState<{
    id: string;
    type: 'success' | 'info' | 'error';
    title: string;
    message: string;
  } | null>(null);

  // Save snippets to localStorage
  useEffect(() => {
    localStorage.setItem('searchis_snippets_v1', JSON.stringify(snippets));
  }, [snippets]);

  // Save config to localStorage
  useEffect(() => {
    localStorage.setItem('searchis_config_v1', JSON.stringify(config));
  }, [config]);

  // Apply dark mode class to html element
  useEffect(() => {
    const root = document.documentElement;
    if (config.theme === 'dark') {
      root.classList.add('dark');
    } else if (config.theme === 'light') {
      root.classList.remove('dark');
    } else {
      // System mode
      if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
        root.classList.add('dark');
      } else {
        root.classList.remove('dark');
      }
    }
  }, [config.theme]);

  const showToast = (title: string, message: string, type: 'success' | 'info' | 'error' = 'success') => {
    setToast({
      id: `${Date.now()}`,
      type,
      title,
      message,
    });
    setTimeout(() => {
      setToast(null);
    }, 3500);
  };

  // Handlers for Snippet Actions
  const handlePasteSnippet = (snippet: Snippet) => {
    // Write to clipboard simulation
    navigator.clipboard.writeText(snippet.content).catch(() => {});

    // Update usage count & lastUsedAt
    setSnippets(prev =>
      prev.map(s =>
        s.id === snippet.id
          ? {
              ...s,
              usageCount: s.usageCount + 1,
              lastUsedAt: new Date().toISOString(),
            }
          : s
      )
    );

    showToast(
      '粘贴成功 (Simulated Paste)',
      `已写剪贴板并粘贴 Key "${snippet.key}" 到当前活动应用`,
      'success'
    );
  };

  const handleCopySnippet = (snippet: Snippet) => {
    navigator.clipboard.writeText(snippet.content).catch(() => {});
    showToast(
      '已仅复制到剪贴板',
      `片段 "${snippet.title}" 已写入系统剪贴板 (⌘↵)`,
      'info'
    );
  };

  const handleSaveSnippet = (snippetToSave: Snippet) => {
    setSnippets(prev => {
      const exists = prev.some(s => s.id === snippetToSave.id);
      if (exists) {
        return prev.map(s => (s.id === snippetToSave.id ? snippetToSave : s));
      }
      return [snippetToSave, ...prev];
    });
    showToast('片段已保存', `Key "${snippetToSave.key}" 更新成功`, 'success');
  };

  const handleDeleteSnippet = (id: string) => {
    setSnippets(prev =>
      prev.map(s => (s.id === id ? { ...s, deletedAt: new Date().toISOString() } : s))
    );
    showToast('已移入回收站', '片段已被移动至回收站，可随时还原', 'info');
  };

  const handleRestoreSnippet = (id: string) => {
    setSnippets(prev =>
      prev.map(s => {
        if (s.id === id) {
          const { deletedAt, ...rest } = s;
          return rest;
        }
        return s;
      })
    );
    showToast('还原成功', '片段已重新恢复到列表', 'success');
  };

  const handlePermanentDeleteSnippet = (id: string) => {
    setSnippets(prev => prev.filter(s => s.id !== id));
    showToast('彻底删除', '已从本地存储中永久清空该片段', 'error');
  };

  const handleExportData = () => {
    const dataStr = 'data:text/json;charset=utf-8,' + encodeURIComponent(JSON.stringify(snippets, null, 2));
    const downloadAnchor = document.createElement('a');
    downloadAnchor.setAttribute('href', dataStr);
    downloadAnchor.setAttribute('download', `searchis_snippets_backup_${new Date().toISOString().slice(0, 10)}.json`);
    document.body.appendChild(downloadAnchor);
    downloadAnchor.click();
    downloadAnchor.remove();
    showToast('导出成功', '已生成 JSON 备份数据文件', 'success');
  };

  const handleImportData = (e: React.ChangeEvent<HTMLInputElement>) => {
    const fileReader = new FileReader();
    if (e.target.files && e.target.files[0]) {
      fileReader.readAsText(e.target.files[0], 'UTF-8');
      fileReader.onload = (event) => {
        try {
          const parsed = JSON.parse(event.target?.result as string);
          if (Array.isArray(parsed)) {
            setSnippets(parsed);
            showToast('导入成功', `成功从备份恢复 ${parsed.length} 条文本片段`, 'success');
          }
        } catch {
          showToast('导入失败', 'JSON 文件格式不合规', 'error');
        }
      };
    }
  };

  const handleResetSampleData = () => {
    setSnippets(initialSnippets);
    showToast('已恢复初始数据', '初始化 9 条演示 Key 文本片段', 'info');
  };

  return (
    <div className="min-h-screen bg-zinc-50 dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100 flex flex-col justify-between">
      
      {/* Top Header Navigation Bar */}
      <HeaderBar
        currentView={currentView}
        onSelectView={setCurrentView}
        config={config}
        onUpdateConfig={setConfig}
        snippetCount={snippets.filter(s => !s.deletedAt).length}
      />

      {/* Main Active View Container */}
      <main className="flex-1 py-6 px-2 sm:px-4">
        {currentView === 'quick-picker' && (
          <QuickSearchWindow
            snippets={snippets.filter(s => !s.deletedAt)}
            onPasteSnippet={handlePasteSnippet}
            onCopySnippet={handleCopySnippet}
            onEditSnippet={(snippet) => {
              setEditingSnippet(snippet);
              setCurrentView('manager');
            }}
            onCreateNewSnippet={(prefillKey) => {
              setCurrentView('manager');
            }}
          />
        )}

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
            onComplete={() => setCurrentView('quick-picker')}
            onCreateSnippet={(key, title, content) => {
              const newSnip: Snippet = {
                id: `snip-${Date.now()}`,
                key,
                title,
                content,
                aliases: ['guide'],
                tags: ['常用'],
                pinned: true,
                usageCount: 1,
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
              };
              handleSaveSnippet(newSnip);
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
            onClose={() => setCurrentView('quick-picker')}
          />
        )}
      </main>

      {/* Toast Notification */}
      {toast && (
        <div className="fixed bottom-6 right-6 z-50 max-w-sm">
          <div className="p-3.5 rounded-lg bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 shadow-md flex items-start gap-3">
            <div className="mt-0.5">
              {toast.type === 'success' && <CheckCircle2 className="w-5 h-5 text-emerald-500" />}
              {toast.type === 'info' && <Copy className="w-5 h-5 text-blue-500" />}
              {toast.type === 'error' && <AlertCircle className="w-5 h-5 text-red-500" />}
            </div>
            <div className="flex-1 pr-2">
              <h4 className="text-sm font-bold text-zinc-900 dark:text-zinc-100">
                {toast.title}
              </h4>
              <p className="text-xs text-zinc-600 dark:text-zinc-400 mt-0.5 leading-tight">
                {toast.message}
              </p>
            </div>
            <button
              onClick={() => setToast(null)}
              className="text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-200 p-0.5"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
