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
  groupId: string | null
  language: SnippetLanguage
  kind: 'text' | 'file'
  fileExt: string | null
  fileSize: number | null
  fileMtime: string | null
  fileMissing: boolean
  fileAutoImported: boolean
}

export const SNIPPET_LANGUAGES = ['plain', 'bash', 'sql', 'yaml', 'json', 'toml', 'ini'] as const
export type SnippetLanguage = (typeof SNIPPET_LANGUAGES)[number]

export interface Group {
  id: string
  name: string
  parentId: string | null
  depth: number
  sortOrder: number
  createdAt: string
  updatedAt: string
  revision: number
  /** 直接成员条目数（不含回收站） */
  snippetCount: number
}

export interface SnippetFields {
  key: string
  title: string
  content: string
  aliases: string[]
  tags: string[]
  sensitive: boolean
  pinned: boolean
  groupId: string | null
  language: SnippetLanguage
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
  language: SnippetLanguage
  groupName: string | null
  kind: 'text' | 'file'
  fileExt?: string | null
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
  /** 请求了自动粘贴但设置中已关闭，已退化为仅复制。 */
  autoPasteDisabled: boolean
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
  trashMove: (id: string) => call<PersistedSnippet>('trash_move', { id }),
  trashRestore: (id: string) => call<PersistedSnippet>('trash_restore', { id }),
  trashPurgeOne: (id: string, confirmationToken: string) =>
    call<void>('trash_purge_one', { input: { id, confirmationToken } }),
  detectCapabilities: () =>
    call<PlatformCapabilities>('detect_capabilities'),
  togglePicker: () =>
    call<void>('toggle_picker'),
}

// SPEC-22：文件模板
export interface TemplateImportOutcome {
  created: PersistedSnippet[]
  rejected: Array<{ name: string; code: string }>
}

export interface ScanResult {
  added: number
  updated: number
  missing: number
  restored: number
  skipped: Array<{ reason: string }>
  failedBatches: number
}

export interface OpenWithCandidate {
  desktopId: string
  name: string
  icon?: string | null
}

export interface TemplateFields {
  title: string
  key: string
  groupId: string | null
  tags: string[]
  aliases: string[]
  pinned: boolean
}

export interface TemplateFormatCopyOutcome extends PasteOutcome {
  /** 给用户的提示，如“公式已转为数值”。 */
  notes: string[]
}

export interface TemplateExportOutcome {
  /** 导出文件完整路径；同一 operationId 重复投递时为空串。 */
  path: string
  counted: boolean
}

export const templatesApi = {
  import: (sourcePaths: string[], groupId: string | null, requestId: string) =>
    call<TemplateImportOutcome>('template_import', { input: { sourcePaths, groupId, requestId } }),
  createBlank: (
    kind: 'md' | 'xlsx',
    title: string,
    key: string,
    groupId: string | null,
    requestId: string,
  ) => call<PersistedSnippet>('template_create_blank', { input: { kind, title, key, groupId, requestId } }),
  update: (snippet: PersistedSnippet, fields: TemplateFields) =>
    call<PersistedSnippet>('template_update', {
      input: { ...fields, id: snippet.id, revision: snippet.revision },
    }),
  open: (id: string, desktopId?: string) =>
    call<void>('template_open', { input: { id, desktopId: desktopId ?? null } }),
  openWithCandidates: (id: string) =>
    call<OpenWithCandidate[]>('template_open_with_candidates', { id }),
  scan: () => call<ScanResult>('template_scan'),
  pickFiles: () => call<string[]>('template_pick_files'),
  // SPEC-23：带格式复制与导出
  formatCopy: (id: string, operationId: string, autoPaste: boolean) =>
    call<TemplateFormatCopyOutcome>('template_format_copy', { input: { id, operationId, autoPaste } }),
  export: (id: string, operationId: string) =>
    call<TemplateExportOutcome>('template_export', { input: { id, operationId } }),
}

export const groupsApi = {
  list: () => call<Group[]>('group_list'),
  create: (name: string, parentId: string | null, requestId: string) =>
    call<Group>('group_create', { input: { name, parentId, requestId } }),
  update: (group: Group, name: string, parentId: string | null) =>
    call<Group>('group_update', {
      input: { id: group.id, revision: group.revision, name, parentId },
    }),
  deletePreview: (id: string) =>
    call<{ snippetCount: number; childGroupCount: number }>('group_delete_preview', { id }),
  delete: (group: Group) =>
    call<{ movedSnippets: number; movedGroups: number }>('group_delete', {
      input: { id: group.id, revision: group.revision },
    }),
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
  openWithByExtension: Record<string, string>
  /** SPEC-24：null 表示使用默认 XDG 下载目录。 */
  exportDir?: string | null
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

// SPEC-24：设置视图路径与 zip 备份
export interface SettingsPaths {
  templatesDir: string
  /** 解析后的默认导出目录；找不到可用目录时为 null。 */
  defaultExportDir: string | null
}

export interface ExportPreview {
  estimatedSizeBytes: number
  sensitiveCount: number
  snippetCount: number
  hasSettings: boolean
  templateCount: number
  templateBytes: number
  /** 估算大小超过 1 GB，或 backup.json 超过 100 MB。 */
  exceedsLimit: boolean
}

export interface ExportOutcome {
  path: string
  sizeBytes: number
  snippetCount: number
  templateCount: number
}

export interface ImportValidation {
  valid: boolean
  snippetCount: number
  settingsPresent: boolean
  schemaVersion: number
  errors: string[]
  importToken: string
  backupVersion: number
  groupCount: number
  templateCount: number
  newCount: number
  updateCount: number
  conflictCount: number
  conflictKeys: string[]
}

export interface ImportOutcome {
  inserted: number
  updated: number
  replaced: number
  skipped: number
  groupsAdded: number
}

export interface BackupProgress {
  phase: 'export' | 'extract' | 'apply'
  done: number
  total: number
}

export const backupApi = {
  pickExportPath: () => call<string | null>('backup_pick_export_path'),
  pickImportFile: () => call<string | null>('backup_pick_import_file'),
  exportPreview: () => call<ExportPreview>('export_preview', { path: '' }),
  exportConfirm: (path: string) => call<ExportOutcome>('export_confirm', { path }),
  importValidate: (path: string) => call<ImportValidation>('import_validate', { path }),
  importCommit: (path: string, importToken: string, overwriteConflicts: boolean) =>
    call<ImportOutcome>('import_commit', { input: { path, importToken, overwriteConflicts } }),
  importDiscard: (importToken: string) => call<void>('import_discard', { importToken }),
  resetExamples: () => call<number>('reset_examples', { input: { confirm: true } }),
}

export const settingsPathsApi = {
  paths: () => call<SettingsPaths>('settings_paths'),
  pickDirectory: (initial?: string | null) =>
    call<string | null>('settings_pick_directory', { initial: initial ?? null }),
  openTemplatesDir: () => call<void>('templates_dir_open'),
}
