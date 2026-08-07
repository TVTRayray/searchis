import React, { useEffect, useMemo, useState } from 'react'
import { AlertCircle, CheckCircle2, Database, Edit3, Plus, Save, Search, ShieldAlert } from 'lucide-react'
import {
  AppError,
  PersistedSnippet,
  SnippetFields,
  snippetsApi,
} from './api/snippets'
import { OpenManagerOptions, QuickSearchWindow } from './components/QuickSearchWindow'

const EMPTY_FIELDS: SnippetFields = {
  key: '',
  title: '',
  content: '',
  aliases: [],
  tags: [],
  sensitive: false,
  pinned: false,
}

const splitList = (value: string) => value.split(',').map(item => item.trim()).filter(Boolean)

export const App: React.FC = () => {
  const [view, setView] = useState<'picker' | 'manager'>('picker')
  const [snippets, setSnippets] = useState<PersistedSnippet[]>([])
  const [selected, setSelected] = useState<PersistedSnippet | null>(null)
  const [fields, setFields] = useState<SnippetFields>(EMPTY_FIELDS)
  const [aliasesText, setAliasesText] = useState('')
  const [tagsText, setTagsText] = useState('')
  const [requestId, setRequestId] = useState(() => crypto.randomUUID())
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<AppError | null>(null)
  const [notice, setNotice] = useState('')
  const [sensitiveRevealed, setSensitiveRevealed] = useState(false)

  useEffect(() => {
    snippetsApi.list()
      .then(items => {
        setSnippets(items)
        if (items[0] && view === 'manager') selectSnippet(items[0])
      })
      .catch(setError)
      .finally(() => setLoading(false))
  }, [])

  const contentBytes = useMemo(() => new TextEncoder().encode(fields.content).length, [fields.content])

  const refreshSnippets = async () => {
    try {
      const items = await snippetsApi.list()
      setSnippets(items)
    } catch (refreshError) {
      setError(refreshError as AppError)
    }
  }

  const openManager = (options?: OpenManagerOptions) => {
    if (options?.editId) {
      const target = snippets.find(s => s.id === options.editId)
      if (target) selectSnippet(target)
    } else if (options?.prefillKey) {
      startCreate()
      setFields(prev => ({ ...prev, key: options.prefillKey! }))
    }
    refreshSnippets()
    setView('manager')
  }

  const selectSnippet = (snippet: PersistedSnippet) => {
    setSelected(snippet)
    setFields({
      key: snippet.key,
      title: snippet.title,
      content: snippet.content,
      aliases: snippet.aliases,
      tags: snippet.tags,
      sensitive: snippet.sensitive,
      pinned: snippet.pinned,
    })
    setAliasesText(snippet.aliases.join(', '))
    setTagsText(snippet.tags.join(', '))
    setSensitiveRevealed(false)
    setError(null)
    setNotice('')
  }

  const startCreate = () => {
    setSelected(null)
    setFields(EMPTY_FIELDS)
    setAliasesText('')
    setTagsText('')
    setRequestId(crypto.randomUUID())
    setSensitiveRevealed(true)
    setError(null)
    setNotice('')
  }

  const reloadSelected = async () => {
    if (!selected || !window.confirm('重新载入会放弃当前未保存输入。是否继续？')) return
    setSaving(true)
    setError(null)
    try {
      const fresh = await snippetsApi.get(selected.id)
      setSnippets(previous => previous.map(item => item.id === fresh.id ? fresh : item))
      selectSnippet(fresh)
      setNotice('已重新载入数据库中的最新版本。')
    } catch (reloadError) {
      setError(reloadError as AppError)
    } finally {
      setSaving(false)
    }
  }

  const updateField = <K extends keyof SnippetFields>(key: K, value: SnippetFields[K]) => {
    setFields(previous => ({ ...previous, [key]: value }))
    if (error?.field === key) setError(null)
    setNotice('')
  }

  const save = async () => {
    setSaving(true)
    setError(null)
    setNotice('')
    const payload = { ...fields, aliases: splitList(aliasesText), tags: splitList(tagsText) }
    try {
      const saved = selected
        ? await snippetsApi.update(selected, payload)
        : await snippetsApi.create(payload, requestId)
      setSnippets(previous => {
        const exists = previous.some(item => item.id === saved.id)
        const next = exists
          ? previous.map(item => item.id === saved.id ? saved : item)
          : [saved, ...previous]
        return next.sort((left, right) => right.updatedAt.localeCompare(left.updatedAt))
      })
      selectSnippet(saved)
      setNotice(selected ? '修改已安全保存。' : '片段已写入加密数据库。')
      setRequestId(crypto.randomUUID())
    } catch (saveError) {
      setError(saveError as AppError)
    } finally {
      setSaving(false)
    }
  }

  if (loading) {
    return <main className="min-h-screen ambient-glow-bg text-theme flex items-center justify-center">正在打开加密数据库…</main>
  }

  if (error && ['DB_KEY_UNAVAILABLE', 'DB_OPEN_FAILED'].includes(error.code) && snippets.length === 0) {
    return (
      <main className="min-h-screen ambient-glow-bg text-theme flex items-center justify-center p-6">
        <section className="raycast-window rounded-2xl max-w-xl p-8 space-y-4" role="alert">
          <ShieldAlert className="w-10 h-10 icon-danger" />
          <h1 className="text-xl font-bold">加密数据库不可用</h1>
          <p className="text-theme-secondary">{error.message}</p>
          <p className="text-sm text-theme-muted">为保护现有数据，Searchis 没有删除、重建或降级为明文数据库。完成修复后请完全退出并重启应用。</p>
          <code className="badge-danger inline-block px-2 py-1 rounded">{error.code}</code>
        </section>
      </main>
    )
  }

  return (
    <main className="min-h-screen ambient-glow-bg text-theme flex flex-col">
      <nav className="theme-titlebar border-b theme-divider px-4 py-2.5 flex items-center gap-2">
        <button
          onClick={() => setView('picker')}
          className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-semibold transition-colors ${view === 'picker' ? 'nav-active border' : 'btn-secondary'}`}
        >
          <Search className="w-4 h-4" /> 检索窗口
        </button>
        <button
          onClick={() => openManager()}
          className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-semibold transition-colors ${view === 'manager' ? 'nav-active border' : 'btn-secondary'}`}
        >
          <Database className="w-4 h-4" /> 管理窗口
        </button>
        <span className="ml-auto text-xs text-theme-muted">本机 SQLCipher 加密持久化 · {snippets.length} 条片段</span>
      </nav>

      {view === 'picker' ? (
        <QuickSearchWindow onClose={() => setView('manager')} onOpenManager={openManager} />
      ) : (
      <div className="max-w-6xl mx-auto p-4 sm:p-6 w-full">
        {loading ? (
          <div className="flex items-center justify-center py-24 text-theme-muted">正在打开加密数据库…</div>
        ) : (
        <div className="raycast-window rounded-2xl overflow-hidden min-h-[680px]">
        <header className="theme-titlebar border-b theme-divider px-5 py-4 flex items-center justify-between">
          <div>
            <h1 className="font-bold flex items-center gap-2"><Database className="w-5 h-5 icon-accent" />Searchis 安全片段库</h1>
            <p className="text-xs text-theme-muted mt-1">本机 SQLCipher 加密持久化 · {snippets.length} 条片段</p>
          </div>
          <button onClick={startCreate} disabled={saving} className="btn-primary rounded-xl px-4 py-2 text-sm font-semibold flex items-center gap-2 disabled:opacity-50">
            <Plus className="w-4 h-4" />新建片段
          </button>
        </header>

        <div className="grid grid-cols-1 md:grid-cols-[320px_1fr] min-h-[610px]">
          <aside className="theme-sidebar border-r theme-divider p-3">
            <h2 className="text-xs uppercase tracking-wider text-theme-muted px-2 py-2">已持久化片段</h2>
            <div className="space-y-2">
              {snippets.length === 0 && <p className="text-sm text-theme-muted p-4">尚无片段。创建第一条后，完全退出再启动即可验证持久化。</p>}
              {snippets.map(snippet => (
                <button
                  key={snippet.id}
                  onClick={() => selectSnippet(snippet)}
                  disabled={saving}
                  className={`w-full disabled:opacity-50 text-left p-3 rounded-xl border ${selected?.id === snippet.id ? 'nav-active' : 'theme-surface-subtle theme-divider-subtle interactive-muted'}`}
                >
                  <span className="badge-accent px-2 py-0.5 rounded text-xs font-mono">{snippet.key}</span>
                  <span className="block text-sm font-semibold mt-2 truncate">{snippet.title}</span>
                  <span className="block text-xs text-theme-muted mt-1">revision {snippet.revision} · {new Date(snippet.updatedAt).toLocaleString()}</span>
                </button>
              ))}
            </div>
          </aside>

          <section className="theme-pane-muted p-5 sm:p-8">
            <div className="flex items-center gap-2 mb-6">
              {selected ? <Edit3 className="w-5 h-5 icon-accent" /> : <Plus className="w-5 h-5 icon-accent" />}
              <h2 className="font-bold">{selected ? '编辑片段' : '创建第一条片段'}</h2>
            </div>

            <fieldset disabled={saving} className="space-y-4 max-w-2xl disabled:opacity-70">
              <Field label="Key" error={error?.field === 'key' ? error.message : undefined}>
                <input aria-label="Key" value={fields.key} onChange={event => updateField('key', event.target.value)} className="input-theme w-full px-3 py-2 rounded-xl font-mono" placeholder="例如 hello-world" />
              </Field>
              <Field label="标题" error={error?.field === 'title' ? error.message : undefined}>
                <input aria-label="标题" value={fields.title} onChange={event => updateField('title', event.target.value)} className="input-theme w-full px-3 py-2 rounded-xl" />
              </Field>
              <Field label={`正文（${contentBytes} / 102400 bytes）`} error={error?.field === 'content' ? error.message : undefined}>
                {selected?.sensitive && !sensitiveRevealed ? (
                  <button onClick={() => setSensitiveRevealed(true)} className="control w-full p-5 rounded-xl text-sm text-theme-secondary">敏感正文已遮挡。点击后仅在当前窗口会话中显示。</button>
                ) : (
                  <textarea aria-label="正文" rows={8} value={fields.content} onChange={event => updateField('content', event.target.value)} className="input-theme w-full px-3 py-2 rounded-xl font-mono resize-y" />
                )}
              </Field>
              <div className="grid sm:grid-cols-2 gap-4">
                <Field label="别名（逗号分隔）" error={error?.field === 'aliases' ? error.message : undefined}>
                  <input aria-label="别名" value={aliasesText} onChange={event => setAliasesText(event.target.value)} className="input-theme w-full px-3 py-2 rounded-xl" />
                </Field>
                <Field label="标签（逗号分隔）" error={error?.field === 'tags' ? error.message : undefined}>
                  <input aria-label="标签" value={tagsText} onChange={event => setTagsText(event.target.value)} className="input-theme w-full px-3 py-2 rounded-xl" />
                </Field>
              </div>
              <div className="control rounded-xl p-3 flex gap-6 text-sm">
                <label className="flex items-center gap-2"><input type="checkbox" checked={fields.sensitive} onChange={event => updateField('sensitive', event.target.checked)} />敏感正文</label>
                <label className="flex items-center gap-2"><input type="checkbox" checked={fields.pinned} onChange={event => updateField('pinned', event.target.checked)} />置顶</label>
              </div>

              {error && (
                <div className="status-danger border rounded-xl p-3 text-sm flex items-start gap-2" role="alert">
                  <AlertCircle className="w-5 h-5 shrink-0" />
                  <div className="space-y-2">
                    <span className="block">{error.message}{error.conflictKey ? ` 冲突 Key：${error.conflictKey}` : ''} <code>{error.code}</code></span>
                    {error.code === 'REVISION_CONFLICT' && (
                      <button type="button" onClick={reloadSelected} className="btn-secondary rounded-lg px-3 py-1.5 font-semibold">放弃未保存输入并重新载入</button>
                    )}
                  </div>
                </div>
              )}
              {notice && <div className="status-success border rounded-xl p-3 text-sm flex gap-2" role="status"><CheckCircle2 className="w-5 h-5" />{notice}</div>}

              <div className="flex justify-end pt-2">
                <button onClick={save} disabled={saving} className="btn-primary rounded-xl px-5 py-2 font-semibold flex items-center gap-2 disabled:opacity-50">
                  <Save className="w-4 h-4" />{saving ? '保存中…' : '保存到加密数据库'}
                </button>
              </div>
            </fieldset>
          </section>
        </div>
        </div>
        )}
      </div>
      )}
    </main>
  )
}

const Field: React.FC<{ label: string; error?: string; children: React.ReactNode }> = ({ label, error, children }) => (
  <label className="block space-y-1.5">
    <span className="text-sm font-semibold text-theme-secondary">{label}</span>
    {children}
    {error && <span className="text-xs text-danger flex items-center gap-1"><AlertCircle className="w-3.5 h-3.5" />{error}</span>}
  </label>
)
