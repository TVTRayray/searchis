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
}

export type ViewMode = 'quick-picker' | 'manager' | 'settings';

export type SortOption = 'updated' | 'usage' | 'alpha' | 'key';

export type SidebarFilter = 'all' | 'pinned' | 'recent' | 'trash' | string;

export interface SearchMatchResult {
  snippet: Snippet;
  score: number;
  matchType: 'exact-key' | 'prefix-key' | 'alias' | 'title' | 'tag' | 'content';
  matchedText?: string;
}
