import React, { useState, useEffect, useRef } from 'react';
import { Snippet, SearchMatchResult } from '../types/snippet';
import { searchSnippets } from '../utils/searchEngine';
import { Search, Star, Eye, EyeOff, Lock, Settings } from 'lucide-react';
import {
  Box,
  VStack,
  HStack,
  Inline,
  Layout,
  LayoutPanel,
  List,
  ListItem,
  Badge,
  Token,
  StatusDot,
  Button,
  IconButton,
  Kbd,
} from './astryx';

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
  onClose,
}) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [results, setResults] = useState<SearchMatchResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [showPreview, setShowPreview] = useState(false);
  const [revealedSensitiveIds, setRevealedSensitiveIds] = useState<Record<string, boolean>>({});

  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  // Focus search input on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Handle search filtering
  useEffect(() => {
    const searchResults = searchSnippets(snippets, searchQuery);
    setResults(searchResults);
    setSelectedIndex(0);
  }, [searchQuery, snippets]);

  // Keyboard navigation & shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose?.();
        return;
      }

      if (e.key === 'Tab') {
        e.preventDefault();
        setShowPreview(prev => !prev);
        return;
      }

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
        if (/^[1-9]$/.test(e.key)) {
          e.preventDefault();
          const index = parseInt(e.key) - 1;
          if (index >= 0 && index < results.length) {
            onPasteSnippet(results[index].snippet);
          }
          return;
        }

        if (e.key === 'Enter' && results.length > 0) {
          e.preventDefault();
          onCopySnippet(results[selectedIndex].snippet);
          return;
        }

        if (e.key === 'e' || e.key === 'E') {
          e.preventDefault();
          if (results.length > 0) {
            onEditSnippet(results[selectedIndex].snippet);
          }
          return;
        }

        if (e.key === 'n' || e.key === 'N') {
          e.preventDefault();
          onCreateNewSnippet(searchQuery);
          return;
        }
      } else {
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
    <Box
      padding="md"
      className="fixed inset-0 flex items-center justify-center z-50 theme-backdrop backdrop-blur-xs"
      onClick={() => onClose?.()}
    >
      {/* Raycast Glassmorphism Container Framed with Astryx Box (760x480) */}
      <Box
        width="760px"
        height="480px"
        radius="xl"
        shadow="window"
        background="surface"
        border="all"
        overflow="hidden"
        className="raycast-window flex flex-col animate-pop-in select-none"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Top Search Input Bar */}
        <Box paddingX="lg" paddingY="md" border="bottom" background="sunken" className="flex-none">
          <HStack align="center" gap="md">
            <Search className="w-5 h-5 text-theme-muted shrink-0" />
            <input
              ref={inputRef}
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full bg-transparent outline-none border-none text-[16px] font-normal text-theme placeholder:text-theme-disabled selection:bg-[color:var(--color-accent)] selection:text-[color:var(--color-accent-contrast)]"
              placeholder="搜索 Key 或文本内容 (如: addr, email-work, pg)..."
            />
          </HStack>
        </Box>

        {/* Main Content Area using Astryx Layout & LayoutPanel */}
        <Layout direction="row" height="calc(100% - 100px)">
          {/* Results list panel */}
          <LayoutPanel
            width={showPreview ? '50%' : '100%'}
            height="100%"
            border={showPreview ? 'right' : 'none'}
            overflow="hidden"
            className="flex flex-col"
          >
            <Box paddingX="lg" paddingY="xs" background="subtle" border="bottom">
              <HStack align="center" justify="space-between">
                <span className="text-[11px] font-bold text-theme-muted uppercase tracking-wider">
                  匹配片段结果
                </span>
                <Badge variant="accent" size="sm">{results.length} 条</Badge>
              </HStack>
            </Box>

            <Box flex="1" overflow="auto" padding="xs">
              {results.length === 0 ? (
                <VStack align="center" justify="center" gap="md" className="py-12 px-4 text-center">
                  {searchQuery ? (
                    <>
                      <span className="text-xs text-theme-muted">未找到与 &quot;{searchQuery}&quot; 匹配的片段</span>
                      <Button
                        variant="primary"
                        size="sm"
                        icon={<Star className="w-3.5 h-3.5" />}
                        onClick={() => onCreateNewSnippet(searchQuery)}
                      >
                        以 &quot;{searchQuery}&quot; 为 Key 新建片段 <Kbd>⌘N</Kbd>
                      </Button>
                    </>
                  ) : (
                    <span className="text-xs text-theme-muted">暂无片段数据</span>
                  )}
                </VStack>
              ) : (
                <List divided={false} ref={listRef as unknown as React.RefObject<HTMLUListElement>}>
                  {results.map((res, index) => {
                    const s = res.snippet;
                    const isSelected = index === selectedIndex;
                    return (
                      <ListItem
                        key={s.id}
                        active={isSelected}
                        onClick={() => {
                          setSelectedIndex(index);
                          onPasteSnippet(s);
                        }}
                        icon={
                          <Box
                            width="28px"
                            height="28px"
                            radius="sm"
                            className={`flex items-center justify-center font-mono font-bold text-xs shrink-0 transition-all ${
                              isSelected ? 'brand-mark' : 'badge-accent'
                            }`}
                          >
                            {s.pinned ? <Star className="w-3.5 h-3.5 fill-current" /> : s.key.slice(0, 2).toUpperCase()}
                          </Box>
                        }
                        title={s.title}
                        meta={<Token size="sm">{s.key}</Token>}
                        extra={
                          <Inline gap="xs" align="center">
                            {s.tags.slice(0, 1).map(tag => (
                              <Badge key={tag} variant="neutral" size="sm">#{tag}</Badge>
                            ))}
                            {index < 9 && (
                              <span className="text-[11px] font-mono text-theme-muted">
                                ⌘{index + 1}
                              </span>
                            )}
                          </Inline>
                        }
                      />
                    );
                  })}
                </List>
              )}
            </Box>
          </LayoutPanel>

          {/* Preview Pane using Astryx LayoutPanel */}
          {showPreview && (
            <LayoutPanel width="50%" height="100%" background="sunken" padding="lg" overflow="auto">
              {selectedSnippet ? (
                <VStack gap="md">
                  <HStack align="center" justify="space-between">
                    <span className="text-[11px] font-bold uppercase tracking-wider text-theme-muted">
                      片段内容预览
                    </span>
                    <HStack align="center" gap="xs">
                      {selectedSnippet.sensitive && (
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={(e) => toggleSensitive(selectedSnippet.id, e)}
                        >
                          {revealedSensitiveIds[selectedSnippet.id] ? (
                            <>
                              <EyeOff className="w-3.5 h-3.5" />
                              <span>遮挡</span>
                            </>
                          ) : (
                            <>
                              <Eye className="w-3.5 h-3.5" />
                              <span>明文</span>
                            </>
                          )}
                        </Button>
                      )}
                      <IconButton
                        icon={<Settings className="w-4 h-4" />}
                        ariaLabel="跳转到该片段的管理编辑页"
                        onClick={() => onEditSnippet(selectedSnippet)}
                      />
                    </HStack>
                  </HStack>

                  {/* Title & Key */}
                  <VStack gap="2xs">
                    <h3 className="text-base font-bold text-theme leading-snug">{selectedSnippet.title}</h3>
                    <Inline gap="xs" align="center">
                      <span className="text-[11px] text-theme-muted">Key:</span>
                      <Token>{selectedSnippet.key}</Token>
                    </Inline>
                  </VStack>

                  {/* Aliases */}
                  {selectedSnippet.aliases && selectedSnippet.aliases.length > 0 && (
                    <Inline gap="xs" align="center">
                      <span className="text-[11px] text-theme-muted">别名:</span>
                      {selectedSnippet.aliases.map(a => (
                        <Badge key={a} variant="neutral" size="sm">{a}</Badge>
                      ))}
                    </Inline>
                  )}

                  {/* Tags */}
                  {selectedSnippet.tags && selectedSnippet.tags.length > 0 && (
                    <Inline gap="xs" align="center">
                      <span className="text-[11px] text-theme-muted">标签:</span>
                      {selectedSnippet.tags.map(t => (
                        <Badge key={t} variant="neutral" size="sm">#{t}</Badge>
                      ))}
                    </Inline>
                  )}

                  {/* Content Box */}
                  <VStack gap="2xs">
                    <span className="text-[11px] text-theme-muted font-medium">文本内容:</span>
                    <Box
                      padding="md"
                      radius="md"
                      background="subtle"
                      border="all"
                      overflow="auto"
                      className="max-h-48 text-xs font-mono whitespace-pre-wrap break-all leading-relaxed"
                    >
                      {selectedSnippet.sensitive && !revealedSensitiveIds[selectedSnippet.id] ? (
                        <VStack align="center" justify="center" gap="xs" className="py-6 text-center text-theme-muted">
                          <Lock className="w-5 h-5 icon-warning" />
                          <span>••••••••••••••••••••</span>
                          <span className="text-[11px] text-theme-disabled">敏感信息默认已隐蔽</span>
                        </VStack>
                      ) : (
                        selectedSnippet.content
                      )}
                    </Box>
                  </VStack>
                </VStack>
              ) : (
                <VStack align="center" justify="center" className="h-full text-theme-muted text-xs">
                  选择左侧片段以查看详细预览
                </VStack>
              )}
            </LayoutPanel>
          )}
        </Layout>

        {/* Bottom Raycast Status Footer Bar */}
        <Box paddingX="lg" paddingY="sm" background="titlebar" border="top" className="flex-none text-xs text-theme-muted">
          <HStack align="center" justify="space-between">
            <HStack align="center" gap="xs">
              <Box width="20px" height="20px" radius="xs" className="brand-mark flex items-center justify-center font-bold text-[10px]">
                S
              </Box>
              <span className="font-semibold text-xs text-theme">Searchis</span>
              <StatusDot status="active" size="sm" />
            </HStack>

            <Inline gap="sm" align="center" className="text-[11px]">
              <Inline gap="3xs" align="center">
                <span className="text-theme-muted">粘贴</span>
                <Kbd>↵</Kbd>
              </Inline>
              <span className="text-theme-muted">|</span>
              <Inline gap="3xs" align="center">
                <span className="text-theme-muted">仅复制</span>
                <Kbd>⌘↵</Kbd>
              </Inline>
              <span className="text-theme-muted">|</span>
              <Inline gap="3xs" align="center">
                <span className="text-theme-muted">预览</span>
                <Kbd>Tab</Kbd>
              </Inline>
              <span className="text-theme-muted">|</span>
              <Inline gap="3xs" align="center">
                <span className="text-theme-muted">关闭</span>
                <Kbd>Esc</Kbd>
              </Inline>
            </Inline>
          </HStack>
        </Box>
      </Box>
    </Box>
  );
};
