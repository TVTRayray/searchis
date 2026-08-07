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
}
