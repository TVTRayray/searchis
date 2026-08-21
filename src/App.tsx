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
import { ThemeProvider, useTheme } from './components/ThemeProvider';
import { HeaderBar } from './components/HeaderBar';
import { QuickSearchWindow } from './components/QuickSearchWindow';
import { ManagerWindow } from './components/ManagerWindow';
import { SettingsModal } from './components/SettingsModal';
import { KeyboardShortcutsModal } from './components/KeyboardShortcutsModal';
import { CheckCircle2, AlertCircle } from 'lucide-react';

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
  const { setTheme } = useTheme();

  // SPEC-06: 从后端加载设置；主窗口以持久化后端主题覆盖旧的本地初始值。
  useEffect(() => {
    settingsApi.get().then(res => {
      setConfig(prev => ({ ...prev, ...res.settings }));
      setTheme(res.settings.theme as 'light' | 'dark' | 'system');
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
  const [pendingPermanentDeleteId, setPendingPermanentDeleteId] = useState<string | null>(null);
  const [editRequestId, setEditRequestId] = useState<string | null>(null);
  const [prefillCreateKey, setPrefillCreateKey] = useState<string | undefined>();

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

  const handleDeleteSnippet = async (id: string) => {
    const target = snippets.find(snippet => snippet.id === id);
    if (!target) return;
    try {
      await snippetsApi.trashMove(id);
      showToast(`片段 "${target.key}" 已移入回收站`);
      await refresh();
    } catch (error) {
      showToast(`移入回收站失败：${errorMessage(error)}`, 'error');
    }
  };

  const handleRestoreSnippet = async (id: string) => {
    const target = snippets.find(snippet => snippet.id === id);
    if (!target) return;
    try {
      await snippetsApi.trashRestore(id);
      showToast(`片段 "${target.key}" 已恢复`);
      await refresh();
    } catch (error) {
      showToast(`恢复片段失败：${errorMessage(error)}`, 'error');
    }
  };

  const handlePermanentDeleteSnippet = (id: string) => {
    if (snippets.some(snippet => snippet.id === id && snippet.deletedAt)) {
      setPendingPermanentDeleteId(id);
    }
  };

  const confirmPermanentDelete = async () => {
    const id = pendingPermanentDeleteId;
    const target = snippets.find(snippet => snippet.id === id);
    setPendingPermanentDeleteId(null);
    if (!id || !target) return;
    try {
      await snippetsApi.trashPurgeOne(id, `purge_${id}_${Date.now()}`);
      showToast(`片段 "${target.key}" 已永久删除`);
      await refresh();
    } catch (error) {
      showToast(`永久删除失败：${errorMessage(error)}`, 'error');
    }
  };

  // 导入导出/重置仍由后续 UI slice 接入既有 Rust 命令。
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
      <div className="main-window-shell main-window-loading">
        <span>正在打开加密数据库…</span>
      </div>
    );
  }

  return (
    <div className="main-window-shell">
      <HeaderBar
        currentView={currentView}
        onSelectView={setCurrentView}
        config={config}
        onUpdateConfig={updateConfig}
        onOpenKeyboardHelp={() => setIsKeyboardHelpOpen(true)}
      />

      <main className="main-window-content">
        {toast && (
          <div className="main-toast" role="status" aria-live="polite">
            {toast.type === 'success' ? <CheckCircle2 className="size-4" aria-hidden /> : <AlertCircle className="size-4" aria-hidden />}
            <span>{toast.message}</span>
          </div>
        )}

        {pendingPermanentDeleteId && (
          <div className="manager-confirm-backdrop" role="presentation">
            <section className="manager-confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby="permanent-delete-title" aria-describedby="permanent-delete-description">
              <h2 id="permanent-delete-title">确认永久删除？</h2>
              <p id="permanent-delete-description">此操作不可恢复，片段将从加密数据库中永久移除。</p>
              <div className="manager-confirm-actions">
                <button type="button" className="manager-button manager-button-secondary" onClick={() => setPendingPermanentDeleteId(null)}>取消</button>
                <button type="button" className="manager-button manager-button-danger" onClick={confirmPermanentDelete}>确认永久删除</button>
              </div>
            </section>
          </div>
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
            editRequestId={editRequestId}
            prefillCreateKey={prefillCreateKey}
            onRequestHandled={handleRequestHandled}
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
      </main>

      <KeyboardShortcutsModal
        isOpen={isKeyboardHelpOpen}
        onClose={() => setIsKeyboardHelpOpen(false)}
      />
    </div>
  );
};

export const App: React.FC = () => {
  return <ThemeProvider>{WINDOW_LABEL === 'search' ? <SearchWindowApp /> : <MainContent />}</ThemeProvider>;
};
