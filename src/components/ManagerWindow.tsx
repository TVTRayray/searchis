import React, { useCallback, useMemo, useRef, useState } from 'react';
import {
  Check,
  Clock,
  Copy,
  Eye,
  EyeOff,
  Layers,
  Plus,
  RotateCcw,
  Search,
  Settings,
  Shield,
  Star,
  Tag,
  Trash2,
  X,
} from 'lucide-react';
import { Snippet, SidebarFilter, SortOption } from '../types/snippet';
import { SelectField } from './ui/SelectField';

interface ManagerWindowProps {
  snippets: Snippet[];
  onSaveSnippet: (snippet: Snippet) => void;
  onDeleteSnippet: (id: string) => void;
  onRestoreSnippet: (id: string) => void;
  onPermanentDeleteSnippet: (id: string) => void;
  onCopySnippet: (snippet: Snippet) => void;
  onPasteSnippet: (snippet: Snippet) => void;
  onOpenSettings: () => void;
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
}

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
  onSaveSnippet,
  onDeleteSnippet,
  onRestoreSnippet,
  onPermanentDeleteSnippet,
  onCopySnippet,
  onPasteSnippet,
  onOpenSettings,
  editRequestId,
  prefillCreateKey,
  onRequestHandled,
  triggerAction,
  initialFilter,
}) => {
  const [activeFilter, setActiveFilter] = useState<SidebarFilter>(initialFilter || 'all');
  const [searchQuery, setSearchQuery, resetSearchQuery, onSearchKeyDown] = useUndoableInput('');
  const [sortBy, setSortBy] = useState<SortOption>('updated');
  const [selectedSnippetId, setSelectedSnippetId] = useState<string | null>(snippets[0]?.id || null);

  const selectedSnippet = snippets.find(snippet => snippet.id === selectedSnippetId) || null;
  const [keyInput, setKeyInput, resetKeyInput, onKeyKeyDown] = useUndoableInput('');
  const [titleInput, setTitleInput, resetTitleInput, onTitleKeyDown] = useUndoableInput('');
  const [contentInput, setContentInput, resetContentInput, onContentKeyDown] = useUndoableInput('');
  const [aliasesInput, setAliasesInput, resetAliasesInput, onAliasesKeyDown] = useUndoableInput('');
  const [tagsInput, setTagsInput, resetTagsInput, onTagsKeyDown] = useUndoableInput('');
  const [isSensitive, setIsSensitive] = useState(false);
  const [isPinned, setIsPinned] = useState(false);
  const [isCreatingNew, setIsCreatingNew] = useState(false);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [sensitiveRevealed, setSensitiveRevealed] = useState(false);
  const [keyError, setKeyError] = useState('');

  const keyInputRef = useRef<HTMLInputElement>(null);

  // 响应外部 initialFilter 变化（例如菜单切换到回收站）
  React.useEffect(() => {
    if (initialFilter) {
      setActiveFilter(initialFilter);
    }
  }, [initialFilter]);

  React.useEffect(() => {
    if (!selectedSnippet) return;
    resetKeyInput(selectedSnippet.key);
    resetTitleInput(selectedSnippet.title);
    resetContentInput(selectedSnippet.content);
    resetAliasesInput(selectedSnippet.aliases.join(', '));
    resetTagsInput(selectedSnippet.tags.join(', '));
    setIsSensitive(!!selectedSnippet.sensitive);
    setIsPinned(!!selectedSnippet.pinned);
    setIsCreatingNew(false);
    setSensitiveRevealed(false);
    setKeyError('');
  }, [selectedSnippetId, selectedSnippet, resetKeyInput, resetTitleInput, resetContentInput, resetAliasesInput, resetTagsInput]);

  const handleStartCreateNew = useCallback(() => {
    setSelectedSnippetId(null);
    setIsCreatingNew(true);
    resetKeyInput('');
    resetTitleInput('');
    resetContentInput('');
    resetAliasesInput('');
    resetTagsInput('');
    setIsSensitive(false);
    setIsPinned(false);
    setSensitiveRevealed(true);
    setKeyError('');
    setTimeout(() => keyInputRef.current?.focus(), 50);
  }, [resetKeyInput, resetTitleInput, resetContentInput, resetAliasesInput, resetTagsInput]);

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
        if (activeFilter !== 'all' && activeFilter !== 'pinned' && activeFilter !== 'recent' && !snippet.tags.includes(activeFilter)) return false;
      }

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
    }), [snippets, activeFilter, searchQuery, sortBy]);

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
        <div className="manager-sidebar-heading">
          <span>分类目录</span>
          <button type="button" className="manager-icon-button manager-icon-button-small" aria-label="打开系统设置" title="打开系统设置" onClick={onOpenSettings}>
            <Settings className="size-3.5" aria-hidden />
          </button>
        </div>
        <div className="manager-side-scroll">
          {navItem('all', '全部片段', <Layers className="size-4" aria-hidden />, snippets.filter(snippet => !snippet.deletedAt).length, 'accent')}
          {navItem('pinned', '固定片段', <Star className="size-4" aria-hidden />, pinnedCount, 'warning')}
          {navItem('recent', '最近使用', <Clock className="size-4" aria-hidden />)}
          <div className="manager-side-divider" />
          <div className="manager-side-section-title">标签分类</div>
          {allTags.map(tag => navItem(tag, tag, <Tag className="size-3.5" aria-hidden />, tagCounts[tag]))}
          <div className="manager-side-divider" />
          {navItem('trash', '回收站', <Trash2 className="size-4" aria-hidden />, deletedCount || undefined, 'danger')}
        </div>
      </aside>

      <section className="manager-list-pane" aria-label="片段列表">
        <div className="manager-list-toolbar">
          <div className="manager-list-search-row">
            <label className="manager-search-field">
              <Search className="size-4" aria-hidden />
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
                  <X className="size-3.5" aria-hidden />
                </button>
              )}
            </label>
            <button type="button" className="manager-add-button" aria-label="新建片段" title="新建片段" onClick={handleStartCreateNew}>
              <Plus className="size-4" aria-hidden />
            </button>
          </div>
          <div className="manager-list-meta">
            <span>{activeFilter === 'trash' ? '回收站' : `${filteredSnippets.length} 条结果`}</span>
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
              <div className="manager-empty-mark"><Layers className="size-5" aria-hidden /></div>
              <strong>暂无匹配的文本片段</strong>
              <span>{activeFilter === 'trash' ? '回收站为空' : '从一个简短、可重复使用的 Key 开始。'}</span>
              {activeFilter !== 'trash' && <button type="button" className="manager-text-action" onClick={handleStartCreateNew}>新建第一条片段</button>}
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
                    <span className="manager-snippet-mark" aria-hidden>
                      {snippet.pinned ? <Star className="size-3.5" fill="currentColor" /> : snippet.key.slice(0, 2).toUpperCase()}
                    </span>
                    <span className="manager-snippet-copy">
                      <span className="manager-snippet-title">{snippet.title}</span>
                      <span className="manager-snippet-subline">
                        <span className="manager-snippet-key">{snippet.key}</span>
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

      <main className="manager-editor" aria-label={isCreatingNew ? '创建片段' : '编辑片段'}>
        {selectedSnippet || isCreatingNew ? (
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
                      {copiedId === selectedSnippet.id ? <Check className="size-3.5" aria-hidden /> : <Copy className="size-3.5" aria-hidden />}
                      {copiedId === selectedSnippet.id ? '已复制' : '仅复制'}
                    </button>
                    <button type="button" className="manager-button manager-button-primary" onClick={() => onPasteSnippet(selectedSnippet)}>快速粘贴</button>
                  </>
                )}
                {selectedSnippet?.deletedAt ? (
                  <>
                    <button type="button" className="manager-button manager-button-secondary" onClick={() => onRestoreSnippet(selectedSnippet.id)}>
                      <RotateCcw className="size-3.5" aria-hidden />恢复
                    </button>
                    <button type="button" className="manager-button manager-button-danger" onClick={() => onPermanentDeleteSnippet(selectedSnippet.id)}>
                      <Trash2 className="size-3.5" aria-hidden />彻底删除
                    </button>
                  </>
                ) : selectedSnippet && !isCreatingNew ? (
                  <button type="button" className="manager-icon-button manager-danger-icon" aria-label="放入回收站" title="放入回收站" onClick={() => onDeleteSnippet(selectedSnippet.id)}>
                    <Trash2 className="size-4" aria-hidden />
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
                        {sensitiveRevealed ? <EyeOff className="size-3.5" aria-hidden /> : <Eye className="size-3.5" aria-hidden />}
                        {sensitiveRevealed ? '隐蔽' : '显示明文'}
                      </button>
                    )}
                    <span className="manager-character-count">{contentInput.length} 字符</span>
                  </div>
                </div>
                {isSensitive && !sensitiveRevealed ? (
                  <button type="button" className="manager-sensitive-panel" onClick={() => setSensitiveRevealed(true)} aria-label="显示敏感文本正文">
                    <Shield className="size-6" aria-hidden />
                    <strong>敏感文本正文默认已安全隐蔽</strong>
                    <span>点击此处显示明文正文</span>
                  </button>
                ) : (
                  <textarea
                    value={contentInput}
                    onChange={event => setContentInput(event.target.value)}
                    onKeyDown={onContentKeyDown}
                    placeholder="在此输入需要快速粘贴的任意文本片段... (支持 Ctrl+Z 撤销)"
                    rows={8}
                    className="manager-control manager-textarea manager-mono"
                  />
                )}
              </section>

              <section className="manager-form-section">
                <div className="manager-section-heading"><h2>元信息</h2><span>别名和标签帮助你更快找到片段。</span></div>
                <div className="manager-form-grid">
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
              <button type="button" className="manager-button manager-button-primary manager-save-button" onClick={handleSave}>
                保存片段
              </button>
            </div>
          </div>
        ) : (
          <div className="manager-editor-empty">
            <div className="manager-empty-mark manager-empty-mark-large">
              <Layers className="size-6" aria-hidden />
            </div>
            <strong>未选择任何片段</strong>
            <span>在左侧选择一个片段查看详情，或直接新建。</span>
            <button type="button" className="manager-button manager-button-primary" onClick={handleStartCreateNew}>
              新建片段
            </button>
          </div>
        )}
      </main>
    </div>
  );
};
