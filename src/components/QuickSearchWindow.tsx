import React, { useState, useEffect, useRef } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { SearchResultItem, snippetsApi } from '../api/snippets';
import { Search, Star, Lock, Settings, AlertCircle } from 'lucide-react';
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
  onClose: () => void;
  onEditSnippet: (item: SearchResultItem) => void;
  onCreateNewSnippet: (prefillKey?: string) => void;
}

const DEFAULT_LIMIT = 20;

export const QuickSearchWindow: React.FC<QuickSearchWindowProps> = ({
  onClose,
  onEditSnippet,
  onCreateNewSnippet,
}) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [results, setResults] = useState<SearchResultItem[]>([]);
  const [total, setTotal] = useState(0);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [showPreview, setShowPreview] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const seqRef = useRef(0);
  const copyingRef = useRef(false);

  // Focus search input on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // 窗口每次重新显示（show）时聚焦输入框并清空查询（rofi 式：每次呼出都是新检索）
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    try {
      getCurrentWindow()
        .onFocusChanged(({ payload }) => {
          if (payload) {
            setSearchQuery('');
            setResults([]);
            setTotal(0);
            setSelectedIndex(-1);
            setError(null);
            inputRef.current?.focus();
          }
        })
        .then(fn => { unlisten = fn; })
        .catch(() => {});
    } catch {
      // 非 Tauri 环境
    }
    return () => unlisten?.();
  }, []);

  // Real backend search with debounce, ignoring stale responses.
  useEffect(() => {
    const seq = ++seqRef.current;
    setLoading(true);
    setError(null);
    const timer = window.setTimeout(async () => {
      try {
        const response = await snippetsApi.search(searchQuery, DEFAULT_LIMIT);
        if (seq !== seqRef.current) return;
        setResults(response.items);
        setTotal(response.total);
        setSelectedIndex(prev =>
          response.items.length === 0
            ? -1
            : Math.max(0, Math.min(prev, response.items.length - 1)),
        );
      } catch (searchError) {
        if (seq === seqRef.current) {
          setResults([]);
          setTotal(0);
          setSelectedIndex(-1);
          setError((searchError as { message?: string }).message ?? '检索失败');
        }
      } finally {
        if (seq === seqRef.current) setLoading(false);
      }
    }, 120);
    return () => window.clearTimeout(timer);
  }, [searchQuery]);

  const selected = selectedIndex >= 0 ? results[selectedIndex] ?? null : null;

  // Copy via real command; on success with close intent, notify App to close window.
  const doCopy = async (item: SearchResultItem, keepOpen: boolean) => {
    if (copyingRef.current) return;
    copyingRef.current = true;
    setError(null);
    try {
      const operationId = crypto.randomUUID();
      const outcome = await snippetsApi.copy(item.id, operationId, keepOpen);
      setResults(prev =>
        prev.map(it =>
          it.id === outcome.snippet.id
            ? { ...it, usageCount: outcome.snippet.usageCount }
            : it,
        ),
      );
      if (!keepOpen) onClose();
    } catch (copyError) {
      setError((copyError as { message?: string }).message ?? '复制失败');
    } finally {
      copyingRef.current = false;
    }
  };

  const createFromQuery = async () => {
    const raw = searchQuery.trim();
    if (!raw) return;
    try {
      const outcome = await snippetsApi.prepareNew(raw);
      onCreateNewSnippet(outcome.normalizedKey);
    } catch (prepareError) {
      setError((prepareError as { message?: string }).message ?? '无法生成 Key');
    }
  };

  // Keyboard navigation & shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // 输入法组合态：Esc/J/K/Enter 均不拦截（Esc 用于取消组合）。
      if (e.isComposing) return;

      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
        return;
      }
      if (e.key === 'Tab') {
        e.preventDefault();
        setShowPreview(prev => !prev);
        return;
      }
      // J/K 导航仅在焦点不在文本输入框时生效，避免劫持正文输入（如 "json"/"key"）。
      const target = e.target as HTMLElement | null;
      const isTypingTarget = !!target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA');
      if (!isTypingTarget && (e.key === 'ArrowDown' || e.key === 'k' || e.key === 'K')) {
        e.preventDefault();
        setSelectedIndex(prev => Math.min(prev + 1, Math.max(0, results.length - 1)));
        return;
      }
      if (!isTypingTarget && (e.key === 'ArrowUp' || e.key === 'j' || e.key === 'J')) {
        e.preventDefault();
        setSelectedIndex(prev => Math.max(prev - 1, 0));
        return;
      }

      if (e.ctrlKey || e.metaKey) {
        if (/^[1-9]$/.test(e.key)) {
          e.preventDefault();
          const index = parseInt(e.key) - 1;
          if (index >= 0 && index < results.length) doCopy(results[index], false);
          return;
        }
        if (e.key === 'Enter' && results.length > 0 && selectedIndex >= 0) {
          e.preventDefault();
          doCopy(results[selectedIndex], true);
          return;
        }
        if (e.key === 'e' || e.key === 'E') {
          e.preventDefault();
          if (selected) onEditSnippet(selected);
          return;
        }
        if (e.key === 'n' || e.key === 'N') {
          e.preventDefault();
          createFromQuery();
          return;
        }
      } else {
        if (e.key === 'Enter' && results.length > 0 && selectedIndex >= 0) {
          e.preventDefault();
          doCopy(results[selectedIndex], false);
          return;
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [results, selectedIndex, selected, searchQuery, onClose, onEditSnippet]);

  // Scroll active item into view
  useEffect(() => {
    if (listRef.current && selectedIndex >= 0) {
      const activeElement = listRef.current.children[selectedIndex] as HTMLElement;
      if (activeElement) activeElement.scrollIntoView({ block: 'nearest' });
    }
  }, [selectedIndex]);

  const noResults = !loading && searchQuery.trim().length > 0 && results.length === 0 && !error;

  return (
    <Box className="h-full w-full flex items-center justify-center p-3 select-none">
      <Box
        width="100%"
        height="100%"
        radius="xl"
        shadow="window"
        background="surface"
        border="all"
        overflow="hidden"
        className="raycast-window flex flex-col"
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
              placeholder="搜索 Key、别名、标题、标签或正文 (如: addr, email-work, pg)..."
              aria-label="检索文本片段"
            />
          </HStack>
        </Box>

        {error && (
          <Box paddingX="lg" paddingY="sm" className="status-danger border-b theme-divider flex items-center gap-2" role="alert">
            <AlertCircle className="w-4 h-4 shrink-0" />
            <span className="text-xs">{error}</span>
          </Box>
        )}

        {/* Main Content Area */}
        <Layout direction="row" height="calc(100% - 96px)">
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
                <Badge variant="accent" size="sm">
                  {loading ? '检索中…' : `展示 ${results.length} / 总数 ${total}`}
                </Badge>
              </HStack>
            </Box>

            <Box flex="1" overflow="auto" padding="xs">
              {noResults ? (
                <VStack align="center" justify="center" gap="md" className="py-12 px-4 text-center">
                  <span className="text-xs text-theme-muted">未找到与 &quot;{searchQuery}&quot; 匹配的片段</span>
                  <Button
                    variant="primary"
                    size="sm"
                    icon={<Star className="w-3.5 h-3.5" />}
                    onClick={createFromQuery}
                  >
                    以该查询词为 Key 新建片段 <Kbd>Ctrl+N</Kbd>
                  </Button>
                </VStack>
              ) : (
                <List divided={false} ref={listRef as unknown as React.RefObject<HTMLUListElement>}>
                  {results.map((item, index) => {
                    const isSelected = index === selectedIndex;
                    return (
                      <ListItem
                        key={item.id}
                        active={isSelected}
                        onClick={() => {
                          setSelectedIndex(index);
                          if (index === selectedIndex) doCopy(item, false);
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
                            {item.pinned ? <Star className="w-3.5 h-3.5 fill-current" /> : item.key.slice(0, 2).toUpperCase()}
                          </Box>
                        }
                        title={item.title}
                        meta={
                          <Inline gap="xs" align="center">
                            <Token size="sm">{item.key}</Token>
                            {item.sensitive && (
                              <Badge variant="warning" size="sm">
                                <Lock className="w-3 h-3" /> 敏感
                              </Badge>
                            )}
                          </Inline>
                        }
                        extra={
                          <Inline gap="xs" align="center">
                            {item.tags.slice(0, 1).map(tag => (
                              <Badge key={tag} variant="neutral" size="sm">#{tag}</Badge>
                            ))}
                            {index < 9 && (
                              <span className="text-[11px] font-mono text-theme-muted">
                                Ctrl+{index + 1}
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

          {/* Preview Pane: metadata only — content never leaves storage layer */}
          {showPreview && (
            <LayoutPanel width="50%" height="100%" background="sunken" padding="lg" overflow="auto">
              {selected ? (
                <VStack gap="md">
                  <HStack align="center" justify="space-between">
                    <span className="text-[11px] font-bold uppercase tracking-wider text-theme-muted">
                      片段详情预览
                    </span>
                    <IconButton
                      icon={<Settings className="w-4 h-4" />}
                      ariaLabel="跳转到该片段的管理编辑页"
                      onClick={() => onEditSnippet(selected)}
                    />
                  </HStack>

                  <VStack gap="2xs">
                    <h3 className="text-base font-bold text-theme leading-snug">{selected.title}</h3>
                    <Inline gap="xs" align="center">
                      <span className="text-[11px] text-theme-muted">Key:</span>
                      <Token>{selected.key}</Token>
                      {selected.pinned && <Star className="w-3 h-3 icon-accent fill-current" />}
                    </Inline>
                  </VStack>

                  {selected.aliases.length > 0 && (
                    <Inline gap="xs" align="center">
                      <span className="text-[11px] text-theme-muted">别名:</span>
                      {selected.aliases.map(a => (
                        <Badge key={a} variant="neutral" size="sm">{a}</Badge>
                      ))}
                    </Inline>
                  )}

                  {selected.tags.length > 0 && (
                    <Inline gap="xs" align="center">
                      <span className="text-[11px] text-theme-muted">标签:</span>
                      {selected.tags.map(t => (
                        <Badge key={t} variant="neutral" size="sm">#{t}</Badge>
                      ))}
                    </Inline>
                  )}

                  <VStack gap="2xs">
                    <span className="text-[11px] text-theme-muted font-medium">文本内容:</span>
                    <Box
                      padding="md"
                      radius="md"
                      background="subtle"
                      border="all"
                      className="text-xs text-theme-muted leading-relaxed"
                    >
                      {selected.sensitive ? (
                        <VStack align="center" justify="center" gap="xs" className="py-6 text-center">
                          <Lock className="w-5 h-5 icon-warning" />
                          <span>••••••••••••••••••••</span>
                          <span className="text-[11px] text-theme-disabled">敏感信息默认已隐蔽</span>
                        </VStack>
                      ) : (
                        '正文内容仅在实际复制时写入剪贴板，不在此窗口展示。'
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

        {/* Bottom Status Footer */}
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
                <span className="text-theme-muted">复制并关闭</span>
                <Kbd>↵</Kbd>
              </Inline>
              <span className="text-theme-muted">|</span>
              <Inline gap="3xs" align="center">
                <span className="text-theme-muted">仅复制</span>
                <Kbd>Ctrl+↵</Kbd>
              </Inline>
              <span className="text-theme-muted">|</span>
              <Inline gap="3xs" align="center">
                <span className="text-theme-muted">预览</span>
                <Kbd>Tab</Kbd>
              </Inline>
              <span className="text-theme-muted">|</span>
              <Inline gap="3xs" align="center">
                <span className="text-theme-muted">编辑</span>
                <Kbd>Ctrl+E</Kbd>
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
