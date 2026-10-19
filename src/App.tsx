import React, { useState, useEffect, useCallback, useRef } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { SidebarFilter, ViewMode } from './types/snippet';
import { SettingsConfig } from './api/snippets';
import {
  AppError,
  Group,
  groupsApi,
  ManagerRequest,
  OpenWithCandidate,
  PersistedSnippet,
  ScanResult,
  SearchResultItem,
  settingsApi,
  snippetsApi,
  templatesApi,
  windowApi,
} from './api/snippets';
import { TemplateFormFields } from './components/ui/FileTemplateDetail';
import { ThemeProvider, useTheme } from './components/ThemeProvider';
import { QuickSearchWindow } from './components/QuickSearchWindow';
import { ManagerWindow } from './components/ManagerWindow';
import { SettingsSection, SettingsView } from './components/SettingsView';
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
  openWithByExtension: {},
};

const errorMessage = (e: unknown): string =>
  (e as AppError)?.message ?? (e as { message?: string })?.message ?? '操作失败';

/** 文件模板错误：带上错误码，方便对照文档与日志。 */
const templateErrorText = (e: unknown): string => {
  const err = e as AppError;
  return err?.code?.startsWith('TEMPLATE_') ? `${errorMessage(e)}（${err.code}）` : errorMessage(e);
};

const SCAN_SKIP_TEXT: Record<string, string> = {
  symlink: '符号链接',
  too_large: '超过 20 MB',
  depth_exceeded: '目录超过 3 层',
  invalid_name: '名称不合法',
  group_conflict: '分组重名',
  limit: '超过 10,000 条上限',
};

const summarizeScan = (result: ScanResult): string => {
  const parts = [`新增 ${result.added}`, `更新 ${result.updated}`, `缺失 ${result.missing}`];
  if (result.restored > 0) parts.push(`恢复 ${result.restored}`);
  const skipped = new Map<string, number>();
  result.skipped.forEach(item => skipped.set(item.reason, (skipped.get(item.reason) ?? 0) + 1));
  skipped.forEach((count, reason) => parts.push(`跳过 ${count}（${SCAN_SKIP_TEXT[reason] ?? reason}）`));
  if (result.failedBatches > 0) parts.push(`${result.failedBatches} 批失败，稍后重试`);
  return parts.join(' · ');
};

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

  const handleOpenManager = useCallback(() => {
    windowApi
      .closeSearch()
      .then(() => windowApi.openManager())
      .catch(() => {});
  }, []);

  return (
    <QuickSearchWindow
      onClose={handleClose}
      onEditSnippet={handleEdit}
      onCreateNewSnippet={handleCreate}
      onOpenManager={handleOpenManager}
      maxResultsCount={maxResults}
    />
  );
};

/* ============ 管理窗口（常规主窗口） ============ */
const MainContent: React.FC = () => {
  const [snippets, setSnippets] = useState<PersistedSnippet[]>([]);
  const [groups, setGroups] = useState<Group[]>([]);
  const [pendingGroupDelete, setPendingGroupDelete] = useState<{
    group: Group;
    snippetCount: number;
    childGroupCount: number;
  } | null>(null);
  const [loading, setLoading] = useState(true);
  const [settingsRevision, setSettingsRevision] = useState(1);
  const settingsRevisionRef = useRef(1);
  const [config, setConfig] = useState<SettingsConfig>(DEFAULT_CONFIG);
  const { setTheme, effectiveTheme } = useTheme();

  // SPEC-06: 从后端加载设置；主窗口以持久化后端主题覆盖旧的本地初始值。
  const reloadSettings = useCallback(() => (
    settingsApi.get().then(res => {
      setConfig(prev => ({ ...prev, ...res.settings }));
      setTheme(res.settings.theme as 'light' | 'dark' | 'system');
      settingsRevisionRef.current = res.revision;
      setSettingsRevision(res.revision);
    }).catch(() => {
      // 后端失败时保留默认值
    })
  ), [setTheme]);

  useEffect(() => {
    void reloadSettings();
  }, [reloadSettings]);

  // SPEC-06 / RF3: 快捷键先注册，成功后再提交 Settings；任一步失败保留旧值。
  const updateConfig = useCallback(async (key: string, value: unknown): Promise<string | null> => {
    const previous = config;
    try {
      if (key === 'globalShortcut') {
        await windowApi.registerShortcut(String(value)).catch(() => {});
      }
      try {
        await settingsApi.update(key, value, settingsRevisionRef.current);
      } catch (error) {
        // 设置在别处被改过（如打开方式记忆、自动化脚本）：取最新 revision 重试一次。
        if ((error as AppError)?.code !== 'REVISION_CONFLICT') throw error;
        const latest = await settingsApi.get();
        settingsRevisionRef.current = latest.revision;
        await settingsApi.update(key, value, latest.revision);
      }
      const nextRevision = settingsRevisionRef.current + 1;
      setConfig(prev => ({ ...prev, [key]: value } as SettingsConfig));
      settingsRevisionRef.current = nextRevision;
      setSettingsRevision(nextRevision);
      return null;
    } catch (error) {
      setConfig(previous);
      showToast(`设置保存失败：${errorMessage(error)}`, 'error');
      return errorMessage(error);
    }
  }, [config]);

  const [currentView, setCurrentView] = useState<ViewMode>('manager');
  const [activeFilterFromMenu, setActiveFilterFromMenu] = useState<SidebarFilter | undefined>(undefined);
  const [menuTriggerAction, setMenuTriggerAction] = useState<{ type: string; timestamp: number } | null>(null);
  const [isKeyboardHelpOpen, setIsKeyboardHelpOpen] = useState(false);
  const [settingsTarget, setSettingsTarget] = useState<{ section: SettingsSection; nonce: number } | null>(null);
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
      const [items, groupItems] = await Promise.all([snippetsApi.list(), groupsApi.list()]);
      setSnippets(items);
      setGroups(groupItems);
    } catch (e) {
      showToast(`读取片段失败：${errorMessage(e)}`, 'error');
    }
  }, []);

  // === SPEC-22: 文件模板 ===
  const [scanning, setScanning] = useState(false);
  const [importing, setImporting] = useState(false);
  const [scanSummary, setScanSummary] = useState<string | null>(null);
  const scanningRef = useRef(false);
  const lastScanAtRef = useRef(0);

  /** 后台扫描模板目录；完成后显示变化计数，有变化时刷新列表。 */
  const runScan = useCallback(async (manual: boolean) => {
    if (scanningRef.current) return;
    scanningRef.current = true;
    setScanning(true);
    try {
      const result = await templatesApi.scan();
      lastScanAtRef.current = Date.now();
      setScanSummary(summarizeScan(result));
      if (manual || result.added + result.updated + result.missing + result.restored > 0) await refresh();
      if (manual) showToast(`扫描完成：${summarizeScan(result)}`);
    } catch (e) {
      setScanSummary(null);
      showToast(`扫描模板目录失败：${templateErrorText(e)}`, 'error');
    } finally {
      scanningRef.current = false;
      setScanning(false);
    }
  }, [refresh]);

  // 初始加载真实数据（不使用 mock），随后在后台扫描模板目录。
  useEffect(() => {
    refresh().finally(() => {
      setLoading(false);
      void runScan(false);
    });
  }, [refresh, runScan]);

  // 监听来自 KDE 原生菜单的全局事件分发
  useEffect(() => {
    let unlistenNavigate: (() => void) | undefined;
    let unlistenMenuAction: (() => void) | undefined;

    listen<string>('navigate', (event) => {
      const dest = event.payload;
      if (dest === 'settings') {
        setCurrentView('settings');
      } else if (dest === 'trash') {
        setCurrentView('manager');
        setActiveFilterFromMenu('trash');
      } else if (dest === 'manager') {
        setCurrentView('manager');
        setActiveFilterFromMenu('all');
      }
    }).then(fn => { unlistenNavigate = fn; }).catch(() => {});

    listen<string>('menu-action', (event) => {
      const action = event.payload;
      if (action === 'shortcuts') {
        setIsKeyboardHelpOpen(true);
      } else if (action === 'about') {
        // Global Menu“关于”：命令 ID 不变，改为打开设置视图的“关于”分节。
        setCurrentView('settings');
        setSettingsTarget({ section: 'about', nonce: Date.now() });
      } else if (action === 'new-snippet') {
        setCurrentView('manager');
        setMenuTriggerAction({ type: 'new-snippet', timestamp: Date.now() });
      } else if (action === 'edit-current') {
        setCurrentView('manager');
        setMenuTriggerAction({ type: 'edit-current', timestamp: Date.now() });
      } else if (action === 'copy-current') {
        setMenuTriggerAction({ type: 'copy-current', timestamp: Date.now() });
      } else if (action === 'import' || action === 'export') {
        setCurrentView('settings');
        setSettingsTarget({ section: 'backup', nonce: Date.now() });
      }
    }).then(fn => { unlistenMenuAction = fn; }).catch(() => {});

    return () => {
      unlistenNavigate?.();
      unlistenMenuAction?.();
    };
  }, []);

  // 消费检索窗口跳转请求（Ctrl+E 编辑 / Ctrl+N 预填新建）
  const runScanRef = useRef(runScan);
  runScanRef.current = runScan;
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
          if (payload) {
            consumeManagerRequest();
            // 管理窗口重新获得焦点时补一次扫描（距上次不足 10 秒则跳过）。
            if (Date.now() - lastScanAtRef.current > 10_000) void runScanRef.current(false);
          }
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
    groupId: snippet.groupId,
    language: snippet.language,
  });

  // 分组操作：错误以文案返回给对话框就地显示；成功后整体刷新（计数、副标题随之更新）。
  const groupErrorText = (error: unknown): string => {
    const err = error as AppError;
    switch (err?.code) {
      case 'GROUP_NAME_INVALID': return '分组名需为 1–64 个字符，不含 /，不能以 . 开头。';
      case 'GROUP_NAME_CONFLICT': return '同级已有同名分组。';
      case 'GROUP_DEPTH_EXCEEDED': return 'GROUP_DEPTH_EXCEEDED：分组最多 3 层。';
      case 'REVISION_CONFLICT': return '分组已在其他窗口修改，请关闭后重试。';
      default: return err?.message ?? '操作失败';
    }
  };

  const handleCreateGroup = async (name: string, parentId: string | null): Promise<string | null> => {
    try {
      await groupsApi.create(name, parentId, crypto.randomUUID());
      await refresh();
      return null;
    } catch (error) {
      if ((error as AppError).code === 'VALIDATION_FAILED') await refresh();
      return groupErrorText(error);
    }
  };

  const handleUpdateGroup = async (group: Group, name: string, parentId: string | null): Promise<string | null> => {
    try {
      await groupsApi.update(group, name, parentId);
      await refresh();
      return null;
    } catch (error) {
      if ((error as AppError).code === 'REVISION_CONFLICT') await refresh();
      return groupErrorText(error);
    }
  };

  const handleRequestDeleteGroup = async (group: Group) => {
    try {
      const preview = await groupsApi.deletePreview(group.id);
      setPendingGroupDelete({ group, ...preview });
    } catch (error) {
      showToast(`读取分组信息失败：${errorMessage(error)}`, 'error');
    }
  };

  const confirmDeleteGroup = async () => {
    const target = pendingGroupDelete;
    setPendingGroupDelete(null);
    if (!target) return;
    try {
      await groupsApi.delete(target.group);
      showToast('分组已删除，其中的内容已上移到上级');
    } catch (error) {
      showToast(`删除分组失败：${errorMessage(error)}`, 'error');
    }
    await refresh();
  };

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
      if ((error as AppError).code === 'TEMPLATE_FILE_MISSING') await handleTemplateFileMissing(error);
      else showToast(`移入回收站失败：${templateErrorText(error)}`, 'error');
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
      showToast(`恢复片段失败：${templateErrorText(error)}`, 'error');
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
      showToast(`永久删除失败：${templateErrorText(error)}`, 'error');
    }
  };

  const selectCreated = (snippet: PersistedSnippet) => {
    setPrefillCreateKey(undefined);
    setEditRequestId(snippet.id);
  };

  const importCodeText = (code: string): string => (
    code === 'TEMPLATE_TOO_LARGE' ? '超过 20 MB 或条目已满' : code
  );

  const handleImportPaths = async (paths: string[], groupId: string | null) => {
    if (paths.length === 0) return;
    setImporting(true);
    try {
      const outcome = await templatesApi.import(paths, groupId, crypto.randomUUID());
      await refresh();
      if (outcome.created.length > 0) selectCreated(outcome.created[0]);
      const rejectedCodes = [...new Set(outcome.rejected.map(item => importCodeText(item.code)))];
      const rejectedNote = outcome.rejected.length
        ? `，${outcome.rejected.length} 个未导入（${rejectedCodes.join('、')}）`
        : '';
      if (outcome.created.length > 0) {
        showToast(`已导入 ${outcome.created.length} 个文件${rejectedNote}；文件以明文保存在模板目录。`);
      } else {
        showToast(`没有文件被导入${rejectedNote}`, 'error');
      }
    } catch (e) {
      showToast(`导入失败：${templateErrorText(e)}`, 'error');
    } finally {
      setImporting(false);
    }
  };

  const handlePickImport = async (groupId: string | null) => {
    try {
      const paths = await templatesApi.pickFiles();
      await handleImportPaths(paths, groupId);
    } catch (e) {
      showToast(`无法打开文件选择器：${templateErrorText(e)}`, 'error');
    }
  };

  const handleCreateBlank = async (
    kind: 'md' | 'xlsx',
    title: string,
    key: string,
    groupId: string | null,
  ): Promise<string | null> => {
    try {
      const created = await templatesApi.createBlank(kind, title.trim(), key.trim(), groupId, crypto.randomUUID());
      await refresh();
      selectCreated(created);
      showToast(`已新建 ${kind === 'md' ? 'Markdown' : 'Excel'} 模板；文件以明文保存在模板目录`);
      return null;
    } catch (e) {
      const err = e as AppError;
      if (err.code === 'KEY_CONFLICT') return `Key "${err.conflictKey ?? key}" 已被使用。`;
      return templateErrorText(e);
    }
  };

  const handleSaveTemplate = async (snippet: PersistedSnippet, fields: TemplateFormFields): Promise<string | null> => {
    try {
      const saved = await templatesApi.update(snippet, fields);
      await refresh();
      showToast(saved.title !== fields.title ? '已保存，文件名已同步（重名已自动加序号）' : '模板信息已保存');
      return null;
    } catch (e) {
      const err = e as AppError;
      if (err.code === 'REVISION_CONFLICT') {
        await refresh();
        return '模板已被其他窗口修改，已重新载入最新数据，请重新编辑。';
      }
      if (err.code === 'KEY_CONFLICT') return `Key "${err.conflictKey ?? fields.key}" 已被使用。`;
      if (err.code === 'TEMPLATE_RECONCILE_PENDING') await refresh();
      return templateErrorText(e);
    }
  };

  /** 文件已被外部删除/改动时：提示并触发一次扫描。 */
  const handleTemplateFileMissing = async (e: unknown) => {
    showToast(templateErrorText(e), 'error');
    await runScan(false);
  };

  const handleOpenTemplate = async (snippet: PersistedSnippet, desktopId?: string) => {
    try {
      await templatesApi.open(snippet.id, desktopId);
      if (desktopId) await reloadSettings();
    } catch (e) {
      if ((e as AppError).code === 'TEMPLATE_FILE_MISSING') await handleTemplateFileMissing(e);
      else showToast(`打开失败：${templateErrorText(e)}`, 'error');
    }
  };

  const handleLoadOpenWith = async (snippet: PersistedSnippet): Promise<OpenWithCandidate[]> => {
    try {
      return await templatesApi.openWithCandidates(snippet.id);
    } catch (e) {
      if ((e as AppError).code === 'TEMPLATE_FILE_MISSING') await handleTemplateFileMissing(e);
      throw e;
    }
  };

  const toggleTheme = () => {
    const nextTheme = effectiveTheme === 'dark' ? 'light' : 'dark';
    setTheme(nextTheme);
    updateConfig('theme', nextTheme);
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

        {pendingGroupDelete && (
          <div className="manager-confirm-backdrop" role="presentation">
            <section className="manager-confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby="group-delete-title" aria-describedby="group-delete-description">
              <h2 id="group-delete-title">删除分组？</h2>
              <p id="group-delete-description">
                {pendingGroupDelete.snippetCount} 个条目和 {pendingGroupDelete.childGroupCount} 个子分组会上移到上级分组，
                与上级重名的子分组会自动加 (2)。条目本身不会被删除。
              </p>
              <div className="manager-confirm-actions">
                <button type="button" className="manager-button manager-button-secondary" onClick={() => setPendingGroupDelete(null)}>取消</button>
                <button type="button" className="manager-button manager-button-danger" onClick={confirmDeleteGroup}>确认删除分组</button>
              </div>
            </section>
          </div>
        )}

        {currentView === 'manager' && (
          <ManagerWindow
            snippets={snippets}
            groups={groups}
            onCreateGroup={handleCreateGroup}
            onUpdateGroup={handleUpdateGroup}
            onRequestDeleteGroup={handleRequestDeleteGroup}
            onSaveSnippet={handleSaveSnippet}
            onDeleteSnippet={handleDeleteSnippet}
            onRestoreSnippet={handleRestoreSnippet}
            onPermanentDeleteSnippet={handlePermanentDeleteSnippet}
            onCopySnippet={handleCopySnippet}
            onPasteSnippet={handlePasteSnippet}
            onOpenSettings={() => setCurrentView('settings')}
            onOpenKeyboardHelp={() => setIsKeyboardHelpOpen(true)}
            onToggleTheme={toggleTheme}
            editRequestId={editRequestId}
            prefillCreateKey={prefillCreateKey}
            onRequestHandled={handleRequestHandled}
            triggerAction={menuTriggerAction}
            initialFilter={activeFilterFromMenu}
            openWithByExtension={config.openWithByExtension}
            scanning={scanning}
            importing={importing}
            scanSummary={scanSummary}
            onRescan={() => void runScan(true)}
            onCreateBlank={handleCreateBlank}
            onPickImport={groupId => void handlePickImport(groupId)}
            onImportPaths={(paths, groupId) => void handleImportPaths(paths, groupId)}
            onSaveTemplate={handleSaveTemplate}
            onOpenTemplate={(snippet, desktopId) => void handleOpenTemplate(snippet, desktopId)}
            onLoadOpenWith={handleLoadOpenWith}
          />
        )}

        {currentView === 'settings' && (
          <SettingsView
            config={config}
            onUpdateConfig={updateConfig}
            target={settingsTarget}
            onClose={() => setCurrentView('manager')}
            onDataChanged={refresh}
            showToast={showToast}
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
