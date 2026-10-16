export interface Snippet {
  id: string;
  key: string;
  normalizedKey: string;
  title: string;
  content: string;
  aliases: string[];
  tags: string[];
  pinned: boolean;
  sensitive: boolean;
  usageCount: number;
  lastUsedAt: string | null;
  createdAt: string;
  updatedAt: string;
  deletedAt: string | null;
  revision: number;
  groupId: string | null;
  language: SnippetLanguage;
  kind: 'text' | 'file';
}

export type SnippetLanguage = 'plain' | 'bash' | 'sql' | 'yaml' | 'json' | 'toml' | 'ini';

export interface Group {
  id: string;
  name: string;
  parentId: string | null;
  depth: number;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
  revision: number;
  snippetCount: number;
}

export type ViewMode = 'quick-picker' | 'manager' | 'settings';

export type SortOption = 'updated' | 'usage' | 'alpha' | 'key';

/** `group:<id>` 为分组，`group:`（id 为空）为未分组，其余为标签名。 */
export type SidebarFilter = 'all' | 'pinned' | 'recent' | 'trash' | string;
export const GROUP_FILTER_PREFIX = 'group:';
export type TypeFilter = 'all' | 'text' | 'file';

export interface SearchMatchResult {
  snippet: Snippet;
  score: number;
  matchType: 'exact-key' | 'prefix-key' | 'alias' | 'title' | 'tag' | 'content';
  matchedText?: string;
}
