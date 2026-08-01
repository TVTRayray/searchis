import React, { useState, useMemo } from 'react';
import { Snippet, SidebarFilter, SortOption } from '../types/snippet';
import {
  Layers,
  Star,
  Clock,
  Tag,
  Trash2,
  Settings,
  Search,
  Plus,
  ArrowUpDown,
  Copy,
  Check,
  Edit3,
  Lock,
  Save,
  RotateCcw,
  AlertCircle,
  ExternalLink,
  Shield,
  Eye,
  EyeOff
} from 'lucide-react';

interface ManagerWindowProps {
  snippets: Snippet[];
  onSaveSnippet: (snippet: Snippet) => void;
  onDeleteSnippet: (id: string) => void;
  onRestoreSnippet: (id: string) => void;
  onPermanentDeleteSnippet: (id: string) => void;
  onCopySnippet: (snippet: Snippet) => void;
  onPasteSnippet: (snippet: Snippet) => void;
  onOpenSettings: () => void;
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
}) => {
  const [activeFilter, setActiveFilter] = useState<SidebarFilter>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [sortBy, setSortBy] = useState<SortOption>('updated');
  const [selectedSnippetId, setSelectedSnippetId] = useState<string | null>(snippets[0]?.id || null);

  // Form State for editing
  const selectedSnippet = snippets.find(s => s.id === selectedSnippetId) || null;
  const [keyInput, setKeyInput] = useState('');
  const [titleInput, setTitleInput] = useState('');
  const [contentInput, setContentInput] = useState('');
  const [aliasesInput, setAliasesInput] = useState('');
  const [tagsInput, setTagsInput] = useState('');
  const [isSensitive, setIsSensitive] = useState(false);
  const [isCreatingNew, setIsCreatingNew] = useState(false);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [showSensitiveText, setShowSensitiveText] = useState(false);

  // Sync editor fields when selection changes
  React.useEffect(() => {
    if (selectedSnippet) {
      setKeyInput(selectedSnippet.key);
      setTitleInput(selectedSnippet.title);
      setContentInput(selectedSnippet.content);
      setAliasesInput(selectedSnippet.aliases.join(', '));
      setTagsInput(selectedSnippet.tags.join(', '));
      setIsSensitive(!!selectedSnippet.sensitive);
      setIsCreatingNew(false);
    }
  }, [selectedSnippetId, selectedSnippet]);

  // Extract unique tags and count map
  const { allTags, tagCounts, pinnedCount, deletedCount } = useMemo(() => {
    const counts: Record<string, number> = {};
    let pinned = 0;
    let deleted = 0;

    snippets.forEach(s => {
      if (s.deletedAt) {
        deleted++;
        return;
      }
      if (s.pinned) pinned++;
      s.tags.forEach(t => {
        counts[t] = (counts[t] || 0) + 1;
      });
    });

    return {
      allTags: Object.keys(counts),
      tagCounts: counts,
      pinnedCount: pinned,
      deletedCount: deleted
    };
  }, [snippets]);

  // Filtered & sorted snippet list
  const filteredSnippets = useMemo(() => {
    return snippets.filter(s => {
      // Trash filter vs normal filter
      if (activeFilter === 'trash') {
        return !!s.deletedAt;
      }
      if (s.deletedAt) return false;

      if (activeFilter === 'pinned') return s.pinned;
      if (activeFilter === 'recent') return !!s.lastUsedAt;
      if (activeFilter !== 'all' && activeFilter !== 'pinned' && activeFilter !== 'recent') {
        // Tag filter
        if (!s.tags.includes(activeFilter)) return false;
      }

      // Search filter
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        return (
          s.key.toLowerCase().includes(q) ||
          s.title.toLowerCase().includes(q) ||
          s.content.toLowerCase().includes(q) ||
          s.aliases.some(a => a.toLowerCase().includes(q)) ||
          s.tags.some(t => t.toLowerCase().includes(q))
        );
      }

      return true;
    }).sort((a, b) => {
      if (sortBy === 'updated') return new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime();
      if (sortBy === 'usage') return b.usageCount - a.usageCount;
      if (sortBy === 'alpha') return a.title.localeCompare(b.title);
      if (sortBy === 'key') return a.key.localeCompare(b.key);
      return 0;
    });
  }, [snippets, activeFilter, searchQuery, sortBy]);

  // Handle key duplicate validation
  const isDuplicateKey = useMemo(() => {
    if (!keyInput.trim()) return false;
    return snippets.some(s => s.key.toLowerCase() === keyInput.trim().toLowerCase() && s.id !== selectedSnippetId);
  }, [keyInput, snippets, selectedSnippetId]);

  const handleStartCreateNew = () => {
    setIsCreatingNew(true);
    setSelectedSnippetId(null);
    setKeyInput('new-key');
    setTitleInput('未命名片段');
    setContentInput('');
    setAliasesInput('');
    setTagsInput('常用');
    setIsSensitive(false);
  };

  const handleSaveForm = () => {
    if (!keyInput.trim() || isDuplicateKey) {
      return;
    }

    const snippetToSave: Snippet = {
      id: selectedSnippetId || `snip-${Date.now()}`,
      key: keyInput.trim().toLowerCase().replace(/\s+/g, '-'),
      title: titleInput.trim() || '未命名片段',
      content: contentInput,
      aliases: aliasesInput.split(',').map(a => a.trim()).filter(Boolean),
      tags: tagsInput.split(',').map(t => t.trim()).filter(Boolean),
      pinned: selectedSnippet ? selectedSnippet.pinned : false,
      sensitive: isSensitive,
      usageCount: selectedSnippet ? selectedSnippet.usageCount : 0,
      createdAt: selectedSnippet ? selectedSnippet.createdAt : new Date().toISOString(),
      updatedAt: new Date().toISOString(),
      lastUsedAt: selectedSnippet?.lastUsedAt
    };

    onSaveSnippet(snippetToSave);
    setSelectedSnippetId(snippetToSave.id);
    setIsCreatingNew(false);
  };

  const handleTogglePin = (s: Snippet, e: React.MouseEvent) => {
    e.stopPropagation();
    onSaveSnippet({ ...s, pinned: !s.pinned, updatedAt: new Date().toISOString() });
  };

  return (
    <div className="max-w-7xl mx-auto p-4 sm:p-6">
      
      {/* Container Window Frame */}
      <div className="rounded-lg bg-white dark:bg-zinc-950 border border-zinc-200 dark:border-zinc-800 shadow-lg overflow-hidden flex flex-col min-h-[640px]">
        
        {/* Top Titlebar */}
        <div className="h-11 px-4 bg-zinc-50 dark:bg-zinc-900 border-b border-zinc-200 dark:border-zinc-800 flex items-center justify-between select-none">
          <div className="text-sm font-semibold text-zinc-700 dark:text-zinc-300 flex items-center gap-2">
            <span>Searchis 内容管理主窗口</span>
            <span className="text-xs text-zinc-400 font-mono">(三栏式)</span>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={handleStartCreateNew}
              className="flex items-center gap-1 px-3 py-1.5 rounded-md bg-blue-600 hover:bg-blue-700 text-white text-xs font-medium transition-colors"
            >
              <Plus className="w-4 h-4" />
              <span>新建片段</span>
              <span className="text-xs bg-blue-700/50 border border-blue-500/50 px-1 rounded">⌘N</span>
            </button>
          </div>
        </div>

        {/* Main 3-Column Body */}
        <div className="flex-1 grid grid-cols-12 min-h-[580px]">
          
          {/* COLUMN 1: Navigation Sidebar */}
          <div className="col-span-12 md:col-span-3 bg-zinc-50 dark:bg-zinc-900 border-r border-zinc-200 dark:border-zinc-800 p-3 flex flex-col justify-between">
            <div className="space-y-6">
              
              {/* Group 1: General Nav */}
              <div className="space-y-1">
                <div className="px-2 py-1 text-xs font-semibold text-zinc-500 uppercase tracking-wider">
                  视图分类
                </div>

                <button
                  onClick={() => setActiveFilter('all')}
                  className={`w-full flex items-center justify-between px-3 py-2 rounded-md text-xs font-medium transition-all ${
                    activeFilter === 'all'
                      ? 'bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-300'
                      : 'text-zinc-700 dark:text-zinc-300 hover:bg-zinc-200/50 dark:hover:bg-zinc-800/50'
                  }`}
                >
                  <div className="flex items-center gap-2">
                    <Layers className="w-4 h-4" />
                    <span>全部片段</span>
                  </div>
                  <span className={`px-2 py-0.5 rounded-md text-xs font-mono ${
                    activeFilter === 'all' ? 'bg-blue-200/50 dark:bg-blue-800/50 text-blue-700 dark:text-blue-300' : 'bg-zinc-200 dark:bg-zinc-800 text-zinc-500'
                  }`}>
                    {snippets.filter(s => !s.deletedAt).length}
                  </span>
                </button>

                <button
                  onClick={() => setActiveFilter('pinned')}
                  className={`w-full flex items-center justify-between px-3 py-2 rounded-md text-xs font-medium transition-all ${
                    activeFilter === 'pinned'
                      ? 'bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-300'
                      : 'text-zinc-700 dark:text-zinc-300 hover:bg-zinc-200/50 dark:hover:bg-zinc-800/50'
                  }`}
                >
                  <div className="flex items-center gap-2">
                    <Star className={`w-4 h-4 ${activeFilter === 'pinned' ? 'text-blue-600 dark:text-blue-400' : 'text-amber-500'}`} />
                    <span>收藏与置顶</span>
                  </div>
                  <span className={`px-2 py-0.5 rounded-md text-xs font-mono ${
                    activeFilter === 'pinned' ? 'bg-blue-200/50 dark:bg-blue-800/50 text-blue-700 dark:text-blue-300' : 'bg-zinc-200 dark:bg-zinc-800 text-zinc-500'
                  }`}>
                    {pinnedCount}
                  </span>
                </button>

                <button
                  onClick={() => setActiveFilter('recent')}
                  className={`w-full flex items-center justify-between px-3 py-2 rounded-md text-xs font-medium transition-all ${
                    activeFilter === 'recent'
                      ? 'bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-300'
                      : 'text-zinc-700 dark:text-zinc-300 hover:bg-zinc-200/50 dark:hover:bg-zinc-800/50'
                  }`}
                >
                  <div className="flex items-center gap-2">
                    <Clock className="w-4 h-4" />
                    <span>最近使用</span>
                  </div>
                </button>
              </div>

              {/* Group 2: Tags */}
              <div className="space-y-1">
                <div className="px-2 py-1 text-xs font-semibold text-zinc-500 uppercase tracking-wider flex items-center justify-between">
                  <span>按标签整理</span>
                  <Tag className="w-3 h-3 text-zinc-400" />
                </div>

                {allTags.map(tag => (
                  <button
                    key={tag}
                    onClick={() => setActiveFilter(tag)}
                    className={`w-full flex items-center justify-between px-3 py-1.5 rounded-md text-xs font-medium transition-all ${
                      activeFilter === tag
                        ? 'bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-300'
                        : 'text-zinc-600 dark:text-zinc-400 hover:bg-zinc-200/50 dark:hover:bg-zinc-800/50'
                    }`}
                  >
                    <div className="flex items-center gap-2">
                      <span className="text-zinc-400 font-mono">#</span>
                      <span>{tag}</span>
                    </div>
                    <span className={`px-1.5 py-0.5 rounded-md text-xs ${
                      activeFilter === tag ? 'bg-blue-200/50 dark:bg-blue-800/50 text-blue-700 dark:text-blue-300' : 'text-zinc-400'
                    }`}>
                      {tagCounts[tag]}
                    </span>
                  </button>
                ))}
              </div>

              {/* Group 3: Trash */}
              <div className="space-y-1 pt-2 border-t border-zinc-200 dark:border-zinc-800">
                <button
                  onClick={() => setActiveFilter('trash')}
                  className={`w-full flex items-center justify-between px-3 py-2 rounded-md text-xs font-medium transition-all ${
                    activeFilter === 'trash'
                      ? 'bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-300'
                      : 'text-zinc-600 dark:text-zinc-400 hover:bg-zinc-200/50 dark:hover:bg-zinc-800/50'
                  }`}
                >
                  <div className="flex items-center gap-2">
                    <Trash2 className="w-4 h-4" />
                    <span>回收站</span>
                  </div>
                  <span className={`px-2 py-0.5 rounded-md text-xs font-mono ${
                    activeFilter === 'trash' ? 'bg-red-200/50 dark:bg-red-800/50 text-red-700 dark:text-red-300' : 'bg-zinc-200 dark:bg-zinc-800 text-zinc-500'
                  }`}>
                    {deletedCount}
                  </span>
                </button>
              </div>

            </div>

            {/* Bottom Settings Link */}
            <div className="pt-3 border-t border-zinc-200 dark:border-zinc-800">
              <button
                onClick={onOpenSettings}
                className="w-full flex items-center gap-2 px-3 py-2 rounded-md text-xs text-zinc-600 dark:text-zinc-400 hover:bg-zinc-200/50 dark:hover:bg-zinc-800/50 transition-colors"
              >
                <Settings className="w-4 h-4 text-zinc-400" />
                <span>偏好设置</span>
              </button>
            </div>

          </div>

          {/* COLUMN 2: Snippet List */}
          <div className="col-span-12 md:col-span-4 border-r border-zinc-200 dark:border-zinc-800 flex flex-col bg-white dark:bg-zinc-950">
            
            {/* Search & Sort Controls */}
            <div className="p-3 border-b border-zinc-200 dark:border-zinc-800 space-y-2">
              <div className="relative">
                <Search className="w-4 h-4 absolute left-3 top-2 text-zinc-400" />
                <input
                  type="text"
                  value={searchQuery}
                  onChange={e => setSearchQuery(e.target.value)}
                  placeholder="搜索 Key, 名称或内容..."
                  className="w-full pl-9 pr-3 py-1.5 rounded-md bg-zinc-100 dark:bg-zinc-900 border border-zinc-300 dark:border-zinc-700 text-xs text-zinc-900 dark:text-zinc-100 placeholder-zinc-400 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                />
              </div>

              <div className="flex items-center justify-between text-xs text-zinc-500">
                <span>排序方式:</span>
                <select
                  value={sortBy}
                  onChange={e => setSortBy(e.target.value as SortOption)}
                  className="bg-transparent border border-zinc-300 dark:border-zinc-700 rounded-md px-2 py-1 text-xs text-zinc-700 dark:text-zinc-300 focus:outline-none"
                >
                  <option value="updated">最近修改时间</option>
                  <option value="usage">按使用频率</option>
                  <option value="key">Key 字母顺序</option>
                  <option value="alpha">名称字母顺序</option>
                </select>
              </div>
            </div>

            {/* List items */}
            <div className="flex-1 overflow-y-auto p-2 space-y-1 max-h-[580px]">
              {filteredSnippets.length === 0 ? (
                <div className="py-16 text-center text-xs text-zinc-500">
                  无相关文本片段
                </div>
              ) : (
                filteredSnippets.map(s => {
                  const isSelected = s.id === selectedSnippetId && !isCreatingNew;

                  return (
                    <div
                      key={s.id}
                      onClick={() => {
                        setSelectedSnippetId(s.id);
                        setIsCreatingNew(false);
                      }}
                      className={`p-3 rounded-md cursor-pointer transition-colors ${
                        isSelected
                          ? 'bg-blue-50 dark:bg-blue-900/20'
                          : 'hover:bg-zinc-50 dark:hover:bg-zinc-900'
                      }`}
                    >
                      <div className="flex items-start justify-between gap-2 mb-1">
                        <span className="px-1.5 py-0.5 rounded-md text-xs font-mono font-medium bg-zinc-100 dark:bg-zinc-800 text-zinc-800 dark:text-zinc-200">
                          {s.key}
                        </span>
                        <button
                          onClick={e => handleTogglePin(s, e)}
                          className="p-1 text-zinc-400 hover:text-amber-500 transition-colors"
                        >
                          <Star className={`w-3.5 h-3.5 ${s.pinned ? 'text-amber-500 fill-amber-500' : ''}`} />
                        </button>
                      </div>

                      <div className="text-xs font-semibold text-zinc-900 dark:text-zinc-100 mb-1 truncate">
                        {s.title}
                      </div>

                      <div className="text-xs text-zinc-500 line-clamp-2 font-mono mb-2">
                        {s.sensitive ? '•••••••• (敏感内容)' : s.content}
                      </div>

                      <div className="flex items-center justify-between text-xs text-zinc-400">
                        <div className="flex items-center gap-1">
                          {s.tags.slice(0, 2).map(t => (
                            <span key={t} className="px-1.5 py-0.5 rounded-md bg-zinc-100 dark:bg-zinc-800 text-zinc-500">
                              #{t}
                            </span>
                          ))}
                        </div>
                        <span>已用 {s.usageCount} 次</span>
                      </div>
                    </div>
                  );
                })
              )}
            </div>

          </div>

          {/* COLUMN 3: Right Inspector & Editor */}
          <div className="col-span-12 md:col-span-5 p-5 flex flex-col bg-white dark:bg-zinc-950 overflow-y-auto">
            {selectedSnippet || isCreatingNew ? (
              <div className="space-y-5">
                
                {/* Editor Header */}
                <div className="flex items-center justify-between pb-3 border-b border-zinc-200 dark:border-zinc-800">
                  <div>
                    <h3 className="text-sm font-bold text-zinc-900 dark:text-zinc-100">
                      {isCreatingNew ? '新建文本片段' : (activeFilter === 'trash' ? '回收站项详情' : '编辑片段')}
                    </h3>
                    <p className="text-xs text-zinc-500 mt-1">
                      {isCreatingNew ? '定义用于键盘检索的 Key 与粘贴文本' : `ID: ${selectedSnippet?.id}`}
                    </p>
                  </div>

                  {/* Actions in Editor Header */}
                  {selectedSnippet && !isCreatingNew && (
                    <div className="flex items-center gap-2">
                      <button
                        onClick={() => {
                          onCopySnippet(selectedSnippet);
                          setCopiedId(selectedSnippet.id);
                          setTimeout(() => setCopiedId(null), 1500);
                        }}
                        className="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-zinc-100 dark:bg-zinc-800 hover:bg-zinc-200 dark:hover:bg-zinc-700 text-xs font-medium text-zinc-700 dark:text-zinc-300 transition-colors"
                      >
                        {copiedId === selectedSnippet.id ? <Check className="w-4 h-4 text-emerald-500" /> : <Copy className="w-4 h-4" />}
                        <span>{copiedId === selectedSnippet.id ? '已复制' : '复制'}</span>
                      </button>

                      <button
                        onClick={() => {
                          onPasteSnippet(selectedSnippet);
                        }}
                        className="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-zinc-100 dark:bg-zinc-800 hover:bg-zinc-200 dark:hover:bg-zinc-700 text-xs font-medium text-zinc-700 dark:text-zinc-300 transition-colors"
                      >
                        <ExternalLink className="w-4 h-4" />
                        <span>模拟粘贴</span>
                      </button>
                    </div>
                  )}
                </div>

                {/* Form Fields - Ordered as requested: Key, 名称, 内容, 别名, 标签, 行为设置 */}
                <div className="space-y-4">
                  
                  {/* Field 1: Key */}
                  <div>
                    <label className="block text-xs font-semibold text-zinc-700 dark:text-zinc-300 mb-1.5 flex items-center justify-between">
                      <span>1. Key (唤醒短语)</span>
                      {isDuplicateKey && (
                        <span className="text-xs text-red-600 font-normal flex items-center gap-1">
                          <AlertCircle className="w-3.5 h-3.5" /> Key 重复！
                        </span>
                      )}
                    </label>
                    <input
                      type="text"
                      disabled={activeFilter === 'trash'}
                      value={keyInput}
                      onChange={e => setKeyInput(e.target.value)}
                      placeholder="例如: addr"
                      className={`w-full px-3 py-2 rounded-md text-xs font-mono font-medium bg-white dark:bg-zinc-900 border ${
                        isDuplicateKey
                          ? 'border-red-500 focus:border-red-500 focus:ring-1 focus:ring-red-500'
                          : 'border-zinc-300 dark:border-zinc-700 focus:border-blue-500 focus:ring-1 focus:ring-blue-500'
                      } outline-none transition-colors`}
                    />
                  </div>

                  {/* Field 2: Title */}
                  <div>
                    <label className="block text-xs font-semibold text-zinc-700 dark:text-zinc-300 mb-1.5">
                      2. 名称 (Title)
                    </label>
                    <input
                      type="text"
                      disabled={activeFilter === 'trash'}
                      value={titleInput}
                      onChange={e => setTitleInput(e.target.value)}
                      placeholder="例如: 公司地址"
                      className="w-full px-3 py-2 rounded-md text-xs bg-white dark:bg-zinc-900 border border-zinc-300 dark:border-zinc-700 text-zinc-900 dark:text-zinc-100 outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-colors"
                    />
                  </div>

                  {/* Field 3: Content */}
                  <div>
                    <div className="flex items-center justify-between mb-1.5">
                      <label className="block text-xs font-semibold text-zinc-700 dark:text-zinc-300">
                        3. 内容 (Content)
                      </label>
                      <span className="text-xs font-mono text-zinc-500">
                        {contentInput.length} 字符
                      </span>
                    </div>
                    <textarea
                      rows={6}
                      disabled={activeFilter === 'trash'}
                      value={contentInput}
                      onChange={e => setContentInput(e.target.value)}
                      placeholder="输入需要粘贴的文本..."
                      className="w-full px-3 py-2 rounded-md text-xs font-mono bg-white dark:bg-zinc-900 border border-zinc-300 dark:border-zinc-700 text-zinc-900 dark:text-zinc-100 outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-colors resize-y"
                    />
                  </div>

                  {/* Field 4 & 5: Aliases and Tags */}
                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <label className="block text-xs font-semibold text-zinc-700 dark:text-zinc-300 mb-1.5">
                        4. 别名 (逗号分隔)
                      </label>
                      <input
                        type="text"
                        disabled={activeFilter === 'trash'}
                        value={aliasesInput}
                        onChange={e => setAliasesInput(e.target.value)}
                        placeholder="如: dizhi, address"
                        className="w-full px-3 py-2 rounded-md text-xs bg-white dark:bg-zinc-900 border border-zinc-300 dark:border-zinc-700 text-zinc-900 dark:text-zinc-100 outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-colors"
                      />
                    </div>
                    <div>
                      <label className="block text-xs font-semibold text-zinc-700 dark:text-zinc-300 mb-1.5">
                        5. 标签 (逗号分隔)
                      </label>
                      <input
                        type="text"
                        disabled={activeFilter === 'trash'}
                        value={tagsInput}
                        onChange={e => setTagsInput(e.target.value)}
                        placeholder="如: 常用, 工作"
                        className="w-full px-3 py-2 rounded-md text-xs bg-white dark:bg-zinc-900 border border-zinc-300 dark:border-zinc-700 text-zinc-900 dark:text-zinc-100 outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-colors"
                      />
                    </div>
                  </div>

                  {/* Field 6: 行为设置 */}
                  <div className="pt-2">
                    <label className="block text-xs font-semibold text-zinc-700 dark:text-zinc-300 mb-2">
                      6. 行为设置
                    </label>
                    <div className="space-y-3 p-3 rounded-md border border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900/50">
                      <label className="flex items-center gap-2 cursor-pointer text-xs text-zinc-700 dark:text-zinc-300">
                        <input
                          type="checkbox"
                          checked={isSensitive}
                          onChange={e => setIsSensitive(e.target.checked)}
                          disabled={activeFilter === 'trash'}
                          className="rounded-sm border-zinc-300 text-blue-600 focus:ring-blue-500"
                        />
                        <Shield className="w-4 h-4 text-zinc-500" />
                        <span>敏感数据 (列表中隐藏明文)</span>
                      </label>
                      
                      {/* Placeholder for future options */}
                      <div className="flex items-center gap-2 text-xs text-zinc-400 opacity-70">
                        <input type="checkbox" disabled className="rounded-sm border-zinc-300" />
                        <Settings className="w-4 h-4 text-zinc-400" />
                        <span>自动回车 (未来版本支持)</span>
                      </div>
                    </div>
                  </div>

                </div>

                {/* Bottom Actions */}
                <div className="pt-4 mt-auto border-t border-zinc-200 dark:border-zinc-800 flex items-center justify-between">
                  {activeFilter === 'trash' && selectedSnippet ? (
                    <div className="flex items-center gap-3 w-full justify-between">
                      <button
                        onClick={() => onRestoreSnippet(selectedSnippet.id)}
                        className="flex items-center gap-1.5 px-4 py-2 rounded-md bg-zinc-100 hover:bg-zinc-200 dark:bg-zinc-800 dark:hover:bg-zinc-700 text-zinc-900 dark:text-zinc-100 text-xs font-medium transition-colors"
                      >
                        <RotateCcw className="w-4 h-4" />
                        <span>恢复片段</span>
                      </button>
                      <button
                        onClick={() => onPermanentDeleteSnippet(selectedSnippet.id)}
                        className="flex items-center gap-1.5 px-4 py-2 rounded-md bg-red-50 hover:bg-red-100 dark:bg-red-900/20 dark:hover:bg-red-900/40 text-red-600 dark:text-red-400 text-xs font-medium transition-colors"
                      >
                        <Trash2 className="w-4 h-4" />
                        <span>彻底删除</span>
                      </button>
                    </div>
                  ) : (
                    <>
                      <div>
                        {selectedSnippet && !isCreatingNew && (
                          <button
                            onClick={() => onDeleteSnippet(selectedSnippet.id)}
                            className="flex items-center gap-1.5 text-xs text-red-600 hover:text-red-700 font-medium px-3 py-2 rounded-md hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
                          >
                            <Trash2 className="w-4 h-4" />
                            <span>删除</span>
                          </button>
                        )}
                      </div>

                      <div className="flex items-center gap-3">
                        {isCreatingNew && (
                          <button
                            onClick={() => {
                              setIsCreatingNew(false);
                              setSelectedSnippetId(snippets[0]?.id || null);
                            }}
                            className="px-4 py-2 rounded-md bg-zinc-100 dark:bg-zinc-800 text-zinc-700 dark:text-zinc-300 text-xs font-medium hover:bg-zinc-200 dark:hover:bg-zinc-700 transition-colors"
                          >
                            取消
                          </button>
                        )}
                        <button
                          onClick={handleSaveForm}
                          disabled={isDuplicateKey || !keyInput.trim()}
                          className="flex items-center gap-1.5 px-4 py-2 rounded-md bg-blue-600 hover:bg-blue-700 disabled:bg-blue-400 text-white text-xs font-medium transition-colors"
                        >
                          <Save className="w-4 h-4" />
                          <span>保存</span>
                        </button>
                      </div>
                    </>
                  )}
                </div>

              </div>
            ) : (
              <div className="h-full flex items-center justify-center text-xs text-zinc-500">
                请选择片段查看详情或点击新建
              </div>
            )}
          </div>

        </div>

      </div>

    </div>
  );
};
