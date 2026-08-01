import React, { useState, useEffect, useRef } from 'react';
import { Snippet, SearchMatchResult } from '../types/snippet';
import { searchSnippets } from '../utils/searchEngine';
import { Search } from 'lucide-react';

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

  const selectedSnippet = results[selectedIndex]?.snippet;

  return (
    <div className="fixed inset-0 flex items-center justify-center z-50 bg-black/50">
      {/* 720x460 container */}
      <div 
      className="w-[720px] h-[460px] bg-white dark:bg-zinc-900 rounded-lg shadow-lg flex flex-col overflow-hidden text-zinc-800 dark:text-zinc-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Search area */}
        <div className="flex-none p-4 border-b border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900">
          <div className="flex items-center gap-3">
            <Search className="w-5 h-5 text-zinc-400" />
            <input
              ref={inputRef}
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="flex-1 bg-transparent outline-none text-[16px] text-zinc-900 dark:text-zinc-100 placeholder-zinc-400 dark:placeholder-zinc-500"
              placeholder="输入 Key 或搜索片段内容..."
            />
          </div>
        </div>

        {/* Main Content Area */}
        <div className="flex-1 flex overflow-hidden">
          {/* Results list */}
          <div className={`flex flex-col h-full overflow-hidden ${showPreview ? 'w-1/2 border-r border-zinc-200 dark:border-zinc-800' : 'w-full'}`}>
            <div className="flex-none px-4 py-2 text-xs text-zinc-500 bg-zinc-50 dark:bg-zinc-900 border-b border-zinc-200 dark:border-zinc-800">
              匹配结果 ({results.length})
            </div>
            
            <div ref={listRef} className="flex-1 overflow-y-auto p-2 bg-white dark:bg-zinc-900">
              {results.length === 0 ? (
                <div className="p-4 text-center text-zinc-500 text-sm">
                  {searchQuery ? (
                    <>
                      <p className="mb-2">未找到与 "{searchQuery}" 匹配的片段</p>
                      <button
                        onClick={() => onCreateNewSnippet(searchQuery)}
                        className="text-blue-600 dark:text-blue-400 hover:underline text-xs"
                      >
                        以 "{searchQuery}" 为 Key 新建片段
                      </button>
                    </>
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
                      className={`flex items-center gap-3 p-2 rounded-md cursor-pointer transition-colors ${
                        isSelected ? 'bg-blue-600 text-white' : 'hover:bg-zinc-100 dark:hover:bg-zinc-800'
                      }`}
                      onClick={() => {
                        setSelectedIndex(index);
                        onPasteSnippet(s);
                      }}
                    >
                      {/* Pinned star */}
                      {s.pinned && (
                        <span className={`text-xs ${isSelected ? 'text-amber-300' : 'text-amber-400'}`}>★</span>
                      )}

                      {/* Key badge - more prominent */}
                      <span className={`font-mono font-bold text-sm px-1.5 py-0.5 rounded-md flex-shrink-0 ${
                        isSelected ? 'bg-blue-500 text-white' : 'bg-blue-50 dark:bg-blue-950 text-blue-600 dark:text-blue-400 border border-blue-200 dark:border-blue-800'
                      }`}>
                        {s.key}
                      </span>
                      
                      {/* Title */}
                      <span className={`truncate flex-1 text-sm ${isSelected ? 'text-white' : 'text-zinc-700 dark:text-zinc-300'}`}>
                        {s.title}
                      </span>

                      {/* Tags */}
                      {s.tags.slice(0, 2).map(tag => (
                        <span key={tag} className={`text-xs ${isSelected ? 'text-blue-200' : 'text-zinc-400'}`}>
                          #{tag}
                        </span>
                      ))}

                      {/* Number shortcut hint */}
                      {index < 9 && (
                        <span className={`text-xs font-mono opacity-60 ${isSelected ? 'text-white' : 'text-zinc-500'}`}>
                          ⌘{index + 1}
                        </span>
                      )}
                    </div>
                  );
                })
              )}
            </div>
          </div>

          {/* Preview pane */}
          {showPreview && (
            <div className="w-1/2 flex flex-col overflow-hidden bg-zinc-50 dark:bg-zinc-950">
              {selectedSnippet ? (
                <div className="flex-1 p-4 overflow-y-auto">
                  <div className="mb-3">
                    <span className="font-mono font-bold text-sm px-1.5 py-0.5 rounded-md bg-blue-50 dark:bg-blue-950 text-blue-600 dark:text-blue-400 border border-blue-200 dark:border-blue-800">
                      {selectedSnippet.key}
                    </span>
                    <h3 className="font-semibold text-base text-zinc-800 dark:text-zinc-200 mt-2">{selectedSnippet.title}</h3>
                  </div>

                  {selectedSnippet.aliases && selectedSnippet.aliases.length > 0 && (
                    <div className="flex items-center gap-2 mb-2 text-xs">
                      <span className="text-zinc-400">别名:</span>
                      <div className="flex flex-wrap gap-1 font-mono">
                        {selectedSnippet.aliases.map((alias, i) => (
                          <span key={i} className="px-1.5 py-0.5 rounded bg-zinc-200 dark:bg-zinc-800 text-zinc-600 dark:text-zinc-300 text-xs">
                            {alias}
                          </span>
                        ))}
                      </div>
                    </div>
                  )}

                  {selectedSnippet.tags && selectedSnippet.tags.length > 0 && (
                    <div className="flex items-center gap-2 mb-3 text-xs">
                      <span className="text-zinc-400">标签:</span>
                      <div className="flex flex-wrap gap-1">
                        {selectedSnippet.tags.map(t => (
                          <span key={t} className="px-1.5 py-0.5 rounded bg-zinc-200 dark:bg-zinc-800 text-zinc-500 dark:text-zinc-400 text-xs">
                            #{t}
                          </span>
                        ))}
                      </div>
                    </div>
                  )}
                  
                  <div className="bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 rounded-md p-3">
                    <pre className="text-sm font-mono text-zinc-800 dark:text-zinc-200 whitespace-pre-wrap break-all leading-relaxed">
                      {selectedSnippet.sensitive ? '•••••••••••••••' : selectedSnippet.content}
                    </pre>
                  </div>
                </div>
              ) : (
                <div className="flex-1 flex items-center justify-center text-zinc-500 text-sm">
                  选择左侧片段进行预览
                </div>
              )}
            </div>
          )}
        </div>

        {/* Bottom shortcut hints */}
        <div className="flex-none px-4 py-2.5 bg-zinc-100 dark:bg-zinc-950 border-t border-zinc-200 dark:border-zinc-800 text-xs text-zinc-600 dark:text-zinc-400 flex justify-between items-center">
          <div className="flex items-center gap-4">
            <span><kbd className="mac-kbd">↑</kbd><kbd className="mac-kbd">↓</kbd> 选择</span>
            <span><kbd className="mac-kbd">↵</kbd> 粘贴</span>
            <span><kbd className="mac-kbd">⌘</kbd><kbd className="mac-kbd">↵</kbd> 仅复制</span>
            <span><kbd className="mac-kbd">Tab</kbd> {showPreview ? '收起预览' : '展开预览'}</span>
          </div>
          <div className="flex items-center gap-4">
            <span><kbd className="mac-kbd">⌘</kbd><kbd className="mac-kbd">E</kbd> 编辑</span>
            <span><kbd className="mac-kbd">⌘</kbd><kbd className="mac-kbd">N</kbd> 新建</span>
            <span><kbd className="mac-kbd">Esc</kbd> 关闭</span>
          </div>
        </div>
      </div>
    </div>
  );
};
