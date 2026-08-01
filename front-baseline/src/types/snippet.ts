export interface Snippet {
  id: string;
  key: string;            // 唯一标识，如 "addr-company"
  title: string;          // 展示名称
  content: string;        // 实际粘贴内容
  aliases: string[];      // 别名列表
  tags: string[];         // 标签
  pinned: boolean;        // 是否置顶
  sensitive?: boolean;    // 是否敏感内容 (默认遮挡)
  usageCount: number;     // 使用次数
  lastUsedAt?: string;    // 最近使用时间 (ISO 8601)
  createdAt: string;
  updatedAt: string;
  deletedAt?: string;     // 回收站使用
}

export type ViewMode = 'quick-picker' | 'manager' | 'onboarding' | 'settings';

export type SortOption = 'updated' | 'usage' | 'alpha' | 'key';

export type SidebarFilter = 'all' | 'pinned' | 'recent' | 'trash' | string; // string is tag name

export interface SearchMatchResult {
  snippet: Snippet;
  score: number;
  matchType: 'exact-key' | 'prefix-key' | 'alias' | 'title' | 'tag' | 'content';
  matchedText?: string;
}

export interface SettingsConfig {
  globalShortcut: string;
  copyShortcut: string;
  autoPaste: boolean;
  restoreClipboard: boolean;
  launchAtLogin: boolean;
  theme: 'dark' | 'light' | 'system';
  accentColor: 'blue' | 'purple' | 'pink' | 'orange' | 'green' | 'slate';
  playAudioFeedback: boolean;
  maxResultsCount: number;
}
