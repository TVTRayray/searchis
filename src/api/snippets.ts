import { invoke } from '@tauri-apps/api/core'

export interface PersistedSnippet {
  id: string
  key: string
  normalizedKey: string
  title: string
  content: string
  aliases: string[]
  tags: string[]
  pinned: boolean
  sensitive: boolean
  usageCount: number
  lastUsedAt: string | null
  createdAt: string
  updatedAt: string
  deletedAt: string | null
  revision: number
}

export interface SnippetFields {
  key: string
  title: string
  content: string
  aliases: string[]
  tags: string[]
  sensitive: boolean
  pinned: boolean
}

export interface AppError {
  code: string
  message: string
  field?: string
  conflictKey?: string
}

export interface SearchResultItem {
  id: string
  key: string
  title: string
  aliases: string[]
  tags: string[]
  pinned: boolean
  sensitive: boolean
  usageCount: number
  lastUsedAt: string | null
}

export interface SearchResponse {
  items: SearchResultItem[]
  total: number
}

export interface PasteOutcome {
  snippet: SearchResultItem
  action: string // "copied" | "pasted" | "degraded"
  targetValid: boolean
  counted: boolean
}

// SPEC-02 copy_snippet 命令返回类型（保留向后兼容）
export interface CopyOutcome {
  snippet: SearchResultItem
  counted: boolean
}

export interface PrepareNewOutcome {
  normalizedKey: string
}

// SPEC-03
export interface PlatformCapabilities {
  clipboardWrite: boolean
  x11Inject: boolean
  kglobalaccel: boolean
}

export interface ShortcutInfo {
  current: string
  registered: boolean
}

export interface ManagerRequest {
  editId?: string
  prefillKey?: string
}

function normalizeError(error: unknown): AppError {
  if (typeof error === 'object' && error !== null && 'code' in error && 'message' in error) {
    return error as AppError
  }
  return {
    code: 'IPC_FAILED',
    message: '无法连接本地应用服务。请完全退出并重启 Searchis。',
  }
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args)
  } catch (error) {
    throw normalizeError(error)
  }
}

export const snippetsApi = {
  list: () => call<PersistedSnippet[]>('snippet_list'),
  get: (id: string) => call<PersistedSnippet>('snippet_get', { id }),
  create: (fields: SnippetFields, requestId: string) =>
    call<PersistedSnippet>('snippet_create', { input: { ...fields, requestId } }),
  update: (snippet: PersistedSnippet, fields: SnippetFields) =>
    call<PersistedSnippet>('snippet_update', {
      input: { ...fields, id: snippet.id, revision: snippet.revision },
    }),
  search: (query: string, limit: number) =>
    call<SearchResponse>('search_snippets', { input: { query, limit } }),
  copy: (id: string, operationId: string, keepOpen: boolean) =>
    call<CopyOutcome>('copy_snippet', { input: { id, operationId, keepOpen } }),
  prepareNew: (rawQuery: string) =>
    call<PrepareNewOutcome>('prepare_new_snippet', { rawQuery }),
  // SPEC-03
  executePaste: (id: string, operationId: string, autoPaste: boolean) =>
    call<PasteOutcome>('execute_paste', { input: { snippetId: id, operationId, autoPaste } }),
  detectCapabilities: () =>
    call<PlatformCapabilities>('detect_capabilities'),
  togglePicker: () =>
    call<void>('toggle_picker'),
}

export const windowApi = {
  openSearch: () => call<void>('open_search_window'),
  closeSearch: () => call<void>('close_search_window'),
  openManager: (request?: ManagerRequest) =>
    call<void>('open_manager_window', { request: request ?? null }),
  takeManagerRequest: () =>
    call<ManagerRequest | null>('take_manager_request'),
  registerShortcut: (accelerator: string) =>
    call<ShortcutInfo>('register_shortcut', { accelerator }),
}

// SPEC-06: Settings
export interface SettingsConfig {
  globalShortcut: string
  autoPaste: boolean
  restoreClipboard: boolean
  launchAtLogin: boolean
  theme: string
  maxResultsCount: number
  trashAutoPurgeDays: number | null
  onboardingCompletedAt: string | null
  schemaVersion: number
}

export interface SettingsResponse {
  settings: SettingsConfig
  revision: number
  autostartActual: boolean
}

export const settingsApi = {
  get: () => call<SettingsResponse>('settings_get'),
  update: (key: string, value: unknown, revision: number) =>
    call<SettingsConfig>('settings_update', { input: { key, value, revision } }),
  autostartGet: () => call<boolean>('autostart_get'),
  autostartSet: (enabled: boolean) =>
    call<boolean>('autostart_set', { input: { enabled } }),
}
