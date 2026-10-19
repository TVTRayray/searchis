import React, { useCallback, useMemo, useRef, useState } from 'react';
import {
  Check,
  Clock,
  Copy,
  Eye,
  EyeOff,
  FileText,
  Keyboard,
  Layers,
  Moon,
  RefreshCw,
  RotateCcw,
  Search,
  Settings,
  Shield,
  Star,
  Sun,
  Tag,
  Terminal,
  Trash2,
  Upload,
  X,
} from 'lucide-react';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { OpenWithCandidate, templatesApi } from '../api/snippets';
import { Group, GROUP_FILTER_PREFIX, Snippet, SidebarFilter, SnippetLanguage, SortOption, TypeFilter } from '../types/snippet';
import { SelectField } from './ui/SelectField';
import { CodeEditor } from './ui/CodeEditor';
import { GroupTree } from './ui/GroupTree';
import { GroupDialog } from './ui/GroupDialog';
import { PlusMenu } from './ui/PlusMenu';
import { NewTemplateDialog } from './ui/NewTemplateDialog';
import { FileTemplateDetail, TemplateFormFields } from './ui/FileTemplateDetail';
import { FileTypeIcon, fileCategory } from './ui/FileTypeIcon';
import { THEME_LABELS, useTheme } from './ThemeProvider';

interface ManagerWindowProps {
  snippets: Snippet[];
  groups: Group[];
  /** 返回错误文案；成功返回 null。 */
  onCreateGroup: (name: string, parentId: string | null) => Promise<string | null>;
  onUpdateGroup: (group: Group, name: string, parentId: string | null) => Promise<string | null>;
  onRequestDeleteGroup: (group: Group) => void;
  onSaveSnippet: (snippet: Snippet) => void;
  onDeleteSnippet: (id: string) => void;
  onRestoreSnippet: (id: string) => void;
  onPermanentDeleteSnippet: (id: string) => void;
  onCopySnippet: (snippet: Snippet) => void;
  onPasteSnippet: (snippet: Snippet) => void;
  onOpenSettings: () => void;
  onOpenKeyboardHelp: () => void;
  onToggleTheme: () => void;
  /** 检索窗口 Ctrl+E 跳转：编辑指定片段 */
  editRequestId?: string | null;
  /** 检索窗口 Ctrl+N 跳转：以该 Key 预填新建 */
  prefillCreateKey?: string;
  /** 请求消费完成后通知 App 清除 */
  onRequestHandled?: () => void;
  /** 菜单触发动作 (新建 / 编辑 / 复制) */
  triggerAction?: { type: string; timestamp: number } | null;
  /** 外部指定的初始分类 (如回收站) */
  initialFilter?: SidebarFilter;
  // === SPEC-22: 文件模板 ===
  openWithByExtension?: Record<string, string>;
  scanning?: boolean;
  /** 文件导入进行中。 */
  importing?: boolean;
  /** 最近一次扫描的摘要（如“新增 1 · 更新 0 · 缺失 0”），空表示尚未扫描。 */
  scanSummary?: string | null;
  onRescan?: () => void;
  onCreateBlank?: (kind: 'md' | 'xlsx', title: string, key: string, groupId: string | null) => Promise<string | null>;
  /** 选择文件对话框 → 导入到分组。 */
  onPickImport?: (groupId: string | null) => void;
  /** 拖入文件 → 导入到分组。 */
  onImportPaths?: (paths: string[], groupId: string | null) => void;
  onSaveTemplate?: (snippet: Snippet, fields: TemplateFormFields) => Promise<string | null>;
  onOpenTemplate?: (snippet: Snippet, desktopId?: string) => void;
  onLoadOpenWith?: (snippet: Snippet) => Promise<OpenWithCandidate[]>;
}

const LANGUAGE_OPTIONS: Array<{ value: SnippetLanguage; label: string }> = [
  { value: 'plain', label: '纯文本' },
  { value: 'bash', label: 'Bash' },
  { value: 'sql', label: 'SQL' },
  { value: 'yaml', label: 'YAML' },
  { value: 'json', label: 'JSON' },
  { value: 'toml', label: 'TOML' },
  { value: 'ini', label: 'INI' },
];

const TYPE_FILTER_OPTIONS: Array<[TypeFilter, string]> = [
  ['all', '全部'],
  ['text', '文本'],
  ['file', '文件'],
];

const sortOptions: Array<[SortOption, string]> = [
  ['updated', '更新时间'],
  ['usage', '使用频率'],
  ['alpha', '名称首字母'],
  ['key', 'Key 字母'],
];

/**
 * 撤销/重做支持 hook：为文本输入框提供可靠的 Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z 历史记录
 */
function useUndoableInput(initialValue: string = '') {
  const [history, setHistory] = useState<{
    past: string[];
    present: string;
    future: string[];
  }>({
    past: [],
    present: initialValue,
    future: [],
  });

  const setValue = useCallback((valOrFn: string | ((prev: string) => string)) => {
    setHistory(curr => {
      const nextVal = typeof valOrFn === 'function' ? valOrFn(curr.present) : valOrFn;
      if (nextVal === curr.present) return curr;
      return {
        past: [...curr.past.slice(-60), curr.present],
        present: nextVal,
        future: [],
      };
    });
  }, []);

  const resetValue = useCallback((newVal: string) => {
    setHistory({
      past: [],
      present: newVal,
      future: [],
    });
  }, []);

  const undo = useCallback(() => {
    setHistory(curr => {
      if (curr.past.length === 0) return curr;
      const previous = curr.past[curr.past.length - 1];
      const newPast = curr.past.slice(0, curr.past.length - 1);
      return {
        past: newPast,
        present: previous,
        future: [curr.present, ...curr.future],
      };
    });
  }, []);

  const redo = useCallback(() => {
    setHistory(curr => {
      if (curr.future.length === 0) return curr;
      const next = curr.future[0];
      const newFuture = curr.future.slice(1);
      return {
        past: [...curr.past, curr.present],
        present: next,
        future: newFuture,
      };
    });
  }, []);

  const onKeyDown = useCallback((e: React.KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && !e.altKey) {
      if (e.key === 'z' || e.key === 'Z') {
        if (e.shiftKey) {
          e.preventDefault();
          redo();
        } else {
          e.preventDefault();
          undo();
        }
      } else if (e.key === 'y' || e.key === 'Y') {
        e.preventDefault();
        redo();
      }
    }
  }, [undo, redo]);

  return [history.present, setValue, resetValue, onKeyDown] as const;
}

export const ManagerWindow: React.FC<ManagerWindowProps> = ({
  snippets,
  groups,
  onCreateGroup,
  onUpdateGroup,
  onRequestDeleteGroup,
  onSaveSnippet,
  onDeleteSnippet,
  onRestoreSnippet,
  onPermanentDeleteSnippet,
  onCopySnippet,
  onPasteSnippet,
  onOpenSettings,
  onOpenKeyboardHelp,
  onToggleTheme,
  editRequestId,
  prefillCreateKey,
  onRequestHandled,
  triggerAction,
  initialFilter,
  openWithByExtension,
  scanning = false,
  importing = false,
  scanSummary = null,
  onRescan,
  onCreateBlank,
  onPickImport,
  onImportPaths,
  onSaveTemplate,
  onOpenTemplate,
  onLoadOpenWith,
}) => {
  const [activeFilter, setActiveFilter] = useState<SidebarFilter>(initialFilter || 'all');
  const [searchQuery, setSearchQuery, resetSearchQuery, onSearchKeyDown] = useUndoableInput('');
  const [sortBy, setSortBy] = useState<SortOption>('updated');
  const [selectedSnippetId, setSelectedSnippetId] = useState<string | null>(snippets[0]?.id || null);

  const selectedSnippet = snippets.find(snippet => snippet.id === selectedSnippetId) || null;
  const [keyInput, setKeyInput, resetKeyInput, onKeyKeyDown] = useUndoableInput('');
  const [titleInput, setTitleInput, resetTitleInput, onTitleKeyDown] = useUndoableInput('');
  // 正文由 CodeMirror 自带撤销栈管理，这里只保存当前文本。
  const [contentInput, setContentInput] = useState('');
  const resetContentInput = setContentInput;
  // 编辑器按“已装载的文档”重挂载：与 contentInput 在同一批更新里递增，新实例首帧即为目标正文。
  const [editorSeq, setEditorSeq] = useState(0);
  const loadedEditorDocRef = useRef<string | null>(null);
  const [languageInput, setLanguageInput] = useState<SnippetLanguage>('plain');
  const [groupInput, setGroupInput] = useState<string>('');
  const [typeFilter, setTypeFilter] = useState<TypeFilter>('all');
  const [newTemplateKind, setNewTemplateKind] = useState<'md' | 'xlsx' | null>(null);
  const [dragActive, setDragActive] = useState(false);
  const [groupDialog, setGroupDialog] = useState<{ group: Group | null; parentId: string | null } | null>(null);
  const [aliasesInput, setAliasesInput, resetAliasesInput, onAliasesKeyDown] = useUndoableInput('');
  const [tagsInput, setTagsInput, resetTagsInput, onTagsKeyDown] = useUndoableInput('');
  const [isSensitive, setIsSensitive] = useState(false);
  const [isPinned, setIsPinned] = useState(false);
  const [isCreatingNew, setIsCreatingNew] = useState(false);
  const runTemplateAction = async (
    snippet: Snippet,
    action: (id: string) => Promise<string>,
    failure: string,
  ): Promise<{ ok: boolean; message: string }> => {
    try {
      return { ok: true, message: await action(snippet.id) };
    } catch (e) {
      return { ok: false, message: `${failure}：${(e as { message?: string })?.message ?? '未知错误'}` };
    }
  };
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [sensitiveRevealed, setSensitiveRevealed] = useState(false);
  const [keyError, setKeyError] = useState('');

  const keyInputRef = useRef<HTMLInputElement>(null);
  const activeFilterRef = useRef(activeFilter);
  activeFilterRef.current = activeFilter;
  const { theme, effectiveTheme } = useTheme();
  const themeLabel = theme === 'system'
    ? `${THEME_LABELS.system} · ${THEME_LABELS[effectiveTheme]}`
    : THEME_LABELS[theme];

  // 响应外部 initialFilter 变化（例如菜单切换到回收站）
  React.useEffect(() => {
    if (initialFilter) {
      setActiveFilter(initialFilter);
    }
  }, [initialFilter]);

  React.useEffect(() => {
    if (!selectedSnippet) return;
    if (loadedEditorDocRef.current !== selectedSnippet.id) {
      loadedEditorDocRef.current = selectedSnippet.id;
      setEditorSeq(seq => seq + 1);
    }
    resetKeyInput(selectedSnippet.key);
    resetTitleInput(selectedSnippet.title);
    resetContentInput(selectedSnippet.content);
    resetAliasesInput(selectedSnippet.aliases.join(', '));
    resetTagsInput(selectedSnippet.tags.join(', '));
    setLanguageInput(selectedSnippet.language ?? 'plain');
    setGroupInput(selectedSnippet.groupId ?? '');
    setIsSensitive(!!selectedSnippet.sensitive);
    setIsPinned(!!selectedSnippet.pinned);
    setIsCreatingNew(false);
    setSensitiveRevealed(false);
    setKeyError('');
  }, [selectedSnippetId, selectedSnippet, resetKeyInput, resetTitleInput, resetContentInput, resetAliasesInput, resetTagsInput]);

  const handleStartCreateNew = useCallback(() => {
    setSelectedSnippetId(null);
    setIsCreatingNew(true);
    loadedEditorDocRef.current = null;
    setEditorSeq(seq => seq + 1);
    resetKeyInput('');
    resetTitleInput('');
    resetContentInput('');
    resetAliasesInput('');
    resetTagsInput('');
    setLanguageInput('plain');
    // 在某个分组视图里新建，默认放进该分组。
    setGroupInput(activeFilterRef.current.startsWith(GROUP_FILTER_PREFIX) ? activeFilterRef.current.slice(GROUP_FILTER_PREFIX.length) : '');
    setIsSensitive(false);
    setIsPinned(false);
    setSensitiveRevealed(true);
    setKeyError('');
    setTimeout(() => keyInputRef.current?.focus(), 50);
  }, [resetKeyInput, resetTitleInput, resetContentInput, resetAliasesInput, resetTagsInput]);

  /** 当前所在分组视图的分组 id；其他视图为 null（未分组）。 */
  const currentGroupId = (): string | null => (
    activeFilterRef.current.startsWith(GROUP_FILTER_PREFIX)
      ? activeFilterRef.current.slice(GROUP_FILTER_PREFIX.length) || null
      : null
  );

  // 拖入文件到管理窗口：导入到当前分组。
  const importPathsRef = useRef(onImportPaths);
  importPathsRef.current = onImportPaths;
  React.useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    try {
      getCurrentWebview()
        .onDragDropEvent(event => {
          const payload = event.payload;
          if (payload.type === 'enter' || payload.type === 'over') setDragActive(true);
          else if (payload.type === 'leave') setDragActive(false);
          else if (payload.type === 'drop') {
            setDragActive(false);
            if (payload.paths.length > 0) importPathsRef.current?.(payload.paths, currentGroupId());
          }
        })
        .then(fn => { if (cancelled) fn(); else unlisten = fn; })
        .catch(() => {});
    } catch {
      // 非 Tauri 环境（纯浏览器预览）没有拖入事件
    }
    return () => { cancelled = true; unlisten?.(); };
  }, []);

  const handleCopy = useCallback((snippet: Snippet) => {
    onCopySnippet(snippet);
    setCopiedId(snippet.id);
    window.setTimeout(() => setCopiedId(null), 1500);
  }, [onCopySnippet]);

  // 处理全局/子级菜单动作触发 (新建 / 编辑 / 复制)
  React.useEffect(() => {
    if (!triggerAction) return;
    if (triggerAction.type === 'new-snippet') {
      handleStartCreateNew();
    } else if (triggerAction.type === 'edit-current') {
      if (selectedSnippet) {
        setIsCreatingNew(false);
        keyInputRef.current?.focus();
      }
    } else if (triggerAction.type === 'copy-current') {
      if (selectedSnippet) {
        handleCopy(selectedSnippet);
      }
    }
  }, [triggerAction, handleStartCreateNew, handleCopy, selectedSnippet]);

  React.useEffect(() => {
    if (!prefillCreateKey) return;
    handleStartCreateNew();
    resetKeyInput(prefillCreateKey);
    onRequestHandled?.();
  }, [prefillCreateKey, handleStartCreateNew, resetKeyInput, onRequestHandled]);

  React.useEffect(() => {
    if (!editRequestId) return;
    const target = snippets.find(snippet => snippet.id === editRequestId);
    if (!target) return;
    setSelectedSnippetId(target.id);
    setIsCreatingNew(false);
    onRequestHandled?.();
  }, [editRequestId, snippets, onRequestHandled]);

  React.useEffect(() => {
    if (!isCreatingNew && selectedSnippetId && !snippets.some(snippet => snippet.id === selectedSnippetId)) {
      setSelectedSnippetId(snippets[0]?.id ?? null);
    }
  }, [snippets, selectedSnippetId, isCreatingNew]);

  const ungroupedCount = useMemo(
    () => snippets.filter(snippet => !snippet.deletedAt && !snippet.groupId).length,
    [snippets],
  );
  const groupNames = useMemo(() => new Map(groups.map(group => [group.id, group.name])), [groups]);
  const groupOptions = useMemo(() => {
    const options = [{ value: '', label: '未分组' }];
    const walk = (parent: string | null) => {
      for (const group of groups) {
        if ((group.parentId ?? null) !== parent) continue;
        options.push({ value: group.id, label: `${'　'.repeat(group.depth - 1)}${group.name}` });
        walk(group.id);
      }
    };
    walk(null);
    return options;
  }, [groups]);

  const { allTags, tagCounts, pinnedCount, deletedCount } = useMemo(() => {
    const counts: Record<string, number> = {};
    let pinned = 0;
    let deleted = 0;

    snippets.forEach(snippet => {
      if (snippet.deletedAt) {
        deleted += 1;
        return;
      }
      if (snippet.pinned) pinned += 1;
      snippet.tags.forEach(tag => { counts[tag] = (counts[tag] || 0) + 1; });
    });

    return { allTags: Object.keys(counts).sort((a, b) => a.localeCompare(b)), tagCounts: counts, pinnedCount: pinned, deletedCount: deleted };
  }, [snippets]);

  const filteredSnippets = useMemo(() => snippets
    .filter(snippet => {
      if (activeFilter === 'trash') {
        if (!snippet.deletedAt) return false;
      } else {
        if (snippet.deletedAt) return false;
        if (activeFilter === 'pinned' && !snippet.pinned) return false;
        if (activeFilter === 'recent' && !snippet.lastUsedAt) return false;
        if (activeFilter.startsWith(GROUP_FILTER_PREFIX)) {
          if ((snippet.groupId ?? '') !== activeFilter.slice(GROUP_FILTER_PREFIX.length)) return false;
        } else if (activeFilter !== 'all' && activeFilter !== 'pinned' && activeFilter !== 'recent' && !snippet.tags.includes(activeFilter)) return false;
      }
      // 文件模板在 SPEC-22 之前不存在；“文件”筛选始终为空。
      const isFile = snippet.kind === 'file';
      if (typeFilter === 'file' && !isFile) return false;
      if (typeFilter === 'text' && isFile) return false;

      const query = searchQuery.trim().toLowerCase();
      if (!query) return true;
      return snippet.key.toLowerCase().includes(query)
        || snippet.title.toLowerCase().includes(query)
        || snippet.content.toLowerCase().includes(query)
        || snippet.aliases.some(alias => alias.toLowerCase().includes(query))
        || snippet.tags.some(tag => tag.toLowerCase().includes(query));
    })
    .sort((a, b) => {
      if (sortBy === 'updated') return new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime();
      if (sortBy === 'usage') return b.usageCount - a.usageCount;
      if (sortBy === 'alpha') return a.title.localeCompare(b.title);
      return a.key.localeCompare(b.key);
    }), [snippets, activeFilter, searchQuery, sortBy, typeFilter]);

  const handleSave = () => {
    const trimmedKey = keyInput.trim().toLowerCase();
    if (!trimmedKey) {
      setKeyError('Key 不能为空');
      return;
    }
    if (!/^[a-zA-Z0-9_-]+$/.test(trimmedKey)) {
      setKeyError('Key 只能包含字母、数字、- 和 _');
      return;
    }

    const duplicate = snippets.find(snippet => snippet.key === trimmedKey && snippet.id !== selectedSnippet?.id && !snippet.deletedAt);
    if (duplicate) {
      setKeyError(`Key "${trimmedKey}" 已被片段 "${duplicate.title}" 使用`);
      return;
    }

    const now = new Date().toISOString();
    const nextSnippet: Snippet = {
      id: selectedSnippet && !isCreatingNew ? selectedSnippet.id : `snip-${Date.now()}`,
      key: trimmedKey,
      normalizedKey: trimmedKey,
      title: titleInput.trim() || trimmedKey,
      content: contentInput,
      aliases: aliasesInput.split(',').map(value => value.trim()).filter(Boolean),
      tags: tagsInput.split(',').map(value => value.trim()).filter(Boolean),
      pinned: isPinned,
      sensitive: isSensitive,
      usageCount: selectedSnippet ? selectedSnippet.usageCount : 0,
      lastUsedAt: selectedSnippet ? selectedSnippet.lastUsedAt : null,
      createdAt: selectedSnippet ? selectedSnippet.createdAt : now,
      updatedAt: now,
      deletedAt: selectedSnippet ? selectedSnippet.deletedAt : null,
      revision: selectedSnippet ? selectedSnippet.revision : 0,
      groupId: groupInput || null,
      language: languageInput,
      kind: 'text',
      fileExt: null,
      fileSize: null,
      fileMtime: null,
      fileMissing: false,
      fileAutoImported: false,
    };

    onSaveSnippet(nextSnippet);
    setSelectedSnippetId(nextSnippet.id);
    setIsCreatingNew(false);
    setKeyError('');
  };

  const navItem = (
    filter: SidebarFilter,
    label: string,
    icon: React.ReactNode,
    count?: number,
    tone = '',
  ) => (
    <button
      type="button"
      className={`manager-side-item ${activeFilter === filter ? 'is-active' : ''}`}
      onClick={() => setActiveFilter(filter)}
      aria-current={activeFilter === filter ? 'page' : undefined}
    >
      <span className={`manager-side-icon ${tone}`}>{icon}</span>
      <span className="manager-side-label">{label}</span>
      {count !== undefined && <span className="manager-side-count">{count}</span>}
    </button>
  );

  const status = isCreatingNew ? 'new' : selectedSnippet?.deletedAt ? 'deleted' : 'saved';

  return (
    <div className="manager-workspace">
      <aside className="manager-sidebar" aria-label="片段分类">
        <div className="manager-sidebar-heading">分类目录</div>
        <div className="manager-side-scroll">
          {navItem('all', '全部片段', <Layers strokeWidth={1.5} className="size-4" aria-hidden />, snippets.filter(snippet => !snippet.deletedAt).length, 'accent')}
          {navItem('pinned', '固定片段', <Star strokeWidth={1.5} className="size-4" aria-hidden />, pinnedCount, 'warning')}
          {navItem('recent', '最近使用', <Clock strokeWidth={1.5} className="size-4" aria-hidden />)}
          <div className="manager-side-divider" />
          <GroupTree
            groups={groups}
            activeFilter={activeFilter}
            ungroupedCount={ungroupedCount}
            onSelect={setActiveFilter}
            onCreate={parentId => setGroupDialog({ group: null, parentId })}
            onEdit={group => setGroupDialog({ group, parentId: group.parentId })}
            onDelete={onRequestDeleteGroup}
          />
          <div className="manager-side-divider" />
          <details className="manager-tag-section" open>
            <summary className="manager-side-section-title">标签</summary>
            {allTags.length === 0 && <div className="manager-side-empty">暂无标签</div>}
            {allTags.map(tag => navItem(tag, tag, <Tag strokeWidth={1.5} className="size-3.5" aria-hidden />, tagCounts[tag]))}
          </details>
          <div className="manager-side-divider" />
          {navItem('trash', '回收站', <Trash2 strokeWidth={1.5} className="size-4" aria-hidden />, deletedCount || undefined, 'danger')}
        </div>
        <div className="manager-sidebar-footer">
          <button type="button" className="manager-icon-button" aria-label="键盘快捷键速查" title="键盘快捷键速查" onClick={onOpenKeyboardHelp}>
            <Keyboard strokeWidth={1.5} className="size-[18px]" aria-hidden />
          </button>
          <button
            type="button"
            className="manager-icon-button"
            aria-label={`切换主题（当前：${themeLabel}）`}
            title={`切换主题（当前：${themeLabel}）`}
            onClick={onToggleTheme}
          >
            {effectiveTheme === 'dark' ? <Sun strokeWidth={1.5} className="size-[18px]" aria-hidden /> : <Moon strokeWidth={1.5} className="size-[18px]" aria-hidden />}
          </button>
          <button type="button" className="manager-icon-button" aria-label="打开设置" title="打开设置" onClick={onOpenSettings}>
            <Settings strokeWidth={1.5} className="size-[18px]" aria-hidden />
          </button>
        </div>
      </aside>

      <section className="manager-list-pane" aria-label="片段列表">
        <div className="manager-list-toolbar">
          <div className="manager-list-search-row">
            <label className="manager-search-field">
              <Search strokeWidth={1.5} className="size-4" aria-hidden />
              <span className="sr-only">过滤片段</span>
              <input
                type="search"
                value={searchQuery}
                onChange={event => setSearchQuery(event.target.value)}
                onKeyDown={onSearchKeyDown}
                placeholder="过滤片段... (Ctrl+Z 撤销)"
                aria-label="过滤片段"
              />
              {searchQuery && (
                <button type="button" className="manager-search-clear" aria-label="清除过滤条件" onClick={() => resetSearchQuery('')}>
                  <X strokeWidth={1.5} className="size-3.5" aria-hidden />
                </button>
              )}
            </label>
            <PlusMenu
              onNewText={handleStartCreateNew}
              onNewMarkdown={() => setNewTemplateKind('md')}
              onNewXlsx={() => setNewTemplateKind('xlsx')}
              onImport={() => onPickImport?.(currentGroupId())}
            />
          </div>
          <div className="manager-type-filter" role="group" aria-label="类型筛选">
            {TYPE_FILTER_OPTIONS.map(([value, label]) => (
              <button
                key={value}
                type="button"
                className={`manager-type-chip ${typeFilter === value ? 'is-active' : ''}`}
                aria-pressed={typeFilter === value}
                onClick={() => setTypeFilter(value)}
              >
                {label}
              </button>
            ))}
          </div>
          {importing && (
            <div className="manager-scan-summary" role="status" data-testid="import-progress">导入中…</div>
          )}
          {scanSummary && !scanning && (
            <div className="manager-scan-summary" role="status" data-testid="scan-summary">{scanSummary}</div>
          )}
          <div className="manager-list-meta">
            <span>{activeFilter === 'trash' ? '回收站' : `${filteredSnippets.length} 条结果`}</span>
            <button
              type="button"
              className="manager-text-action manager-rescan"
              onClick={onRescan}
              disabled={scanning || !onRescan}
              title="重新扫描模板目录"
            >
              <RefreshCw strokeWidth={1.5} className={`size-3 ${scanning ? 'is-spinning' : ''}`} aria-hidden />
              {scanning ? '扫描中…' : '重新扫描'}
            </button>
            <div>
              <span className="sr-only">排序方式</span>
              <SelectField
                value={sortBy}
                options={sortOptions.map(([value, label]) => ({ value, label }))}
                onChange={value => setSortBy(value as SortOption)}
                ariaLabel="排序方式"
                className="manager-select"
              />
            </div>
          </div>
        </div>

        <div className="manager-snippet-scroll">
          {filteredSnippets.length === 0 ? (
            <div className="manager-empty-state">
              <div className="manager-empty-mark"><Layers strokeWidth={1.5} className="size-5" aria-hidden /></div>
              <strong>{typeFilter === 'file' ? '暂无文件模板' : '暂无匹配的条目'}</strong>
              <span>{activeFilter === 'trash' ? '回收站为空' : typeFilter === 'file' ? '通过“+”菜单导入文件，或把文件拖进窗口。' : '从一个简短、可重复使用的 Key 开始。'}</span>
              {activeFilter !== 'trash' && typeFilter !== 'file' && <button type="button" className="manager-text-action" onClick={handleStartCreateNew}>新建第一条片段</button>}
            </div>
          ) : (
            <div className="manager-snippet-list" role="listbox" aria-label="文本片段">
              {filteredSnippets.map(snippet => {
                const isSelected = snippet.id === selectedSnippetId;
                return (
                  <button
                    key={snippet.id}
                    type="button"
                    role="option"
                    aria-selected={isSelected}
                    className={`manager-snippet-row ${isSelected ? 'is-selected' : ''}`}
                    onClick={() => {
                      setSelectedSnippetId(snippet.id);
                      setIsCreatingNew(false);
                    }}
                  >
                    <span className={`manager-snippet-mark ${snippet.kind === 'file' ? 'file-mark' : ''}`} data-category={snippet.kind === 'file' ? fileCategory(snippet.fileExt) : undefined} aria-hidden>
                      {snippet.kind === 'file'
                        ? <FileTypeIcon ext={snippet.fileExt} />
                        : snippet.pinned
                        ? <Star strokeWidth={1.5} className="size-3.5" fill="currentColor" />
                        : snippet.language && snippet.language !== 'plain'
                          ? <Terminal strokeWidth={1.5} className="size-3.5" />
                          : <FileText strokeWidth={1.5} className="size-3.5" />}
                    </span>
                    <span className="manager-snippet-copy">
                      <span className="manager-snippet-title">{snippet.title}</span>
                      <span className="manager-snippet-subline">
                        <span className="manager-snippet-key">{snippet.key}</span>
                        {snippet.kind === 'file' && snippet.fileExt && (
                          <span className="manager-language-badge manager-file-badge" data-testid="file-badge">{snippet.fileExt.toUpperCase()}</span>
                        )}
                        {snippet.kind === 'file' && snippet.fileMissing && (
                          <span className="manager-missing-badge" data-testid="missing-badge">文件缺失</span>
                        )}
                        {snippet.kind !== 'file' && snippet.language && snippet.language !== 'plain' && (
                          <span className="manager-language-badge" data-testid="language-badge">{snippet.language.toUpperCase()}</span>
                        )}
                        {snippet.groupId && groupNames.get(snippet.groupId) && (
                          <span className="manager-snippet-group">{groupNames.get(snippet.groupId)}</span>
                        )}
                        <span>更新于 {new Date(snippet.updatedAt).toLocaleDateString()}</span>
                      </span>
                    </span>
                    {snippet.sensitive && <span className="manager-sensitive-dot" aria-label="敏感内容" />}
                  </button>
                );
              })}
            </div>
          )}
        </div>
      </section>

      <main className="manager-editor" aria-label={isCreatingNew ? '创建片段' : selectedSnippet?.kind === 'file' ? '文件模板' : '编辑片段'}>
        {selectedSnippet?.kind === 'file' && !isCreatingNew ? (
          <FileTemplateDetail
            key={selectedSnippet.id}
            snippet={selectedSnippet}
            groupOptions={groupOptions}
            rememberedDesktopId={openWithByExtension?.[selectedSnippet.fileExt ?? '']}
            onSave={fields => onSaveTemplate?.(selectedSnippet, fields) ?? Promise.resolve('当前版本无法保存文件模板。')}
            onOpen={desktopId => onOpenTemplate?.(selectedSnippet, desktopId)}
            onExport={() => runTemplateAction(selectedSnippet, async id => {
              const outcome = await templatesApi.export(id, crypto.randomUUID());
              return outcome.path ? `已导出到 ${outcome.path}` : '已导出';
            }, '导出失败')}
            onFormatCopy={() => runTemplateAction(selectedSnippet, async id => {
              // 管理窗口里只写剪贴板，不自动粘贴到其他窗口。
              const outcome = await templatesApi.formatCopy(id, crypto.randomUUID(), false);
              return outcome.notes?.length ? `已带格式复制（${outcome.notes.join('；')}）` : '已带格式复制';
            }, '带格式复制失败')}
            onLoadOpenWith={() => onLoadOpenWith?.(selectedSnippet) ?? Promise.resolve([])}
            onTrash={() => onDeleteSnippet(selectedSnippet.id)}
            onRestore={() => onRestoreSnippet(selectedSnippet.id)}
            onPermanentDelete={() => onPermanentDeleteSnippet(selectedSnippet.id)}
          />
        ) : selectedSnippet || isCreatingNew ? (
          <div className="manager-editor-inner">
            <div className="manager-editor-toolbar">
              <div className="manager-editor-title-group">
                <span className={`manager-status-dot ${status}`} aria-hidden />
                <div>
                  <h1>{isCreatingNew ? '创建新文本片段' : selectedSnippet?.title}</h1>
                  <span>{isCreatingNew ? '新建模式' : selectedSnippet?.deletedAt ? '已删除' : '已加密保存'}</span>
                </div>
              </div>
              <div className="manager-editor-actions">
                {selectedSnippet && !isCreatingNew && (
                  <>
                    <button type="button" className="manager-button manager-button-secondary" onClick={() => handleCopy(selectedSnippet)}>
                      {copiedId === selectedSnippet.id ? <Check strokeWidth={1.5} className="size-3.5" aria-hidden /> : <Copy strokeWidth={1.5} className="size-3.5" aria-hidden />}
                      {copiedId === selectedSnippet.id ? '已复制' : '仅复制'}
                    </button>
                    <button type="button" className="manager-button manager-button-primary" onClick={() => onPasteSnippet(selectedSnippet)}>快速粘贴</button>
                  </>
                )}
                {selectedSnippet?.deletedAt ? (
                  <>
                    <button type="button" className="manager-button manager-button-secondary" onClick={() => onRestoreSnippet(selectedSnippet.id)}>
                      <RotateCcw strokeWidth={1.5} className="size-3.5" aria-hidden />恢复
                    </button>
                    <button type="button" className="manager-button manager-button-danger" onClick={() => onPermanentDeleteSnippet(selectedSnippet.id)}>
                      <Trash2 strokeWidth={1.5} className="size-3.5" aria-hidden />彻底删除
                    </button>
                  </>
                ) : selectedSnippet && !isCreatingNew ? (
                  <button type="button" className="manager-icon-button manager-danger-icon" aria-label="放入回收站" title="放入回收站" onClick={() => onDeleteSnippet(selectedSnippet.id)}>
                    <Trash2 strokeWidth={1.5} className="size-4" aria-hidden />
                  </button>
                ) : null}
              </div>
            </div>

            <div className="manager-form-card">
              <section className="manager-form-section">
                <div className="manager-section-heading">
                  <h2>基本信息</h2>
                  <span>用 Key 找到它，用标题认出它。</span>
                </div>
                <div className="manager-form-grid">
                  <label className="manager-field">
                    <span>Key <small>唯一快速检索标识 (支持 Ctrl+Z)</small></span>
                    <input
                      ref={keyInputRef}
                      type="text"
                      value={keyInput}
                      onChange={event => { setKeyInput(event.target.value); setKeyError(''); }}
                      onKeyDown={onKeyKeyDown}
                      placeholder="例如 email-work, addr-office"
                      className={`manager-control manager-mono ${keyError ? 'has-error' : ''}`}
                      aria-invalid={!!keyError}
                    />
                    {keyError && <em className="manager-field-error">{keyError}</em>}
                  </label>
                  <label className="manager-field">
                    <span>标题 <small>展示名称 (支持 Ctrl+Z)</small></span>
                    <input
                      type="text"
                      value={titleInput}
                      onChange={event => setTitleInput(event.target.value)}
                      onKeyDown={onTitleKeyDown}
                      placeholder="例如 工作邮箱, 公司地址"
                      className="manager-control"
                    />
                  </label>
                </div>
              </section>

              <section className="manager-form-section">
                <div className="manager-section-heading manager-content-heading">
                  <div><h2>文本内容</h2></div>
                  <div className="manager-content-tools">
                    {isSensitive && (
                      <button type="button" className="manager-inline-action" onClick={() => setSensitiveRevealed(value => !value)}>
                        {sensitiveRevealed ? <EyeOff strokeWidth={1.5} className="size-3.5" aria-hidden /> : <Eye strokeWidth={1.5} className="size-3.5" aria-hidden />}
                        {sensitiveRevealed ? '隐蔽' : '显示明文'}
                      </button>
                    )}
                    <span className="manager-character-count">{contentInput.length} 字符</span>
                  </div>
                </div>
                {isSensitive && !sensitiveRevealed ? (
                  <button type="button" className="manager-sensitive-panel" onClick={() => setSensitiveRevealed(true)} aria-label="显示敏感文本正文">
                    <Shield strokeWidth={1.5} className="size-6" aria-hidden />
                    <strong>敏感文本正文默认已安全隐蔽</strong>
                    <span>点击此处显示明文正文</span>
                  </button>
                ) : (
                  <CodeEditor
                    key={`editor-${editorSeq}`}
                    value={contentInput}
                    language={languageInput}
                    onChange={setContentInput}
                    placeholder="在此输入需要快速粘贴的任意文本片段..."
                    ariaLabel="文本内容"
                    className="manager-control manager-code-editor"
                  />
                )}
              </section>

              <section className="manager-form-section">
                <div className="manager-section-heading"><h2>元信息</h2><span>别名和标签帮助你更快找到片段。</span></div>
                <div className="manager-form-grid">
                  <div className="manager-field">
                    <span>分组 <small>最多 3 层</small></span>
                    <SelectField value={groupInput} options={groupOptions} onChange={setGroupInput} ariaLabel="分组" />
                  </div>
                  <div className="manager-field">
                    <span>语言 <small>只影响高亮与徽标</small></span>
                    <SelectField
                      value={languageInput}
                      options={LANGUAGE_OPTIONS}
                      onChange={value => setLanguageInput(value as SnippetLanguage)}
                      ariaLabel="语言"
                    />
                  </div>
                  <label className="manager-field">
                    <span>别名 <small>英文逗号分隔 (支持 Ctrl+Z)</small></span>
                    <input
                      type="text"
                      value={aliasesInput}
                      onChange={event => setAliasesInput(event.target.value)}
                      onKeyDown={onAliasesKeyDown}
                      placeholder="mail, workmail, 邮箱"
                      className="manager-control"
                    />
                  </label>
                  <label className="manager-field">
                    <span>标签 <small>英文逗号分隔 (支持 Ctrl+Z)</small></span>
                    <input
                      type="text"
                      value={tagsInput}
                      onChange={event => setTagsInput(event.target.value)}
                      onKeyDown={onTagsKeyDown}
                      placeholder="常用, 公司, 开发"
                      className="manager-control"
                    />
                  </label>
                </div>
              </section>

              <section className="manager-form-section">
                <div className="manager-section-heading"><h2>行为</h2><span>控制片段在检索和编辑时的表现。</span></div>
                <div className="manager-toggle-grid">
                  <label className="manager-toggle-row">
                    <span><strong>敏感内容防护</strong><small>开启后默认隐藏正文内容</small></span>
                    <input type="checkbox" className="manager-switch" checked={isSensitive} onChange={event => setIsSensitive(event.target.checked)} />
                  </label>
                  <label className="manager-toggle-row">
                    <span><strong>固定到顶部</strong><small>在检索列表中优先靠前显示</small></span>
                    <input type="checkbox" className="manager-switch" checked={isPinned} onChange={event => setIsPinned(event.target.checked)} />
                  </label>
                </div>
              </section>
            </div>

            <div className="manager-save-bar">
              <span>保存后会写入本地加密数据库。</span>
              <button type="button" className={`manager-button ${selectedSnippet && !isCreatingNew ? 'manager-button-secondary' : 'manager-button-primary'} manager-save-button`} onClick={handleSave}>
                保存片段数据
              </button>
            </div>
          </div>
        ) : (
          <div className="manager-editor-empty">
            <div className="manager-empty-mark manager-empty-mark-large">
              <Layers strokeWidth={1.5} className="size-6" aria-hidden />
            </div>
            <strong>未选择任何片段</strong>
            <span>在左侧选择一个片段查看详情，或直接新建。</span>
            <button type="button" className="manager-button manager-button-primary" onClick={handleStartCreateNew}>
              新建片段
            </button>
          </div>
        )}
      </main>

      {dragActive && (
        <div className="manager-drop-overlay" role="status" data-testid="drop-overlay">
          <Upload strokeWidth={1.5} className="size-6" aria-hidden />
          <strong>松开以导入文件</strong>
          <span>
            {(() => {
              const id = currentGroupId();
              return id && groupNames.get(id) ? `导入到分组「${groupNames.get(id)}」` : '导入到未分组';
            })()}
            ；单个文件不超过 20 MB。
          </span>
        </div>
      )}

      {newTemplateKind && (
        <NewTemplateDialog
          kind={newTemplateKind}
          onClose={() => setNewTemplateKind(null)}
          onSubmit={async (title, key) => {
            const message = await (onCreateBlank?.(newTemplateKind, title, key, currentGroupId()) ?? Promise.resolve('当前版本无法新建文件模板。'));
            if (!message) setNewTemplateKind(null);
            return message;
          }}
        />
      )}

      {groupDialog && (
        <GroupDialog
          group={groupDialog.group}
          initialParentId={groupDialog.parentId}
          groups={groups}
          onClose={() => setGroupDialog(null)}
          onSubmit={async (name, parentId) => {
            const message = groupDialog.group
              ? await onUpdateGroup(groupDialog.group, name, parentId)
              : await onCreateGroup(name, parentId);
            if (!message) setGroupDialog(null);
            return message;
          }}
        />
      )}
    </div>
  );
};
