import React, { useState, useEffect, useRef } from 'react';
import { Snippet, SearchMatchResult } from '../types/snippet';
import { searchSnippets } from '../utils/searchEngine';
import { Search, Star, Eye, EyeOff, Lock, Settings, Sparkles, Command, CornerDownLeft } from 'lucide-react';

interface QuickSearchWindowProps {
  snippets: Snippet[];
  onPasteSnippet: (snippet: Snippet) => void;
  onCopySnippet: (snippet: Snippet) => void;
  onEditSnippet: (snippet: Snippet) => void;
  onCreateNewSnippet: (prefillKey?: string) => void;
  onClose?: () => void;
}

export const QuickSearchWindow: React.FC<QuickSearchWindowProps> = ({
  snippets,
  onPasteSnippet,
  onCopySnippet,
  onEditSnippet,
  onCreateNewSnippet,
  onClose
}) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [results, setResults] = useState<SearchMatchResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [showPreview, setShowPreview] = useState(false);
  const [revealedSensitiveIds, setRevealedSensitiveIds] = useState<Record<string, boolean>>({});
  
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // Focus input on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Handle search filtering
  useEffect(() => {
    const searchResults = searchSnippets(snippets, searchQuery);
    setResults(searchResults);
    setSelectedIndex(0);
  }, [searchQuery, snippets]);

  // Handle keyboard events
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // 1. Esc: close immediately
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose?.();
        return;
      }

      // 2. Tab: toggle preview
      if (e.key === 'Tab') {
        e.preventDefault();
        setShowPreview(prev => !prev);
        return;
      }

      // 3. Arrow keys: navigate results
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedIndex(prev => Math.min(prev + 1, Math.max(0, results.length - 1)));
        return;
      }
      
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedIndex(prev => Math.max(prev - 1, 0));
        return;
      }

      const isModifier = e.ctrlKey || e.metaKey;

      if (isModifier) {
        // Ctrl/Cmd + 1~9
        if (/^[1-9]$/.test(e.key)) {
          e.preventDefault();
          const index = parseInt(e.key) - 1;
          if (index >= 0 && index < results.length) {
            onPasteSnippet(results[index].snippet);
          }
          return;
        }

        // Ctrl/Cmd + Enter
        if (e.key === 'Enter' && results.length > 0) {
          e.preventDefault();
          onCopySnippet(results[selectedIndex].snippet);
          return;
        }

        // Ctrl/Cmd + E
        if (e.key === 'e' || e.key === 'E') {
          e.preventDefault();
          if (results.length > 0) {
            onEditSnippet(results[selectedIndex].snippet);
          }
          return;
        }

        // Ctrl/Cmd + N
        if (e.key === 'n' || e.key === 'N') {
          e.preventDefault();
          onCreateNewSnippet(searchQuery);
          return;
        }
      } else {
        // Enter: paste and close
        if (e.key === 'Enter' && results.length > 0) {
          e.preventDefault();
          onPasteSnippet(results[selectedIndex].snippet);
          return;
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [results, selectedIndex, showPreview, searchQuery, onClose, onPasteSnippet, onCopySnippet, onEditSnippet, onCreateNewSnippet]);

  // Scroll active item into view
  useEffect(() => {
    if (listRef.current) {
      const activeElement = listRef.current.children[selectedIndex] as HTMLElement;
      if (activeElement) {
        activeElement.scrollIntoView({ block: 'nearest' });
      }
    }
  }, [selectedIndex]);

  const toggleSensitive = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setRevealedSensitiveIds(prev => ({ ...prev, [id]: !prev[id] }));
  };

  const selectedSnippet = results[selectedIndex]?.snippet;

  return (
    <div className="fixed inset-0 flex items-center justify-center z-50 theme-backdrop backdrop-blur-xs p-4">
      {/* Raycast Glassmorphism Container (~760x480) */}
      <div 
        className="w-[760px] h-[480px] raycast-window rounded-2xl flex flex-col overflow-hidden animate-pop-in"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Search area (Top Input Bar) */}
        <div className="flex-none px-4 py-3.5 border-b theme-divider flex items-center gap-3">
          <Search className="w-5 h-5 text-theme-muted flex-shrink-0" />
          <input
            ref={inputRef}
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full bg-transparent outline-none border-none text-[17px] font-normal text-theme placeholder:text-theme-disabled selection:bg-[color:var(--accent)] selection:text-[color:var(--accent-contrast)]"
            placeholder="Search snippets or keys (e.g. window management, addr)..."
          />
        </div>

        {/* Main Content Area */}
        <div className="flex-1 flex overflow-hidden">
          {/* Results list */}
          <div className={`flex flex-col h-full overflow-hidden ${showPreview ? 'w-1/2 border-r theme-divider' : 'w-full'}`}>
            <div className="flex-none px-4 py-2 text-[11px] font-semibold text-theme-muted uppercase tracking-wider">
              Results ({results.length})
            </div>
            
            <div ref={listRef} className="flex-1 overflow-y-auto px-2 py-1 space-y-1">
              {results.length === 0 ? (
                <div className="py-12 px-4 text-center text-theme-muted text-sm">
                  {searchQuery ? (
                    <div className="space-y-2">
                      <p className="text-theme-muted">未找到与 &quot;{searchQuery}&quot; 匹配的片段</p>
                      <button
                        onClick={() => onCreateNewSnippet(searchQuery)}
                        className="btn-primary inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors"
                      >
                        <span>以 &quot;{searchQuery}&quot; 为 Key 新建片段</span>
                        <kbd className="raycast-kbd text-[10px]">⌘N</kbd>
                      </button>
                    </div>
                  ) : (
                    <p>暂无片段数据</p>
                  )}
                </div>
              ) : (
                results.map((res, index) => {
                  const s = res.snippet;
                  const isSelected = index === selectedIndex;
                  return (
                    <div
                      key={s.id}
                      className={`group flex items-center justify-between px-3 py-2.5 rounded-xl cursor-pointer transition-all ${
                        isSelected 
                          ? 'raycast-item-active' 
                          : 'interactive-muted'
                      }`}
                      onClick={() => {
                        setSelectedIndex(index);
                        onPasteSnippet(s);
                      }}
                    >
                      <div className="flex items-center gap-3 min-w-0 pr-2">
                        {/* Left Icon Badge - Warm Rose Accent Fill when selected */}
                        <div className={`w-7 h-7 rounded-lg flex items-center justify-center font-mono font-bold text-xs flex-shrink-0 transition-all ${
                          isSelected
                            ? 'brand-mark'
                            : 'badge-accent'
                        }`}>
                          {s.pinned ? <Star className="w-3.5 h-3.5 fill-current" /> : s.key.slice(0, 2).toUpperCase()}
                        </div>

                        {/* Title & Key Badge */}
                        <div className="min-w-0 flex items-center gap-2">
                          <span className="text-[14px] font-medium truncate text-theme">
                            {s.title}
                          </span>
                          <span className={`text-xs font-mono px-1.5 py-0.5 rounded text-[11px] font-semibold ${
                            isSelected 
                              ? 'badge-accent' 
                              : 'badge-accent'
                          }`}>
                            {s.key}
                          </span>
                        </div>
                      </div>

                      {/* Right Tags & Shortcut Hint */}
                      <div className="flex items-center gap-2 flex-shrink-0">
                        {s.tags.slice(0, 1).map(tag => (
                          <span key={tag} className={`text-[10px] px-1.5 py-0.5 rounded ${
                            isSelected ? 'badge-neutral' : 'badge-neutral'
                          }`}>
                            #{tag}
                          </span>
                        ))}
                        {index < 9 && (
                          <span className="text-[11px] font-mono text-theme-muted">
                            ⌘{index + 1}
                          </span>
                        )}
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          </div>

          {/* Preview pane */}
          {showPreview && (
            <div className="w-1/2 flex flex-col overflow-hidden theme-pane-muted p-4 border-l theme-divider-subtle">
              {selectedSnippet ? (
                <div className="flex-1 space-y-3.5 overflow-y-auto">
                  
                  {/* Top Action Header */}
                  <div className="flex items-center justify-between">
                    <span className="text-[10px] font-semibold uppercase tracking-wider text-theme-muted">
                      片段内容预览
                    </span>
                    <div className="flex items-center gap-1.5">
                      {selectedSnippet.sensitive && (
                        <button
                          onClick={e => toggleSensitive(selectedSnippet.id, e)}
                          className="flex items-center gap-1 text-[11px] text-accent hover:underline mr-1"
                        >
                          {revealedSensitiveIds[selectedSnippet.id] ? (
                            <>
                              <EyeOff className="w-3.5 h-3.5" />
                              <span>遮挡</span>
                            </>
                          ) : (
                            <>
                              <Eye className="w-3.5 h-3.5" />
                              <span>显示明文</span>
                            </>
                          )}
                        </button>
                      )}
                      
                      {/* Gear Icon: Jump to snippet manager page */}
                      <button
                        onClick={() => onEditSnippet(selectedSnippet)}
                        title="跳转到该片段的管理页"
                        className="p-1.5 rounded-lg interactive-muted"
                      >
                        <Settings className="w-4 h-4" />
                      </button>
                    </div>
                  </div>

                  {/* 1. 标题 (Title) */}
                  <div>
                    <h3 className="text-base font-bold text-theme leading-snug">
                      {selectedSnippet.title}
                    </h3>
                  </div>

                  {/* 2. Key */}
                  <div className="flex items-center gap-2">
                    <span className="text-[11px] text-theme-muted font-medium">Key:</span>
                    <span className="badge-accent px-2 py-0.5 rounded text-xs font-mono font-bold">
                      {selectedSnippet.key}
                    </span>
                  </div>

                  {/* 3. 别名 (Aliases) */}
                  {selectedSnippet.aliases && selectedSnippet.aliases.length > 0 && (
                    <div className="flex items-center gap-2 text-xs">
                      <span className="text-[11px] text-theme-muted font-medium flex-shrink-0">别名:</span>
                      <div className="flex flex-wrap gap-1 font-mono text-theme-secondary">
                        {selectedSnippet.aliases.map((alias, aIdx) => (
                          <span key={aIdx} className="badge-neutral px-1.5 py-0.5 rounded text-[11px]">
                            {alias}
                          </span>
                        ))}
                      </div>
                    </div>
                  )}

                  {/* 4. 标签 (Tags) */}
                  {selectedSnippet.tags && selectedSnippet.tags.length > 0 && (
                    <div className="flex items-center gap-2 text-xs">
                      <span className="text-[11px] text-theme-muted font-medium flex-shrink-0">标签:</span>
                      <div className="flex flex-wrap gap-1">
                        {selectedSnippet.tags.map(t => (
                          <span key={t} className="badge-neutral px-2 py-0.5 rounded-md text-[10px] font-medium">
                            #{t}
                          </span>
                        ))}
                      </div>
                    </div>
                  )}

                  {/* 5. 文本内容在最下方 (Content at the very bottom) */}
                  <div className="space-y-1 pt-1">
                    <span className="text-[11px] text-theme-muted font-medium">文本内容:</span>
                    <div className="relative p-3 rounded-xl control text-xs font-mono whitespace-pre-wrap break-all max-h-52 overflow-y-auto leading-relaxed shadow-xs">
                      {selectedSnippet.sensitive && !revealedSensitiveIds[selectedSnippet.id] ? (
                        <div className="flex flex-col items-center justify-center py-6 text-center text-theme-muted">
                          <Lock className="w-5 h-5 mb-1.5 icon-warning" />
                          <span>••••••••••••••••••••</span>
                          <span className="text-[11px] mt-1 text-theme-disabled">敏感信息已隐蔽</span>
                        </div>
                      ) : (
                        selectedSnippet.content
                      )}
                    </div>
                  </div>

                </div>
              ) : (
                <div className="flex-1 flex items-center justify-center text-theme-muted text-sm">
                  选择左侧片段进行预览
                </div>
              )}
            </div>
          )}
        </div>

        {/* Bottom Raycast Status Footer Bar (Matching ref1.png) */}
        <div className="flex-none px-4 py-2.5 theme-titlebar border-t theme-divider text-xs text-theme-muted flex justify-between items-center select-none">
          {/* Left product logo icon - Rich Rose Crimson Obsidian Gradient */}
          <div className="flex items-center gap-2">
            <div className="brand-mark w-5 h-5 rounded-md flex items-center justify-center font-bold text-[10px]">
              S
            </div>
            <span className="font-medium text-[13px] text-theme tracking-tight">Searchis</span>
          </div>

          {/* Right Action Shortcuts (Raycast style) */}
          <div className="flex items-center gap-1.5 text-[12px] font-medium text-theme-secondary">
            <span className="flex items-center gap-1">
              <span className="text-theme-muted">Open Command</span>
              <kbd className="raycast-kbd">↵</kbd>
            </span>

            <span className="theme-separator mx-1 font-light">|</span>

            <span className="flex items-center gap-1">
              <span className="text-theme-muted">仅复制</span>
              <kbd className="raycast-kbd">⌘↵</kbd>
            </span>

            <span className="theme-separator mx-1 font-light">|</span>

            <span className="flex items-center gap-1">
              <span className="text-theme-muted">Actions</span>
              <kbd className="raycast-kbd">⌘K</kbd>
            </span>

            <span className="theme-separator mx-1 font-light">|</span>

            <span className="flex items-center gap-1">
              <span className="text-theme-muted">预览</span>
              <kbd className="raycast-kbd">Tab</kbd>
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};
