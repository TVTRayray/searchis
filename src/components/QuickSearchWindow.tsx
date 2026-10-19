import React, { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import {
  AlertCircle,
  Braces,
  Database,
  FileCode,
  FileText,
  Info,
  LayoutGrid,
  Lock,
  Star,
  Terminal,
} from "lucide-react";
import {
  SearchResultItem,
  settingsApi,
  snippetsApi,
  templatesApi,
} from "../api/snippets";
import { FileTypeIcon, fileCategory } from "./ui/FileTypeIcon";
import { useTheme } from "./ThemeProvider";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandShortcut,
} from "./ui/command";

interface QuickSearchWindowProps {
  onClose: () => void;
  onEditSnippet: (item: SearchResultItem) => void;
  onCreateNewSnippet: (prefillKey?: string) => void;
  onOpenManager: () => void;
  maxResultsCount?: number;
}

const DEFAULT_LIMIT = 20;
const AUTO_PASTE_OFF = "自动粘贴已在设置中关闭";

/** 底栏提示随选中条目类型切换（PRD FR-PCK-02 v0.4）。 */
const TEXT_HINTS: ReadonlyArray<readonly [string, string]> = [
  ["复制", "↵"],
  ["粘贴", "Ctrl+↵"],
  ["编辑", "Ctrl+E"],
  ["关闭", "Esc"],
];
const FILE_HINTS: ReadonlyArray<readonly [string, string]> = [
  ["带格式复制", "↵"],
  ["粘贴", "Ctrl+↵"],
  ["导出", "Shift+↵"],
  ["打开", "Ctrl+O"],
  ["编辑", "Ctrl+E"],
  ["关闭", "Esc"],
];

const highlight = (text: string, query: string) => {
  const needle = query.trim();
  const start = text.toLocaleLowerCase().indexOf(needle.toLocaleLowerCase());
  if (!needle || start < 0) return text;
  return (
    <>
      {text.slice(0, start)}
      <mark>{text.slice(start, start + needle.length)}</mark>
      {text.slice(start + needle.length)}
    </>
  );
};

/** Q-A 左侧类型图标：按 language 区分，文本片段默认 FileText。 */
const languageIcon = (language: string) => {
  const props = { className: "size-4", strokeWidth: 1.5, "aria-hidden": true } as const;
  switch (language) {
    case "bash":
      return <Terminal {...props} />;
    case "sql":
      return <Database {...props} />;
    case "json":
      return <Braces {...props} />;
    case "yaml":
    case "toml":
    case "ini":
      return <FileCode {...props} />;
    default:
      return <FileText {...props} />;
  }
};

export const QuickSearchWindow: React.FC<QuickSearchWindowProps> = ({
  onClose,
  onEditSnippet,
  onCreateNewSnippet,
  onOpenManager,
  maxResultsCount = DEFAULT_LIMIT,
}) => {
  const [searchQuery, setSearchQuery] = useState("");
  const [results, setResults] = useState<SearchResultItem[]>([]);
  const [total, setTotal] = useState(0);
  const [selectedIndex, setSelectedIndex] = useState(-1);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const seqRef = useRef(0);
  const themeSyncSeqRef = useRef(0);
  const backendThemeRef = useRef<string | null>(null);
  const visibleRef = useRef(false);
  const copyingRef = useRef(false);
  const closingRef = useRef(false);
  const isOpenRef = useRef(false);
  const lastThemeSyncAtRef = useRef(0);
  const { setTheme } = useTheme();

  const applyDomTheme = (mode: string) => {
    const dark =
      mode === "dark" ||
      (mode === "system" &&
        window.matchMedia("(prefers-color-scheme: dark)").matches);
    document.documentElement.dataset.theme = dark ? "dark" : "light";
    document.documentElement.classList.toggle("dark", dark);
  };
  const applyTheme = (mode: string) => {
    if (mode !== "light" && mode !== "dark" && mode !== "system") return;
    backendThemeRef.current = mode;
    setTheme(mode);
    applyDomTheme(mode);
    window.setTimeout(() => {
      if (backendThemeRef.current === mode) applyDomTheme(mode);
    }, 0);
  };
  const reconcileTheme = () => {
    if (backendThemeRef.current) applyTheme(backendThemeRef.current);
  };
  const syncTheme = () => {
    const seq = ++themeSyncSeqRef.current;
    return settingsApi
      .get()
      .then(({ settings }) => {
        if (seq === themeSyncSeqRef.current) applyTheme(settings.theme);
      })
      .catch(() => {});
  };
  const close = () => {
    if (closingRef.current) return;
    closingRef.current = true;
    isOpenRef.current = false;
    setSearchQuery("");
    setError(null);
    setNotice(null);
    onClose();
  };
  /** 有提示时让用户看清楚再关闭（自动粘贴被设置关闭、公式转数值等）。 */
  const closeWithNotices = (messages: string[]) => {
    if (messages.length === 0) {
      close();
      return;
    }
    setNotice(messages.join(" "));
    window.setTimeout(close, 1100);
  };

  const fetchResults = useCallback(
    async (query: string) => {
      const seq = ++seqRef.current;
      setLoading(true);
      setError(null);
      try {
        const response = await snippetsApi.search(query, maxResultsCount);
        if (seq !== seqRef.current) return;
        setResults(response.items);
        setTotal(response.total);
        setSelectedIndex(response.items.length ? 0 : -1);
      } catch (searchError) {
        if (seq === seqRef.current) {
          setResults([]);
          setTotal(0);
          setSelectedIndex(-1);
          setError((searchError as { message?: string }).message ?? "检索失败");
        }
      } finally {
        if (seq === seqRef.current) setLoading(false);
      }
    },
    [maxResultsCount],
  );

  useEffect(() => {
    let unlistenFocus: (() => void) | undefined;
    let unlistenFocusEvent: (() => void) | undefined;
    let currentWindow: ReturnType<typeof getCurrentWindow> | undefined;
    let hasBeenFocused = false;
    let focusGraceTimer: number | undefined;

    const focusInput = () => {
      if (inputRef.current && document.activeElement !== inputRef.current) {
        inputRef.current.focus({ preventScroll: true });
      }
    };

    // 多个事件源（focus / visibilitychange / onFocusChanged / 轮询）在窗口
    // 刚显示时可能在短时间内集中触发多次，避免每次都发起 IPC 请求
    // 重新拉取主题，减少主线程竞争，让键盘输入更快地得到响应。
    const throttledSyncTheme = () => {
      const now = Date.now();
      if (now - lastThemeSyncAtRef.current < 250) return;
      lastThemeSyncAtRef.current = now;
      void syncTheme();
    };

    const openAndRefresh = () => {
      const now = Date.now();
      closingRef.current = false;
      hasBeenFocused = false;
      window.clearTimeout(focusGraceTimer);
      focusGraceTimer = window.setTimeout(() => {
        hasBeenFocused = true;
      }, 50);

      lastThemeSyncAtRef.current = now;
      void syncTheme();
      setError(null);
      void fetchResults("");
      focusInput();
    };

    const activate = () => {
      if (!isOpenRef.current) {
        isOpenRef.current = true;
        openAndRefresh();
      } else {
        throttledSyncTheme();
        focusInput();
      }
    };

    try {
      currentWindow = getCurrentWindow();
    } catch {
      /* browser preview */
    }
    activate();

    const onStorage = (event: StorageEvent) => {
      if (event.key === "searchis_config_v1" && event.newValue)
        void syncTheme();
    };
    const refreshVisibleTheme = () => {
      void currentWindow
        ?.isVisible()
        .then((visible) => {
          if (!visible) {
            const wasVisible = visibleRef.current;
            visibleRef.current = false;
            isOpenRef.current = false;
            if (wasVisible) {
              setSearchQuery("");
              setError(null);
            }
            return;
          }
          if (visibleRef.current) return;
          visibleRef.current = true;
          reconcileTheme();
          void syncTheme();
          activate();
        })
        .catch(() => {});
    };

    const themeTimer = window.setInterval(refreshVisibleTheme, 100);
    window.addEventListener("storage", onStorage);
    window.addEventListener("focus", activate);
    window.addEventListener("click", focusInput);

    const onVisibilityChange = () => {
      if (document.visibilityState === "visible") activate();
    };
    document.addEventListener("visibilitychange", onVisibilityChange);

    listen("focus-input", () => {
      focusInput();
    })
      .then((fn) => {
        unlistenFocusEvent = fn;
      })
      .catch(() => {});

    if (currentWindow) {
      currentWindow
        .onFocusChanged(({ payload }) => {
          if (!payload) {
            isOpenRef.current = false;
            if (hasBeenFocused && !closingRef.current) {
              close();
            }
            return;
          }
          hasBeenFocused = true;
          activate();
        })
        .then((fn) => {
          unlistenFocus = fn;
          void currentWindow
            ?.isFocused()
            .then((focused) => {
              if (focused) activate();
            })
            .catch(() => {});
        })
        .catch(() => {});
    }

    return () => {
      window.clearTimeout(focusGraceTimer);
      unlistenFocus?.();
      unlistenFocusEvent?.();
      window.clearInterval(themeTimer);
      window.removeEventListener("storage", onStorage);
      window.removeEventListener("focus", activate);
      window.removeEventListener("click", focusInput);
      document.removeEventListener("visibilitychange", onVisibilityChange);
    };
  }, [fetchResults]);

  useEffect(() => {
    setNotice(null);
    void fetchResults(searchQuery);
  }, [searchQuery, fetchResults]);

  // 自动滚动选中项到可见区域
  useEffect(() => {
    if (selectedIndex < 0 || results.length === 0) return;

    const selectedItem = results[selectedIndex];
    if (!selectedItem) return;

    // 使用 requestAnimationFrame 确保 DOM 已更新
    requestAnimationFrame(() => {
      const element = document.querySelector(
        `[data-snippet-id="${selectedItem.id}"]`,
      );
      if (element) {
        element.scrollIntoView({ behavior: "smooth", block: "nearest" });
      }
    });
  }, [selectedIndex, results]);

  const selected = results[selectedIndex] ?? null;
  const doPaste = async (item: SearchResultItem, keepOpen: boolean) => {
    if (copyingRef.current) return;
    copyingRef.current = true;
    setError(null);
    try {
      const outcome = await snippetsApi.executePaste(
        item.id,
        crypto.randomUUID(),
        !keepOpen,
      );
      setResults((current) =>
        current.map((result) =>
          result.id === outcome.snippet.id
            ? { ...result, usageCount: outcome.snippet.usageCount }
            : result,
        ),
      );
      if (!keepOpen) {
        closeWithNotices(outcome.autoPasteDisabled ? [AUTO_PASTE_OFF] : []);
      }
    } catch (pasteError) {
      setError((pasteError as { message?: string }).message ?? "操作失败");
    } finally {
      copyingRef.current = false;
    }
  };

  const doCopy = async (item: SearchResultItem) => {
    if (copyingRef.current) return;
    copyingRef.current = true;
    setError(null);
    try {
      const outcome = await snippetsApi.copy(
        item.id,
        crypto.randomUUID(),
        false,
      );
      setResults((current) =>
        current.map((result) =>
          result.id === outcome.snippet.id
            ? { ...result, usageCount: outcome.snippet.usageCount }
            : result,
        ),
      );
      close();
    } catch (copyError) {
      setError((copyError as { message?: string }).message ?? "复制失败");
    } finally {
      copyingRef.current = false;
    }
  };

  /** 文件模板：Enter 带格式复制；Ctrl+Enter / Ctrl+数字 带格式复制并自动粘贴。 */
  const doFormatCopy = async (item: SearchResultItem, autoPaste: boolean) => {
    if (copyingRef.current) return;
    copyingRef.current = true;
    setError(null);
    setNotice(null);
    try {
      const outcome = await templatesApi.formatCopy(
        item.id,
        crypto.randomUUID(),
        autoPaste,
      );
      setResults((current) =>
        current.map((result) =>
          result.id === outcome.snippet.id
            ? { ...result, usageCount: outcome.snippet.usageCount }
            : result,
        ),
      );
      closeWithNotices([
        ...outcome.notes,
        ...(outcome.autoPasteDisabled ? [AUTO_PASTE_OFF] : []),
      ]);
    } catch (copyError) {
      setError((copyError as { message?: string }).message ?? "带格式复制失败");
    } finally {
      copyingRef.current = false;
    }
  };

  /** Shift+Enter：导出副本；成功后窗口保持打开并显示路径。 */
  const doExport = async (item: SearchResultItem) => {
    if (copyingRef.current) return;
    copyingRef.current = true;
    setError(null);
    setNotice(null);
    try {
      const outcome = await templatesApi.export(item.id, crypto.randomUUID());
      if (outcome.counted) {
        setResults((current) =>
          current.map((result) =>
            result.id === item.id
              ? { ...result, usageCount: result.usageCount + 1 }
              : result,
          ),
        );
      }
      setNotice(outcome.path ? `已导出到 ${outcome.path}` : "已导出");
    } catch (exportError) {
      setError((exportError as { message?: string }).message ?? "导出失败");
    } finally {
      copyingRef.current = false;
    }
  };

  /** Ctrl+O：用默认软件打开，不计数，成功后关闭。 */
  const doOpen = async (item: SearchResultItem) => {
    if (copyingRef.current) return;
    copyingRef.current = true;
    setError(null);
    setNotice(null);
    try {
      await templatesApi.open(item.id);
      close();
    } catch (openError) {
      setError((openError as { message?: string }).message ?? "无法打开文件");
    } finally {
      copyingRef.current = false;
    }
  };

  const handleItemContextMenu = (
    event: React.MouseEvent,
    item: SearchResultItem,
  ) => {
    event.preventDefault();
    event.stopPropagation();
    closingRef.current = true;
    onEditSnippet(item);
  };

  const createFromQuery = async () => {
    const raw = searchQuery.trim();
    if (!raw) return;
    try {
      const prepared = await snippetsApi.prepareNew(raw);
      closingRef.current = true;
      onCreateNewSnippet(prepared.normalizedKey);
    } catch (prepareError) {
      closingRef.current = false;
      setError(
        (prepareError as { message?: string }).message ?? "无法生成 Key",
      );
    }
  };
  const moveSelection = (delta: number) =>
    setSelectedIndex((index) =>
      Math.max(0, Math.min(index + delta, results.length - 1)),
    );
  const onKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.nativeEvent.isComposing) return;
    if (event.key === "Escape") {
      event.preventDefault();
      close();
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      moveSelection(1);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      moveSelection(-1);
      return;
    }
    if (!(event.ctrlKey || event.metaKey)) {
      if (event.key === "Enter" && selected) {
        event.preventDefault();
        if (event.shiftKey) {
          if (selected.kind === "file") void doExport(selected);
        } else if (selected.kind === "file") {
          void doFormatCopy(selected, false);
        } else {
          void doCopy(selected);
        }
      }
      return;
    }
    if (/^[1-9]$/.test(event.key)) {
      event.preventDefault();
      const item = results[Number(event.key) - 1];
      if (!item) return;
      if (item.kind === "file") void doFormatCopy(item, true);
      else void doPaste(item, false);
      return;
    }
    if (event.key === "Enter" && selected) {
      event.preventDefault();
      if (selected.kind === "file") void doFormatCopy(selected, true);
      else void doPaste(selected, false);
      return;
    }
    if ((event.key === "o" || event.key === "O") && selected) {
      event.preventDefault();
      if (selected.kind === "file") void doOpen(selected);
      return;
    }
    if ((event.key === "e" || event.key === "E") && selected) {
      event.preventDefault();
      closingRef.current = true;
      onEditSnippet(selected);
      return;
    }
    if (event.key === "n" || event.key === "N") {
      event.preventDefault();
      void createFromQuery();
    }
  };
  const pinned = results.filter((item) => item.pinned);
  const ordinary = results.filter((item) => !item.pinned);
  const renderItem = (item: SearchResultItem) => {
    const index = results.indexOf(item);
    return (
      <CommandItem
        key={item.id}
        value={item.id}
        data-snippet-id={item.id}
        onSelect={() => {
          setSelectedIndex(index);
          if (item.kind === "file") void doFormatCopy(item, false);
          else void doCopy(item);
        }}
        onContextMenu={(event) => handleItemContextMenu(event, item)}
      >
        <span
          className={`quick-key ${item.kind === "file" ? "file-mark " : ""}${selectedIndex === index ? (item.pinned ? "quick-key-pinned-selected" : "quick-key-selected") : item.pinned ? "quick-key-pinned" : ""}`}
          data-kind={item.kind}
          data-language={item.language}
          data-category={item.kind === "file" ? fileCategory(item.fileExt) : undefined}
          aria-hidden
        >
          {item.kind === "file" ? (
            <FileTypeIcon ext={item.fileExt} className="size-4" />
          ) : item.pinned ? (
            <Star className="size-3.5 fill-current" strokeWidth={1.5} />
          ) : (
            languageIcon(item.language)
          )}
        </span>
        <div className="min-w-0 flex-1 flex flex-col gap-1">
          <span className="quick-search-title truncate">
            {highlight(item.title, searchQuery)}
          </span>
          <div className="quick-search-subline flex items-center gap-1.5 min-w-0">
            <span className="quick-search-key truncate">
              {highlight(item.key, searchQuery)}
            </span>
            {item.groupName && (
              <>
                <span className="quick-sep" aria-hidden>·</span>
                <span className="quick-group truncate">{item.groupName}</span>
              </>
            )}
            <span className="quick-sep" aria-hidden>·</span>
            <span className="quick-usage">{item.usageCount} 次</span>
          </div>
        </div>
        <div className="quick-badges">
          {item.kind === "file" && item.pinned && (
            <Star
              className="size-3 fill-current quick-pin-mark"
              strokeWidth={1.5}
              data-testid="pin-mark"
              aria-label="已固定"
            />
          )}
          {item.sensitive && (
            <span className="quick-sensitive">
              <Lock className="size-3" strokeWidth={1.5} />
              敏感
            </span>
          )}
          {item.kind === "file" && item.fileExt && (
            <span
              className="quick-language-badge quick-file-badge"
              data-testid="file-badge"
            >
              {item.fileExt.toUpperCase()}
            </span>
          )}
          {item.kind !== "file" && item.language !== "plain" && (
            <span className="quick-language-badge" data-testid="language-badge">
              {item.language.toUpperCase()}
            </span>
          )}
        </div>
        {index < 9 && (
          <CommandShortcut>
            <kbd className="raycast-kbd">Ctrl+{index + 1}</kbd>
          </CommandShortcut>
        )}
      </CommandItem>
    );
  };

  return (
    <main className="quick-search-shell" aria-label="快速检索">
      <section
        className="quick-search-window"
        role="dialog"
        aria-modal="true"
        aria-label="快速检索窗口"
      >
        <Command
          shouldFilter={false}
          disablePointerSelection={true}
          value={selected?.id ?? ""}
          onValueChange={(id) =>
            setSelectedIndex(results.findIndex((item) => item.id === id))
          }
        >
          <CommandInput
            ref={inputRef}
            value={searchQuery}
            onValueChange={setSearchQuery}
            onKeyDown={onKeyDown}
            placeholder="搜索 Key、别名、标题、标签或正文…"
            aria-label="检索文本片段"
            autoFocus
          />
          {error && (
            <div className="quick-error" role="alert">
              <AlertCircle className="size-4" strokeWidth={1.5} aria-hidden />
              {error}
            </div>
          )}
          {notice && (
            <div className="quick-notice" role="status" data-testid="quick-notice">
              <Info className="size-4" strokeWidth={1.5} aria-hidden />
              <span className="truncate">{notice}</span>
            </div>
          )}
          <CommandList className="min-h-0 flex-1">
            <CommandEmpty>
              {!loading && (
                <div className="flex flex-col items-center justify-center gap-3 px-4 text-center">
                  <span className="quick-search-empty-text">
                    {searchQuery ? "没有匹配的片段" : "暂无片段数据"}
                  </span>
                  {searchQuery && (
                    <button
                      type="button"
                      className="btn-primary"
                      onClick={() => void createFromQuery()}
                    >
                      以该查询词为 Key 新建片段{" "}
                      <kbd className="raycast-kbd">Ctrl+N</kbd>
                    </button>
                  )}
                </div>
              )}
            </CommandEmpty>
            {pinned.length > 0 && (
              <CommandGroup heading={`固定片段 · ${pinned.length}`}>
                {pinned.map(renderItem)}
              </CommandGroup>
            )}
            {ordinary.length > 0 && (
              <CommandGroup
                heading={
                  pinned.length
                    ? `全部结果 · ${ordinary.length}`
                    : `匹配片段结果 · ${ordinary.length} 条`
                }
              >
                {ordinary.map(renderItem)}
              </CommandGroup>
            )}
          </CommandList>
        </Command>
        <footer className="quick-footer">
          <span className="flex items-center gap-2">
            <span>
              <b>Searchis</b> ·{" "}
              {loading ? "检索中…" : `展示 ${results.length} / 总数 ${total}`}
            </span>
            <button
              type="button"
              className="quick-footer-button"
              onClick={() => {
                closingRef.current = true;
                onOpenManager();
              }}
              aria-label="打开管理界面"
              title="打开管理界面"
            >
              <LayoutGrid className="size-3.5" strokeWidth={1.5} />
            </button>
          </span>
          <span className="quick-hints" data-testid="quick-hints" data-kind={selected?.kind ?? "text"}>
            {(selected?.kind === "file" ? FILE_HINTS : TEXT_HINTS).map(
              ([label, keys]) => (
                <span key={label}>
                  {label} <kbd className="raycast-kbd">{keys}</kbd>
                </span>
              ),
            )}
          </span>
        </footer>
      </section>
    </main>
  );
};
