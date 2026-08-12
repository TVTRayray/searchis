import React, { useState, useEffect, useRef } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { SearchResultItem, snippetsApi } from '../api/snippets';
import { Star, Eye, EyeOff, Lock, Settings, AlertCircle } from 'lucide-react';
import {
  Box,
  VStack,
  HStack,
  Inline,
} from './layout';
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandShortcut,
} from '@/components/ui/command';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Kbd } from '@/components/ui/kbd';

interface QuickSearchWindowProps {
  onClose: () => void;
  onEditSnippet: (item: SearchResultItem) => void;
  onCreateNewSnippet: (prefillKey?: string) => void;
  maxResultsCount?: number;
}

const DEFAULT_LIMIT = 20;

export const QuickSearchWindow: React.FC<QuickSearchWindowProps> = ({
  onClose,
  onEditSnippet,
  onCreateNewSnippet,
  maxResultsCount = DEFAULT_LIMIT,
}) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [results, setResults] = useState<SearchResultItem[]>([]);
  const [total, setTotal] = useState(0);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [showPreview, setShowPreview] = useState(false);
  const [revealedSensitiveIds, setRevealedSensitiveIds] = useState<Record<string, boolean>>({});
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const inputRef = useRef<HTMLInputElement>(null);
  const seqRef = useRef(0);
  const copyingRef = useRef(false);

  // Focus search input on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Real backend search with debounce
  useEffect(() => {
    const seq = ++seqRef.current;
    setLoading(true);
    setError(null);
    const timer = window.setTimeout(async () => {
      try {
        const response = await snippetsApi.search(searchQuery, maxResultsCount);
        if (seq !== seqRef.current) return;
        setResults(response.items);
        setTotal(response.total);
        setSelectedId(response.items[0]?.id ?? null);
      } catch (searchError) {
        if (seq === seqRef.current) {
          setError((searchError as { message?: string }).message ?? '检索失败');
        }
      } finally {
        if (seq === seqRef.current) setLoading(false);
      }
    }, 120);
    return () => window.clearTimeout(timer);
  }, [searchQuery, maxResultsCount]);

  const selectedIndex = results.findIndex(r => r.id === selectedId);
  const selected = selectedIndex >= 0 ? results[selectedIndex] : null;

  // Keyboard navigation & shortcuts — capture phase so cmdk never double-handles
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.isComposing) return;

      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        onClose();
        return;
      }

      if (e.key === 'Tab') {
        e.preventDefault();
        e.stopPropagation();
        setShowPreview(prev => !prev);
        return;
      }

      if (e.key === 'ArrowDown') {
        e.preventDefault();
        e.stopPropagation();
        if (results.length > 0) {
          const next = Math.min(selectedIndex + 1, results.length - 1);
          setSelectedId(results[next].id);
        }
        return;
      }

      if (e.key === 'ArrowUp') {
        e.preventDefault();
        e.stopPropagation();
        if (results.length > 0) {
          const prev = Math.max(selectedIndex - 1, 0);
          setSelectedId(results[prev].id);
        }
        return;
      }

      const isModifier = e.ctrlKey || e.metaKey;

      if (isModifier) {
        if (/^[1-9]$/.test(e.key)) {
          e.preventDefault();
          e.stopPropagation();
          const index = parseInt(e.key) - 1;
          if (index >= 0 && index < results.length) {
            const opId = crypto.randomUUID();
            snippetsApi.executePaste(results[index].id, opId, true).then(() => onClose());
          }
          return;
        }

        if (e.key === 'Enter' && selected) {
          e.preventDefault();
          e.stopPropagation();
          const opId = crypto.randomUUID();
          snippetsApi.executePaste(selected.id, opId, true).then(() => onClose());
          return;
        }

        if (e.key === 'e' || e.key === 'E') {
          e.preventDefault();
          e.stopPropagation();
          if (selected) onEditSnippet(selected);
          return;
        }

        if (e.key === 'n' || e.key === 'N') {
          e.preventDefault();
          e.stopPropagation();
          onCreateNewSnippet(searchQuery);
          return;
        }
      } else {
        if (e.key === 'Enter' && selected) {
          e.preventDefault();
          e.stopPropagation();
          const opId = crypto.randomUUID();
          snippetsApi.executePaste(selected.id, opId, true).then(() => onClose());
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown, true);
    return () => window.removeEventListener('keydown', handleKeyDown, true);
  }, [results, selectedIndex, selected, searchQuery, onClose, onEditSnippet, onCreateNewSnippet]);

  const toggleSensitive = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setRevealedSensitiveIds(prev => ({ ...prev, [id]: !prev[id] }));
  };

  const highlightQuery = (text: string, query: string) => {
    const q = query.trim();
    if (!q) return text;
    const idx = text.toLowerCase().indexOf(q.toLowerCase());
    if (idx < 0) return text;
    return (
      <>
        {text.slice(0, idx)}
        <mark className="rounded-sm bg-[color:var(--color-accent-subtle)] text-[color:var(--color-accent-fg)] px-0.5">
          {text.slice(idx, idx + q.length)}
        </mark>
        {text.slice(idx + q.length)}
      </>
    );
  };

  const pinnedResults = results.filter(r => r.pinned);
  const otherResults = results.filter(r => !r.pinned);

  const renderResultItem = (item: SearchResultItem, index: number) => {
    const isSelected = item.id === selectedId;
    return (
      <CommandItem
        key={item.id}
        value={item.id}
        onSelect={() => {
          const opId = crypto.randomUUID();
          snippetsApi.executePaste(item.id, opId, true).then(() => onClose());
        }}
        className="gap-2 rounded-lg py-2 pl-2 pr-3"
      >
        <Box
          width="28px"
          height="28px"
          radius="sm"
          className={`flex items-center justify-center font-mono font-bold text-xs shrink-0 ${
            isSelected ? 'brand-mark' : 'badge-accent'
          }`}
        >
          {item.pinned ? <Star className="w-3.5 h-3.5 fill-current" /> : item.key.slice(0, 2).toUpperCase()}
        </Box>
        <VStack gap="2xs" className="min-w-0 flex-1">
          <span className="text-xs font-semibold truncate">{item.title}</span>
          <HStack align="center" gap="xs">
            <span className="font-mono text-[11px] text-[color:var(--color-fg-muted)] truncate">
              {highlightQuery(item.key, searchQuery)}
            </span>
            {item.tags.slice(0, 1).map(tag => (
              <Badge key={tag} variant="secondary" className="rounded-full px-1.5 py-0 text-[10px]">
                #{tag}
              </Badge>
            ))}
          </HStack>
        </VStack>
        {item.sensitive && <Lock className="w-3 h-3 text-warning shrink-0" />}
        {index < 9 && <CommandShortcut className="text-[11px]">Ctrl+{index + 1}</CommandShortcut>}
      </CommandItem>
    );
  };

  return (
    <Box
      padding="md"
      className="fixed inset-0 flex items-center justify-center z-50 theme-backdrop backdrop-blur-xs"
      onClick={() => onClose()}
    >
      <Box
        width="min(760px, 92vw)"
        height="480px"
        radius="xl"
        shadow="window"
        background="surface"
        border="all"
        overflow="hidden"
        role="dialog"
        aria-modal="true"
        aria-label="快速检索"
        className="raycast-window flex flex-col animate-pop-in select-none"
        onClick={(e: React.MouseEvent) => e.stopPropagation()}
      >
        {error && (
          <Box paddingX="lg" paddingY="sm" className="status-danger border-b theme-divider flex items-center gap-2" role="alert">
            <AlertCircle className="w-4 h-4 shrink-0" />
            <span className="text-xs">{error}</span>
          </Box>
        )}

        <Command shouldFilter={false} value={selectedId ?? ''} onValueChange={setSelectedId} className="flex h-full flex-col bg-transparent">
          <CommandInput
            autoFocus
            ref={inputRef as any}
            value={searchQuery}
            onValueChange={setSearchQuery}
            placeholder="搜索 Key 或文本内容 (如: addr, email-work, pg)..."
            aria-label="搜索片段"
            className="text-base"
          />

          <div className="flex flex-1 overflow-hidden">
            <CommandList className={`max-h-full overflow-y-auto ${showPreview ? 'w-1/2 border-r' : 'w-full'}`}>
              <CommandEmpty className="py-12">
                <VStack align="center" justify="center" gap="md" className="px-4 text-center">
                  {searchQuery ? (
                    <>
                      <span className="text-xs text-muted-foreground">未找到与 &quot;{searchQuery}&quot; 匹配的片段</span>
                      <Button
                        variant="default"
                        size="sm"
                        className="gap-1.5"
                        onClick={() => onCreateNewSnippet(searchQuery)}
                      >
                        <Star className="size-3.5" />
                        以 &quot;{searchQuery}&quot; 为 Key 新建片段 <Kbd>Ctrl+N</Kbd>
                      </Button>
                    </>
                  ) : (
                    <span className="text-xs text-muted-foreground">暂无片段数据</span>
                  )}
                </VStack>
              </CommandEmpty>

              {pinnedResults.length > 0 && (
                <CommandGroup heading={`固定片段 · ${pinnedResults.length}`}>
                  {pinnedResults.map(res => renderResultItem(res, results.indexOf(res)))}
                </CommandGroup>
              )}

              {otherResults.length > 0 && (
                <CommandGroup heading={pinnedResults.length > 0 ? `全部结果 · ${otherResults.length}` : `匹配片段结果 · ${otherResults.length} 条`}>
                  {otherResults.map(res => renderResultItem(res, results.indexOf(res)))}
                </CommandGroup>
              )}
            </CommandList>

            {showPreview && (
              <Box width="50%" background="sunken" padding="lg" overflow="auto" className="border-l">
                {selected ? (
                  <VStack gap="md">
                    <HStack align="center" justify="space-between">
                      <span className="text-xs font-medium text-muted-foreground">片段内容预览</span>
                      <HStack align="center" gap="xs">
                        {selected.sensitive && (
                          <Button variant="ghost" size="sm" onClick={(e) => toggleSensitive(selected.id, e)}>
                            {revealedSensitiveIds[selected.id] ? (
                              <><EyeOff className="size-3.5" /><span>遮挡</span></>
                            ) : (
                              <><Eye className="size-3.5" /><span>明文</span></>
                            )}
                          </Button>
                        )}
                        <Button variant="ghost" size="icon" aria-label="编辑片段" onClick={() => onEditSnippet(selected)}>
                          <Settings className="size-4" />
                        </Button>
                      </HStack>
                    </HStack>
                    <VStack gap="2xs">
                      <h3 className="text-base font-bold text-theme leading-snug">{selected.title}</h3>
                      <Inline gap="xs" align="center">
                        <span className="text-[11px] text-muted-foreground">Key:</span>
                        <Badge variant="outline" className="rounded-full font-mono text-[11px]">{selected.key}</Badge>
                      </Inline>
                    </VStack>
                    {selected.aliases.length > 0 && (
                      <Inline gap="xs" align="center">
                        <span className="text-[11px] text-muted-foreground">别名:</span>
                        {selected.aliases.map(a => <Badge key={a} variant="secondary" className="text-[10px]">{a}</Badge>)}
                      </Inline>
                    )}
                    {selected.tags.length > 0 && (
                      <Inline gap="xs" align="center">
                        <span className="text-[11px] text-muted-foreground">标签:</span>
                        {selected.tags.map(t => <Badge key={t} variant="secondary" className="text-[10px]">#{t}</Badge>)}
                      </Inline>
                    )}
                    <VStack gap="2xs">
                      <span className="text-[11px] font-medium text-muted-foreground">文本内容</span>
                      <Box padding="md" radius="md" background="subtle" border="all" className="text-xs text-theme-muted whitespace-pre-wrap break-words max-h-48 overflow-auto">
                        {selected.sensitive && !revealedSensitiveIds[selected.id] ? (
                          <VStack align="center" justify="center" gap="xs" className="py-6">
                            <Lock className="w-5 h-5 icon-warning" />
                            <span>••••••••</span>
                            <span className="text-[11px] text-theme-disabled">敏感信息默认已隐蔽</span>
                          </VStack>
                        ) : '内容仅在实际粘贴时使用，不在此处展示。'}
                      </Box>
                    </VStack>
                  </VStack>
                ) : (
                  <VStack align="center" justify="center" className="h-full text-theme-muted text-xs">选择片段查看预览</VStack>
                )}
              </Box>
            )}
          </div>
        </Command>

        {/* Status Bar */}
        <Box paddingX="lg" paddingY="xs" background="titlebar" border="top" className="flex-none text-[11px] text-theme-muted flex items-center justify-between">
          <Inline gap="xs" align="center">
            <Box width="16px" height="16px" radius="xs" className="brand-mark flex items-center justify-center font-bold text-[9px]">S</Box>
            <span className="font-semibold">Searchis</span>
          </Inline>
          <Inline gap="sm" align="center">
            <span>粘贴 <Kbd>↵</Kbd></span>
            <span>|</span>
            <span>仅复制 <Kbd>Ctrl+↵</Kbd></span>
            <span>|</span>
            <span>预览 <Kbd>Tab</Kbd></span>
            <span>|</span>
            <span>编辑 <Kbd>Ctrl+E</Kbd></span>
            <span>|</span>
            <span>关闭 <Kbd>Esc</Kbd></span>
          </Inline>
        </Box>
      </Box>
    </Box>
  );
};
