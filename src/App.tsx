import React, { useState, useEffect, useCallback, useRef } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ViewMode } from './types/snippet';
import { SettingsConfig } from './api/snippets';
import {
  AppError,
  ManagerRequest,
  PersistedSnippet,
  SearchResultItem,
  settingsApi,
  snippetsApi,
  windowApi,
} from './api/snippets';
import {
  ThemeProvider,
  AppShell,
  Box,
  VStack,
  HStack,
  Button,
  Kbd,
  useTheme,
} from './components/astryx';
import { HeaderBar } from './components/HeaderBar';
import { QuickSearchWindow } from './components/QuickSearchWindow';
import { ManagerWindow } from './components/ManagerWindow';
import { SettingsModal } from './components/SettingsModal';
import { OnboardingWizard } from './components/OnboardingWizard';
import { KeyboardShortcutsModal } from './components/KeyboardShortcutsModal';
import { CheckCircle2, AlertCircle, Zap } from 'lucide-react';

// 检测当前 Tauri 窗口标签：search = 独立无边框检索窗口，其余（main）= 管理窗口。
// 纯浏览器（vite dev 无 Tauri）环境回退为管理窗口形态。
let WINDOW_LABEL = 'main';
try {
  WINDOW_LABEL = getCurrentWindow().label;
} catch {
  // 非 Tauri 环境
}

const DEFAULT_CONFIG: SettingsConfig = {
  globalShortcut: 'Alt+O',
  autoPaste: true,
  restoreClipboard: false,
  launchAtLogin: true,
  theme: 'system',
  maxResultsCount: 20,
  trashAutoPurgeDays: null,
  onboardingCompletedAt: null,
  schemaVersion: 1,
};

const errorMessage = (e: unknown): string =>
  (e as AppError)?.message ?? (e as { message?: string })?.message ?? '操作失败';

/* ============ 检索窗口（独立无边框窗口） ============ */
const SearchWindowApp: React.FC = () => {
  const [maxResults, setMaxResults] = useState(DEFAULT_CONFIG.maxResultsCount);

  useEffect(() => {
    settingsApi.get().then(res => setMaxResults(res.settings.maxResultsCount)).catch(() => {});
  }, []);

  const handleClose = useCallback(() => {
    windowApi.closeSearch().catch(() => {});
  }, []);

  const handleEdit = useCallback((item: SearchResultItem) => {
    windowApi
      .closeSearch()
      .then(() => windowApi.openManager({ editId: item.id }))
      .catch(() => {});
  }, []);

  const handleCreate = useCallback((prefillKey?: string) => {
    windowApi
      .closeSearch()
      .then(() => windowApi.openManager(prefillKey ? { prefillKey } : undefined))
      .catch(() => {});
  }, []);

  return (
    <QuickSearchWindow
      onClose={handleClose}
      onEditSnippet={handleEdit}
      onCreateNewSnippet={handleCreate}
      maxResultsCount={maxResults}
    />
  );
};

/* ============ 管理窗口（常规主窗口） ============ */
const MainContent: React.FC = () => {
  const [snippets, setSnippets] = useState<PersistedSnippet[]>([]);
  const [loading, setLoading] = useState(true);
  const [settingsRevision, setSettingsRevision] = useState(1);
  const settingsRevisionRef = useRef(1);
  const [config, setConfig] = useState<SettingsConfig>(DEFAULT_CONFIG);

  // SPEC-06: 从后端加载设置
  useEffect(() => {
    settingsApi.get().then(res => {
      setConfig(prev => ({ ...prev, ...res.settings }));
      settingsRevisionRef.current = res.revision;
      setSettingsRevision(res.revision);
    }).catch(() => {
      // 后端失败时保留默认值
    });
  }, []);

  // SPEC-06 / RF3: 快捷键先注册，成功后再提交 Settings；任一步失败保留旧值。
  const updateConfig = useCallback((key: string, value: unknown) => {
    const previous = config;
    const revision = settingsRevisionRef.current;

    void (async () => {
      try {
        if (key === 'globalShortcut') {
          await windowApi.registerShortcut(String(value));
        }
        await settingsApi.update(key, value, revision);
        setConfig(prev => ({ ...prev, [key]: value } as SettingsConfig));
        settingsRevisionRef.current = revision + 1;
        setSettingsRevision(revision + 1);
      } catch (error) {
        if (key === 'globalShortcut' && previous.globalShortcut !== value) {
          await windowApi.registerShortcut(previous.globalShortcut).catch(() => {});
        }
        setConfig(previous);
        showToast(`设置保存失败：${errorMessage(error)}`, 'error');
      }
    })();
  }, [config]);

  const [currentView, setCurrentView] = useState<ViewMode>('manager');
  const [isKeyboardHelpOpen, setIsKeyboardHelpOpen] = useState(false);
  const [toast, setToast] = useState<{ message: string; type: 'success' | 'error' } | null>(null);
  const [editRequestId, setEditRequestId] = useState<string | null>(null);
  const [prefillCreateKey, setPrefillCreateKey] = useState<string | undefined>();

  const { effectiveTheme } = useTheme();

  const showToast = (message: string, type: 'success' | 'error' = 'success') => {
    setToast({ message, type });
    window.setTimeout(() => setToast(null), 3000);
  };

  const refresh = useCallback(async () => {
    try {
      const items = await snippetsApi.list();
      setSnippets(items);
    } catch (e) {
      showToast(`读取片段失败：${errorMessage(e)}`, 'error');
    }
  }, []);

  // 初始加载真实数据（不使用 mock）
  useEffect(() => {
    refresh().finally(() => setLoading(false));
  }, [refresh]);

  // 消费检索窗口跳转请求（Ctrl+E 编辑 / Ctrl+N 预填新建）
  const consumeManagerRequest = useCallback(async () => {
    try {
      const request = await windowApi.takeManagerRequest();
      if (!request) return;
      applyManagerRequest(request);
    } catch {
      // 窗口请求消费失败不影响主流程
    }
  }, []);

  const applyManagerRequest = (request: ManagerRequest) => {
    if (request.prefillKey) {
      setPrefillCreateKey(request.prefillKey);
      setEditRequestId(null);
      setCurrentView('manager');
    } else if (request.editId) {
      setEditRequestId(request.editId);
      setPrefillCreateKey(undefined);
      setCurrentView('manager');
    }
  };

  useEffect(() => {
    consumeManagerRequest();
  }, [consumeManagerRequest]);

  // 窗口重新聚焦时再次消费（检索窗口已在显示状态下发出请求的场景）
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    try {
      getCurrentWindow()
        .onFocusChanged(({ payload }) => {
          if (payload) consumeManagerRequest();
        })
        .then(fn => { unlisten = fn; })
        .catch(() => {});
    } catch {
      // 非 Tauri 环境
    }
    return () => unlisten?.();
  }, [consumeManagerRequest]);

  // 预填/编辑请求由 ManagerWindow 消费后回调清除
  const handleRequestHandled = useCallback(() => {
    setEditRequestId(null);
    setPrefillCreateKey(undefined);
  }, []);

  const fieldsFrom = (snippet: PersistedSnippet) => ({
    key: snippet.key,
    title: snippet.title,
    content: snippet.content,
    aliases: snippet.aliases,
    tags: snippet.tags,
    sensitive: snippet.sensitive,
    pinned: snippet.pinned,
  });

  // ManagerWindow 以构造的 Snippet 调用；此处映射到真实 create/update（带 revision）
  const handleSaveSnippet = async (snippet: PersistedSnippet) => {
    const existing = snippets.find(s => s.id === snippet.id);
    try {
      if (existing) {
        await snippetsApi.update(existing, fieldsFrom(snippet));
        showToast(`片段 "${snippet.title}" 已保存`);
      } else {
        await snippetsApi.create(fieldsFrom(snippet), crypto.randomUUID());
        showToast(`片段 "${snippet.title}" 已创建`);
      }
      await refresh();
    } catch (e) {
      const err = e as AppError;
      if (err.code === 'REVISION_CONFLICT') {
        showToast('片段已被其他窗口修改，已重新载入最新数据。请重新编辑。', 'error');
        await refresh();
      } else if (err.field === 'key' || err.code === 'KEY_CONFLICT') {
        showToast(err.message, 'error');
      } else {
        showToast(`保存失败：${err.message}`, 'error');
      }
    }
  };

  const handleCopySnippet = async (snippet: PersistedSnippet) => {
    try {
      const outcome = await snippetsApi.copy(snippet.id, crypto.randomUUID(), true);
      showToast(`已复制 "${snippet.key}" 到剪贴板${outcome.counted ? '' : '（重复操作）'}`);
      await refresh();
    } catch (e) {
      showToast(`复制失败：${errorMessage(e)}`, 'error');
    }
  };

  const handlePasteSnippet = async (snippet: PersistedSnippet) => {
    await handleCopySnippet(snippet);
  };

  // 以下功能由后续 Spec 提供：回收站（SPEC-05）、导入导出/重置（SPEC-07）
  const notYet = (feature: string) => () => {
    showToast(`${feature}将在后续版本提供（对应 Spec）`, 'error');
  };

  const handleOpenSearchWindow = () => {
    windowApi.openSearch().catch(() => showToast('无法打开检索窗口', 'error'));
  };

  // 本窗口焦点态 Alt+O 呼出检索窗口（PRD FR-PCK-01 语义）
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.altKey && e.code === 'KeyO') {
        e.preventDefault();
        handleOpenSearchWindow();
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, []);

  if (loading) {
    return (
      <Box className="h-screen w-full flex items-center justify-center ambient-glow-bg">
        <span className="text-sm text-theme-muted">正在打开加密数据库…</span>
      </Box>
    );
  }

  return (
    <AppShell
      title="Searchis"
      subtitle={`Arch Linux 本机文本片段工具 (${effectiveTheme === 'dark' ? '深色' : '浅色'}主题)`}
      nav={
        <HeaderBar
          currentView={currentView}
          onSelectView={setCurrentView}
          config={config}
          onUpdateConfig={updateConfig}
          snippetCount={snippets.filter(s => !s.deletedAt).length}
          onOpenKeyboardHelp={() => setIsKeyboardHelpOpen(true)}
          onOpenQuickPicker={handleOpenSearchWindow}
        />
      }
      actions={
        <Button
          variant="primary"
          size="sm"
          icon={<Zap className="w-3.5 h-3.5" />}
          onClick={handleOpenSearchWindow}
        >
          呼出快速窗口 <Kbd>Alt+O</Kbd>
        </Button>
      }
    >
      {/* Toast Notification */}
      {toast && (
        <div className="fixed bottom-6 right-6 z-50 animate-pop-in">
          <Box
            paddingX="lg"
            paddingY="md"
            radius="lg"
            shadow="elevated"
            background="elevated"
            border="all"
            className={`flex items-center gap-2 ${
              toast.type === 'success' ? 'status-success' : 'status-danger'
            }`}
          >
            {toast.type === 'success' ? (
              <CheckCircle2 className="w-4 h-4" />
            ) : (
              <AlertCircle className="w-4 h-4" />
            )}
            <span className="text-xs font-semibold">{toast.message}</span>
          </Box>
        </div>
      )}

      {/* Main View Renderer */}
      {currentView === 'manager' && (
        <ManagerWindow
          snippets={snippets}
          onSaveSnippet={handleSaveSnippet}
          onDeleteSnippet={notYet('回收站')}
          onRestoreSnippet={notYet('回收站')}
          onPermanentDeleteSnippet={notYet('回收站')}
          onCopySnippet={handleCopySnippet}
          onPasteSnippet={handlePasteSnippet}
          onOpenSettings={() => setCurrentView('settings')}
          editRequestId={editRequestId}
          prefillCreateKey={prefillCreateKey}
          onRequestHandled={handleRequestHandled}
        />
      )}

      {currentView === 'onboarding' && (
        <OnboardingWizard
          onComplete={() => setCurrentView('manager')}
          onCreateSnippet={(key, title, content) => {
            snippetsApi
              .create({ key, title, content, aliases: [], tags: ['向导新建'], sensitive: false, pinned: false }, crypto.randomUUID())
              .then(() => refresh())
              .then(() => showToast('片段已创建'))
              .catch(e => showToast(`创建失败：${errorMessage(e)}`, 'error'));
          }}
        />
      )}

      {currentView === 'settings' && (
        <SettingsModal
          config={config}
          onUpdateConfig={updateConfig}
          onExportData={notYet('数据导出')}
          onImportData={notYet('数据导入')}
          onResetSampleData={notYet('重置示例数据')}
          onClose={() => setCurrentView('manager')}
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
  return <ThemeProvider>{WINDOW_LABEL === 'search' ? <SearchWindowApp /> : <MainContent />}</ThemeProvider>;
};
