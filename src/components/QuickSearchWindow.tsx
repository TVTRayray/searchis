import React, { useEffect, useMemo, useRef, useState } from 'react'
import {
  AlertCircle,
  CheckCircle2,
  ChevronRight,
  Clipboard,
  Copy,
  Keyboard,
  Plus,
  Search,
  Shield,
  Star,
} from 'lucide-react'
import { AppError, SearchResultItem, snippetsApi } from '../api/snippets'

export interface OpenManagerOptions {
  prefillKey?: string
  editId?: string
}

interface QuickSearchWindowProps {
  onClose: () => void
  onOpenManager: (options: OpenManagerOptions) => void
}

const DEFAULT_LIMIT = 20

export const QuickSearchWindow: React.FC<QuickSearchWindowProps> = ({
  onClose,
  onOpenManager,
}) => {
  const [query, setQuery] = useState('')
  const [items, setItems] = useState<SearchResultItem[]>([])
  const [total, setTotal] = useState(0)
  const [selectedIndex, setSelectedIndex] = useState(0)
  const [previewOpen, setPreviewOpen] = useState(true)
  const [copying, setCopying] = useState(false)
  const copyingRef = useRef(false)
  const [error, setError] = useState<AppError | null>(null)
  const [notice, setNotice] = useState('')
  const [loading, setLoading] = useState(false)
  const seqRef = useRef(0)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    inputRef.current?.focus()
  }, [])

  // 防抖检索：忽略过期响应，避免输入乱序覆盖。
  useEffect(() => {
    const seq = ++seqRef.current
    setLoading(true)
    setError(null)
    const timer = window.setTimeout(async () => {
      try {
        const response = await snippetsApi.search(query, DEFAULT_LIMIT)
        if (seq !== seqRef.current) return
        setItems(response.items)
        setTotal(response.total)
        setSelectedIndex(prev =>
          response.items.length === 0
            ? -1
            : Math.max(0, Math.min(prev, response.items.length - 1)),
        )
      } catch (searchError) {
        if (seq === seqRef.current) {
          setItems([])
          setTotal(0)
          setError(searchError as AppError)
        }
      } finally {
        if (seq === seqRef.current) setLoading(false)
      }
    }, 120)
    return () => window.clearTimeout(timer)
  }, [query])

  const selected = useMemo(
    () => (selectedIndex >= 0 ? items[selectedIndex] ?? null : null),
    [items, selectedIndex],
  )

  const move = (delta: number) => {
    if (items.length === 0) return
    setSelectedIndex(prev =>
      Math.max(0, Math.min(items.length - 1, prev + delta)),
    )
  }

  const showError = (e: AppError) => {
    setError(e)
    setNotice('')
  }

  const doCopy = async (item: SearchResultItem, keepOpen: boolean) => {
    if (copyingRef.current) return
    copyingRef.current = true
    setCopying(true)
    setError(null)
    setNotice('')
    try {
      const operationId = crypto.randomUUID()
      const outcome = await snippetsApi.copy(item.id, operationId, keepOpen)
      setItems(prev =>
        prev.map(it =>
          it.id === outcome.snippet.id
            ? { ...it, usageCount: outcome.snippet.usageCount }
            : it,
        ),
      )
      setNotice(
        outcome.counted
          ? `已复制「${item.key}」到系统剪贴板`
          : `已复制「${item.key}」（该操作此前已处理，未重复计数）`,
      )
      if (!keepOpen) {
        window.setTimeout(onClose, 120)
      }
    } catch (copyError) {
      showError(copyError as AppError)
    } finally {
      copyingRef.current = false
      setCopying(false)
    }
  }

  const createFromQuery = async () => {
    const raw = query.trim()
    if (!raw) return
    try {
      const outcome = await snippetsApi.prepareNew(raw)
      onOpenManager({ prefillKey: outcome.normalizedKey })
    } catch (prepareError) {
      showError(prepareError as AppError)
    }
  }

  const openEdit = () => {
    if (!selected) return
    onOpenManager({ editId: selected.id })
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    // 输入法组合态：Esc 用于取消组合，不关闭窗口；J/K/Enter 不拦截。
    if (e.nativeEvent.isComposing) return
    if (e.key === 'Escape') {
      e.preventDefault()
      onClose()
      return
    }
    if (e.key === 'Tab') {
      e.preventDefault()
      setPreviewOpen(prev => !prev)
      return
    }
    if (e.key === 'Enter') {
      if (e.ctrlKey) {
        if (selected) {
          e.preventDefault()
          doCopy(selected, true)
        }
      } else if (selected) {
        e.preventDefault()
        doCopy(selected, false)
      }
      return
    }
    if (e.key === 'ArrowDown' || e.key === 'k' || e.key === 'K') {
      e.preventDefault()
      move(1)
      return
    }
    if (e.key === 'ArrowUp' || e.key === 'j' || e.key === 'J') {
      e.preventDefault()
      move(-1)
      return
    }
    if (e.ctrlKey && !e.altKey && !e.metaKey) {
      const num = Number(e.key)
      if (num >= 1 && num <= 9 && items.length >= num) {
        e.preventDefault()
        doCopy(items[num - 1], false)
        return
      }
      if (e.key === 'e' || e.key === 'E') {
        e.preventDefault()
        openEdit()
        return
      }
      if (e.key === 'n' || e.key === 'N') {
        e.preventDefault()
        createFromQuery()
        return
      }
    }
  }

  const noResults = !loading && query.trim().length > 0 && items.length === 0 && !error

  return (
    <div className="max-w-3xl mx-auto p-4 sm:p-6">
      <div className="rounded-2xl raycast-window overflow-hidden min-h-[520px] flex flex-col">
        <div className="h-11 px-4 theme-titlebar border-b theme-divider flex items-center gap-2">
          <Search className="w-4 h-4 icon-accent" />
          <span className="text-sm font-bold text-theme">Searchis 快速检索</span>
          <span className="ml-auto text-xs text-theme-muted flex items-center gap-3">
            <span><kbd className="raycast-kbd">↵</kbd> 复制并关闭</span>
            <span><kbd className="raycast-kbd">Ctrl+↵</kbd> 仅复制</span>
            <span><kbd className="raycast-kbd">Esc</kbd> 关闭</span>
          </span>
        </div>

        <div className="px-5 pt-5 pb-3">
          <div className="relative">
            <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-theme-muted" />
            <input
              ref={inputRef}
              type="text"
              value={query}
              onChange={e => setQuery(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="输入 Key、别名、标题、标签或正文…"
              aria-label="检索文本片段"
              className="input-theme w-full pl-9 pr-3 py-2.5 rounded-xl text-sm"
            />
          </div>
        </div>

        {error && (
          <div className="mx-5 mb-3 status-danger border rounded-xl p-3 text-xs flex gap-2" role="alert">
            <AlertCircle className="w-4 h-4 shrink-0" />
            <span>{error.message} <code>{error.code}</code></span>
          </div>
        )}
        {notice && !error && (
          <div className="mx-5 mb-3 status-success border rounded-xl p-3 text-xs flex gap-2" role="status">
            <CheckCircle2 className="w-4 h-4 shrink-0" />
            <span>{notice}</span>
          </div>
        )}

        {noResults ? (
          <div className="flex-1 px-5 py-10 text-center space-y-4">
            <p className="text-sm text-theme-muted">
              没有匹配 <code className="px-1.5 py-0.5 rounded badge-neutral font-mono">{query.trim()}</code> 的片段
            </p>
            <button
              onClick={createFromQuery}
              className="btn-primary inline-flex items-center gap-2 px-4 py-2 rounded-xl text-sm font-semibold"
            >
              <Plus className="w-4 h-4" />
              以该查询词作为 Key 新建片段
            </button>
          </div>
        ) : (
          <div className="flex-1 px-5 pb-5 grid grid-cols-1 md:grid-cols-[1fr_240px] gap-4 min-h-[320px]">
            <div className="theme-surface-subtle border theme-divider rounded-xl overflow-hidden flex flex-col">
              <div className="px-3 py-2 border-b theme-divider text-xs text-theme-muted flex items-center justify-between">
                <span>{loading ? '检索中…' : `展示 ${items.length} / 总数 ${total}`}</span>
                <span className="flex items-center gap-1"><Keyboard className="w-3 h-3" /> ↑↓/J/K</span>
              </div>
              <div className="flex-1 overflow-y-auto max-h-[360px] divide-y theme-divider-subtle">
                {items.length === 0 && !loading && (
                  <p className="p-6 text-xs text-theme-muted text-center">输入内容开始检索</p>
                )}
                {items.map((item, index) => (
                  <button
                    key={item.id}
                    onClick={() => {
                      setSelectedIndex(index)
                      if (index === selectedIndex) doCopy(item, false)
                    }}
                    onMouseMove={() => setSelectedIndex(index)}
                    className={`w-full text-left px-3 py-2.5 transition-colors ${
                      index === selectedIndex ? 'raycast-item-active' : ''
                    }`}
                  >
                    <div className="flex items-center gap-2 mb-1">
                      <span className="badge-accent px-1.5 py-0.5 rounded text-xs font-mono font-bold">
                        {item.key}
                      </span>
                      {item.pinned && <Star className="w-3 h-3 icon-accent" />}
                      {item.sensitive && (
                        <span className="badge-warning px-1.5 py-0.5 rounded text-xs flex items-center gap-0.5">
                          <Shield className="w-3 h-3" /> 敏感
                        </span>
                      )}
                      {index === selectedIndex && (
                        <span className="ml-auto flex items-center gap-1 text-xs text-theme-muted">
                          <span className="text-accent"><kbd className="raycast-kbd">↵</kbd> 复制</span>
                        </span>
                      )}
                    </div>
                    <div className="text-sm font-semibold text-theme truncate">{item.title}</div>
                    <div className="flex items-center gap-1.5 mt-1">
                      {item.tags.slice(0, 3).map(tag => (
                        <span key={tag} className="badge-neutral px-1.5 py-0.5 rounded text-xs">#{tag}</span>
                      ))}
                      <span className="ml-auto text-xs text-theme-muted">使用 {item.usageCount} 次</span>
                    </div>
                  </button>
                ))}
              </div>
            </div>

            {previewOpen && (
              <aside className="theme-surface-subtle border theme-divider rounded-xl p-3 space-y-2 hidden md:block">
                <h3 className="text-xs font-bold text-theme-secondary flex items-center gap-1">
                  <ChevronRight className="w-3 h-3 icon-accent" /> 预览 (Tab 切换)
                </h3>
                {selected ? (
                  <>
                    <p className="text-xs text-theme-muted">Key</p>
                    <p className="text-sm font-mono font-bold text-accent break-all">{selected.key}</p>
                    <p className="text-xs text-theme-muted">标题</p>
                    <p className="text-sm text-theme break-words">{selected.title}</p>
                    <p className="text-xs text-theme-muted">别名</p>
                    <p className="text-xs text-theme-secondary break-words">{selected.aliases.join('、') || '—'}</p>
                    <p className="text-xs text-theme-muted">标签</p>
                    <p className="text-xs text-theme-secondary break-words">{selected.tags.join('、') || '—'}</p>
                    <p className="text-xs text-theme-muted">使用次数</p>
                    <p className="text-sm text-theme">{selected.usageCount}</p>
                    {selected.sensitive && (
                      <p className="text-xs badge-warning px-2 py-1 rounded flex items-center gap-1">
                        <Shield className="w-3 h-3" /> 敏感正文默认遮挡，不展示正文
                      </p>
                    )}
                  </>
                ) : (
                  <p className="text-xs text-theme-muted">选择一条结果查看详情</p>
                )}
              </aside>
            )}
          </div>
        )}

        <div className="px-5 py-3 theme-titlebar border-t theme-divider flex items-center justify-between text-xs text-theme-muted">
          <div className="flex items-center gap-3 flex-wrap">
            <span className="flex items-center gap-1"><kbd className="raycast-kbd">Ctrl+1~9</kbd> 直接选择</span>
            <span className="flex items-center gap-1"><kbd className="raycast-kbd">Ctrl+E</kbd> 编辑当前</span>
            <span className="flex items-center gap-1"><kbd className="raycast-kbd">Ctrl+N</kbd> 以查询词新建</span>
          </div>
          <button onClick={onClose} className="btn-secondary px-3 py-1.5 rounded-lg flex items-center gap-1">
            <Clipboard className="w-3.5 h-3.5" /> 管理窗口
          </button>
        </div>

        {copying && (
          <div className="absolute bottom-4 right-4 btn-primary rounded-lg px-3 py-2 text-xs font-semibold shadow-lg">
            <Copy className="w-3.5 h-3.5 inline mr-1" /> 写入剪贴板…
          </div>
        )}
      </div>
    </div>
  )
}
