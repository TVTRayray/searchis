import React, { useEffect, useRef, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { AlertCircle, Lock, Star } from 'lucide-react';
import { SearchResultItem, settingsApi, snippetsApi } from '../api/snippets';
import { useTheme } from './ThemeProvider';
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList, CommandShortcut } from './ui/command';

interface QuickSearchWindowProps {
  onClose: () => void;
  onEditSnippet: (item: SearchResultItem) => void;
  onCreateNewSnippet: (prefillKey?: string) => void;
  maxResultsCount?: number;
}

const DEFAULT_LIMIT = 20;

const highlight = (text: string, query: string) => {
  const needle = query.trim();
  const start = text.toLocaleLowerCase().indexOf(needle.toLocaleLowerCase());
  if (!needle || start < 0) return text;
  return <>{text.slice(0, start)}<mark>{text.slice(start, start + needle.length)}</mark>{text.slice(start + needle.length)}</>;
};

export const QuickSearchWindow: React.FC<QuickSearchWindowProps> = ({
  onClose, onEditSnippet, onCreateNewSnippet, maxResultsCount = DEFAULT_LIMIT,
}) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [results, setResults] = useState<SearchResultItem[]>([]);
  const [total, setTotal] = useState(0);
  const [selectedIndex, setSelectedIndex] = useState(-1);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const seqRef = useRef(0);
  const themeSyncSeqRef = useRef(0);
  const backendThemeRef = useRef<string | null>(null);
  const visibleRef = useRef(false);
  const copyingRef = useRef(false);
  const closingRef = useRef(false);
  const { setTheme } = useTheme();

  const applyDomTheme = (mode: string) => {
    const dark = mode === 'dark' || (mode === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
    document.documentElement.dataset.theme = dark ? 'dark' : 'light';
    document.documentElement.classList.toggle('dark', dark);
  };
  const applyTheme = (mode: string) => {
    if (mode !== 'light' && mode !== 'dark' && mode !== 'system') return;
    backendThemeRef.current = mode;
    setTheme(mode);
    applyDomTheme(mode);
    // ThemeProvider applies its effect after the state update; re-apply after that commit
    // so a stale initial window theme cannot win the first visible frame.
    window.setTimeout(() => {
      if (backendThemeRef.current === mode) applyDomTheme(mode);
    }, 0);
  };
  const reconcileTheme = () => {
    if (backendThemeRef.current) applyTheme(backendThemeRef.current);
  };
  const syncTheme = () => {
    const seq = ++themeSyncSeqRef.current;
    return settingsApi.get().then(({ settings }) => {
      if (seq === themeSyncSeqRef.current) applyTheme(settings.theme);
    }).catch(() => {});
  };
  const close = () => {
    if (closingRef.current) return;
    closingRef.current = true;
    onClose();
  };

  useEffect(() => {
    let unlistenFocus: (() => void) | undefined;
    let currentWindow: ReturnType<typeof getCurrentWindow> | undefined;
    const activate = () => {
      closingRef.current = false;
      void syncTheme();
      setSearchQuery(''); setResults([]); setTotal(0); setSelectedIndex(-1); setError(null);
      inputRef.current?.focus();
    };
    try { currentWindow = getCurrentWindow(); } catch { /* browser preview */ }
    activate();
    // 主题变更即时同步：管理窗口写 localStorage 后，隐藏中的检索 WebView 通过 storage 事件立即应用。
    const onStorage = (event: StorageEvent) => {
      if (event.key === 'searchis_config_v1' && event.newValue) void syncTheme();
    };
    const refreshVisibleTheme = () => {
      void currentWindow?.isVisible().then(visible => {
        if (!visible) { visibleRef.current = false; return; }
        if (visibleRef.current) return;
        visibleRef.current = true;
        reconcileTheme();
        void syncTheme();
      }).catch(() => {});
    };
    // Some WebKit/KWin paths show the pre-created hidden window without emitting a focus event.
    // Poll visibility only to catch that transition; settings are fetched once per show.
    const themeTimer = window.setInterval(refreshVisibleTheme, 100);
    window.addEventListener('storage', onStorage);
    if (currentWindow) {
      currentWindow.onFocusChanged(({ payload }) => {
        if (!payload) { close(); return; }
        activate();
      }).then(fn => {
        unlistenFocus = fn;
        void currentWindow?.isFocused().then(focused => { if (focused) activate(); }).catch(() => {});
      }).catch(() => {});
    }
    return () => {
      unlistenFocus?.();
      window.clearInterval(themeTimer);
      window.removeEventListener('storage', onStorage);
    };
  }, []);

  useEffect(() => {
    const seq = ++seqRef.current;
    setLoading(true); setError(null);
    const timer = window.setTimeout(async () => {
      try {
        const response = await snippetsApi.search(searchQuery, maxResultsCount);
        if (seq !== seqRef.current) return;
        setResults(response.items); setTotal(response.total);
        setSelectedIndex(response.items.length ? 0 : -1);
      } catch (searchError) {
        if (seq === seqRef.current) {
          setResults([]); setTotal(0); setSelectedIndex(-1);
          setError((searchError as { message?: string }).message ?? '检索失败');
        }
      } finally { if (seq === seqRef.current) setLoading(false); }
    }, 120);
    return () => window.clearTimeout(timer);
  }, [searchQuery, maxResultsCount]);

  const selected = results[selectedIndex] ?? null;
  const doPaste = async (item: SearchResultItem, keepOpen: boolean) => {
    if (copyingRef.current) return;
    copyingRef.current = true; setError(null);
    try {
      const outcome = await snippetsApi.executePaste(item.id, crypto.randomUUID(), !keepOpen);
      setResults(current => current.map(result => result.id === outcome.snippet.id ? { ...result, usageCount: outcome.snippet.usageCount } : result));
      if (!keepOpen) close();
    } catch (pasteError) {
      setError((pasteError as { message?: string }).message ?? '操作失败');
    } finally { copyingRef.current = false; }
  };
  const createFromQuery = async () => {
    const raw = searchQuery.trim();
    if (!raw) return;
    try { closingRef.current = true; onCreateNewSnippet((await snippetsApi.prepareNew(raw)).normalizedKey); }
    catch (prepareError) { setError((prepareError as { message?: string }).message ?? '无法生成 Key'); }
  };
  const moveSelection = (delta: number) => setSelectedIndex(index => Math.max(0, Math.min(index + delta, results.length - 1)));
  const onKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.nativeEvent.isComposing) return;
    if (event.key === 'Escape') { event.preventDefault(); onClose(); return; }
    if (event.key === 'ArrowDown' || event.key === 'j' || event.key === 'J') { event.preventDefault(); moveSelection(1); return; }
    if (event.key === 'ArrowUp' || event.key === 'k' || event.key === 'K') { event.preventDefault(); moveSelection(-1); return; }
    if (!(event.ctrlKey || event.metaKey)) {
      if (event.key === 'Enter' && selected) { event.preventDefault(); void doPaste(selected, false); }
      return;
    }
    if (/^[1-9]$/.test(event.key)) {
      event.preventDefault(); const item = results[Number(event.key) - 1]; if (item) void doPaste(item, false); return;
    }
    if (event.key === 'Enter' && selected) { event.preventDefault(); void doPaste(selected, true); return; }
    if ((event.key === 'e' || event.key === 'E') && selected) { event.preventDefault(); closingRef.current = true; onEditSnippet(selected); return; }
    if (event.key === 'n' || event.key === 'N') { event.preventDefault(); void createFromQuery(); }
  };
  const pinned = results.filter(item => item.pinned);
  const ordinary = results.filter(item => !item.pinned);
  const renderItem = (item: SearchResultItem) => {
    const index = results.indexOf(item);
    return <CommandItem key={item.id} value={item.id} data-snippet-id={item.id} onSelect={() => setSelectedIndex(index)} onClick={() => {
      if (selectedIndex === index) void doPaste(item, false);
    }}>
      <span className={`quick-key ${selectedIndex === index ? 'quick-key-selected' : ''}`} aria-hidden>{item.pinned ? <Star className="size-3.5 fill-current" /> : item.key.slice(0, 2).toUpperCase()}</span>
      <div className="min-w-0 flex-1" style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-2xs)' }}><span className="truncate text-xs font-semibold">{item.title}</span><div className="flex items-center gap-1.5"><span className="truncate font-mono text-[11px] text-[color:var(--quick-muted)]">{highlight(item.key, searchQuery)}</span>{item.sensitive && <span className="quick-sensitive"><Lock className="size-3" />敏感</span>}{item.usageCount > 0 && <span className="quick-usage">使用 {item.usageCount} 次</span>}{item.tags.slice(0, 1).map(tag => <span className="quick-tag" key={tag}>#{tag}</span>)}</div></div>
      {index < 9 && <CommandShortcut>Ctrl+{index + 1}</CommandShortcut>}
    </CommandItem>;
  };

  return <main className="quick-search-shell" aria-label="快速检索">
    <section className="quick-search-window" role="dialog" aria-modal="true" aria-label="快速检索窗口">
      <Command shouldFilter={false} value={selected?.id ?? ''} onValueChange={id => setSelectedIndex(results.findIndex(item => item.id === id))}>
        <CommandInput ref={inputRef} value={searchQuery} onValueChange={setSearchQuery} onKeyDown={onKeyDown} placeholder="搜索 Key、别名、标题、标签或正文…" aria-label="检索文本片段" />
        {error && <div className="quick-error" role="alert"><AlertCircle className="size-4" aria-hidden />{error}</div>}
        <CommandList className="min-h-0 flex-1">
          <CommandEmpty>{!loading && <div className="flex flex-col items-center justify-center gap-3 px-4 text-center"><span className="text-xs text-[color:var(--quick-muted)]">{searchQuery ? '没有匹配的片段' : '暂无片段数据'}</span>{searchQuery && <button type="button" className="btn-primary inline-flex items-center justify-center gap-1.5 cursor-pointer transition-all px-2.5 py-1 text-xs rounded-md" onClick={() => void createFromQuery()}>以该查询词为 Key 新建片段 <kbd className="raycast-kbd">Ctrl+N</kbd></button>}</div>}</CommandEmpty>
          {pinned.length > 0 && <CommandGroup heading={`固定片段 · ${pinned.length}`}>{pinned.map(renderItem)}</CommandGroup>}
          {ordinary.length > 0 && <CommandGroup heading={pinned.length ? `全部结果 · ${ordinary.length}` : `匹配片段结果 · ${ordinary.length} 条`}>{ordinary.map(renderItem)}</CommandGroup>}
        </CommandList>
      </Command>
      <footer className="quick-footer"><span><b>Searchis</b> · {loading ? '检索中…' : `展示 ${results.length} / 总数 ${total}`}</span><span>粘贴 <kbd className="raycast-kbd">↵</kbd>　仅复制 <kbd className="raycast-kbd">Ctrl+↵</kbd>　关闭 <kbd className="raycast-kbd">Esc</kbd></span></footer>
    </section>
  </main>;
};
