import React, { useState, useMemo, useEffect } from 'react';
import { Snippet, SidebarFilter, SortOption } from '../types/snippet';
import {
  Layers,
  Star,
  Clock,
  Tag,
  Trash2,
  Settings,
  Search,
  Plus,
  Copy,
  Check,
  Save,
  RotateCcw,
  Shield,
  Eye,
  EyeOff,
} from 'lucide-react';
import {
  Box,
  VStack,
  HStack,
  Inline,
  Layout,
  LayoutPanel,
  Grid,
} from './layout';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { NativeSelect, NativeSelectOption } from '@/components/ui/native-select';
import { Card, CardContent } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { Kbd } from '@/components/ui/kbd';

interface ManagerWindowProps {
  snippets: Snippet[];
  onSaveSnippet: (snippet: Snippet) => void;
  onDeleteSnippet: (id: string) => void;
  onRestoreSnippet: (id: string) => void;
  onPermanentDeleteSnippet: (id: string) => void;
  onCopySnippet: (snippet: Snippet) => void;
  onPasteSnippet: (snippet: Snippet) => void;
  onOpenSettings: () => void;
  editRequestId?: string | null;
  prefillCreateKey?: string;
  onRequestHandled?: () => void;
}

const statusTokenConfig: Record<string, { color: string; label: string }> = {
  active: { color: 'var(--color-accent)', label: '新建模式' },
  success: { color: 'var(--color-success)', label: '已加密保存' },
  danger: { color: 'var(--color-danger)', label: '已删除' },
};

export const ManagerWindow: React.FC<ManagerWindowProps> = ({
  snippets,
  onSaveSnippet,
  onDeleteSnippet,
  onRestoreSnippet,
  onPermanentDeleteSnippet,
  onCopySnippet,
  onPasteSnippet,
  onOpenSettings,
  editRequestId,
  prefillCreateKey,
  onRequestHandled,
}) => {
  const [activeFilter, setActiveFilter] = useState<SidebarFilter>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [sortBy, setSortBy] = useState<SortOption>('updated');
  const [selectedSnippetId, setSelectedSnippetId] = useState<string | null>(snippets[0]?.id || null);

  // Editor form state
  const selectedSnippet = snippets.find(s => s.id === selectedSnippetId) || null;
  const [keyInput, setKeyInput] = useState('');
  const [titleInput, setTitleInput] = useState('');
  const [contentInput, setContentInput] = useState('');
  const [aliasesInput, setAliasesInput] = useState('');
  const [tagsInput, setTagsInput] = useState('');
  const [isSensitive, setIsSensitive] = useState(false);
  const [isPinned, setIsPinned] = useState(false);
  const [isCreatingNew, setIsCreatingNew] = useState(false);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [sensitiveRevealed, setSensitiveRevealed] = useState(false);
  const [keyError, setKeyError] = useState('');

  // Sync editor fields when selection changes
  React.useEffect(() => {
    if (selectedSnippet) {
      setKeyInput(selectedSnippet.key);
      setTitleInput(selectedSnippet.title);
      setContentInput(selectedSnippet.content);
      setAliasesInput(selectedSnippet.aliases.join(', '));
      setTagsInput(selectedSnippet.tags.join(', '));
      setIsSensitive(!!selectedSnippet.sensitive);
      setIsPinned(!!selectedSnippet.pinned);
      setIsCreatingNew(false);
      setSensitiveRevealed(false);
      setKeyError('');
    }
  }, [selectedSnippetId, selectedSnippet]);

  // 检索窗口 Ctrl+N：预填 Key 新建
  React.useEffect(() => {
    if (prefillCreateKey) {
      setSelectedSnippetId(null);
      setIsCreatingNew(true);
      setKeyInput(prefillCreateKey);
      setTitleInput('');
      setContentInput('');
      setAliasesInput('');
      setTagsInput('');
      setIsSensitive(false);
      setIsPinned(false);
      setSensitiveRevealed(true);
      setKeyError('');
      onRequestHandled?.();
    }
  }, [prefillCreateKey]);

  // 检索窗口 Ctrl+E：定位并选中指定片段
  React.useEffect(() => {
    if (!editRequestId) return;
    const target = snippets.find(s => s.id === editRequestId);
    if (target) {
      setSelectedSnippetId(target.id);
      setIsCreatingNew(false);
      onRequestHandled?.();
    }
  }, [editRequestId, snippets]);

  // Extract unique tags and count map
  const { allTags, tagCounts, pinnedCount, deletedCount } = useMemo(() => {
    const counts: Record<string, number> = {};
    let pinned = 0;
    let deleted = 0;

    snippets.forEach(s => {
      if (s.deletedAt) {
        deleted++;
        return;
      }
      if (s.pinned) pinned++;
      s.tags.forEach(t => {
        counts[t] = (counts[t] || 0) + 1;
      });
    });

    return {
      allTags: Object.keys(counts),
      tagCounts: counts,
      pinnedCount: pinned,
      deletedCount: deleted,
    };
  }, [snippets]);

  // Filtered & sorted snippet list
  const filteredSnippets = useMemo(() => {
    return snippets.filter(s => {
      if (activeFilter === 'trash') {
        if (!s.deletedAt) return false;
      } else {
        if (s.deletedAt) return false;
        if (activeFilter === 'pinned' && !s.pinned) return false;
        if (activeFilter === 'recent') {
          if (!s.lastUsedAt) return false;
        } else if (activeFilter !== 'all' && !s.tags.includes(activeFilter)) {
          return false;
        }
      }

      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      return (
        s.key.toLowerCase().includes(q) ||
        s.title.toLowerCase().includes(q) ||
        s.content.toLowerCase().includes(q) ||
        s.aliases.some(a => a.toLowerCase().includes(q)) ||
        s.tags.some(t => t.toLowerCase().includes(q))
      );
    }).sort((a, b) => {
      if (sortBy === 'updated') return new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime();
      if (sortBy === 'usage') return b.usageCount - a.usageCount;
      if (sortBy === 'alpha') return a.title.localeCompare(b.title);
      if (sortBy === 'key') return a.key.localeCompare(b.key);
      return 0;
    });
  }, [snippets, activeFilter, searchQuery, sortBy]);

  const handleStartCreateNew = () => {
    setSelectedSnippetId(null);
    setIsCreatingNew(true);
    setKeyInput('');
    setTitleInput('');
    setContentInput('');
    setAliasesInput('');
    setTagsInput('');
    setIsSensitive(false);
    setIsPinned(false);
    setSensitiveRevealed(true);
    setKeyError('');
  };

  const handleSave = () => {
    const trimmedKey = keyInput.trim().toLowerCase();
    if (!trimmedKey) {
      setKeyError('Key 不能为空');
      return;
    }
    if (!/^[a-zA-Z0-9_-]+$/.test(trimmedKey)) {
      setKeyError('Key 只能包含字母、数字、- 和 _');
      return;
    }

    // Check duplicate key
    const duplicate = snippets.find(s => s.key === trimmedKey && s.id !== selectedSnippet?.id && !s.deletedAt);
    if (duplicate) {
      setKeyError(`Key "${trimmedKey}" 已被片段 "${duplicate.title}" 使用`);
      return;
    }

    const aliases = aliasesInput.split(',').map(s => s.trim()).filter(Boolean);
    const tags = tagsInput.split(',').map(s => s.trim()).filter(Boolean);

    const now = new Date().toISOString();
    const updatedSnippet: Snippet = {
      id: selectedSnippet && !isCreatingNew ? selectedSnippet.id : `snip-${Date.now()}`,
      key: trimmedKey,
      normalizedKey: trimmedKey,
      title: titleInput.trim() || trimmedKey,
      content: contentInput,
      aliases,
      tags,
      pinned: isPinned,
      sensitive: isSensitive,
      usageCount: selectedSnippet ? selectedSnippet.usageCount : 0,
      lastUsedAt: selectedSnippet?.lastUsedAt ?? null,
      createdAt: selectedSnippet ? selectedSnippet.createdAt : now,
      updatedAt: now,
      deletedAt: selectedSnippet?.deletedAt ?? null,
      revision: selectedSnippet?.revision ?? 0,
    };

    onSaveSnippet(updatedSnippet);
    setSelectedSnippetId(updatedSnippet.id);
    setIsCreatingNew(false);
    setKeyError('');
  };

  const handleCopy = (snippet: Snippet) => {
    onCopySnippet(snippet);
    setCopiedId(snippet.id);
    setTimeout(() => setCopiedId(null), 1500);
  };

  // ⌘S / Ctrl+S saves the form (keyboard-first)
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 's') {
        if (!selectedSnippet && !isCreatingNew) return;
        e.preventDefault();
        handleSave();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [handleSave, selectedSnippet, isCreatingNew]);

  const navItem = (
    active: boolean,
    onClick: () => void,
    icon: React.ReactNode,
    label: string,
    badge?: React.ReactNode,
  ) => (
    <Button
      type="button"
      variant={active ? 'secondary' : 'ghost'}
      className="w-full justify-start gap-2 rounded-lg px-3 py-2 text-xs font-medium"
      onClick={onClick}
    >
      {icon}
      <span className="flex-1 text-left">{label}</span>
      {badge}
    </Button>
  );

  return (
    <Layout direction="row" height="100%">
      {/* 1. Left Sidebar Navigation */}
      <Box as="nav" width="220px" border="right" background="subtle" className="flex flex-col gap-0.5 p-2 shrink-0">
        <HStack align="center" justify="space-between" className="px-1 pb-1">
          <span className="text-xs font-medium text-[color:var(--color-fg-muted)]">分类目录</span>
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label="打开系统设置"
            title="打开系统设置"
            onClick={onOpenSettings}
          >
            <Settings className="size-3.5" />
          </Button>
        </HStack>

        {navItem(
          activeFilter === 'all',
          () => setActiveFilter('all'),
          <Layers className="size-4 text-[color:var(--color-accent-fg)]" />,
          '全部片段',
          <Badge variant="secondary" className="ml-auto rounded-full px-1.5 text-[10px]">
            {snippets.filter(s => !s.deletedAt).length}
          </Badge>,
        )}
        {navItem(
          activeFilter === 'pinned',
          () => setActiveFilter('pinned'),
          <Star className="size-4 text-[color:var(--color-warning)]" />,
          '固定片段',
          pinnedCount > 0 ? (
            <Badge variant="secondary" className="ml-auto rounded-full px-1.5 text-[10px] text-[color:var(--color-warning)]">
              {pinnedCount}
            </Badge>
          ) : undefined,
        )}
        {navItem(
          activeFilter === 'recent',
          () => setActiveFilter('recent'),
          <Clock className="size-4 text-[color:var(--color-fg-muted)]" />,
          '最近使用',
        )}

        <Box paddingY="xs" margin="xs" border="top" />

        <span className="px-3 pb-1 pt-2 text-xs font-medium text-[color:var(--color-fg-muted)]">标签分类</span>
        {allTags.map(tag => (
          <div key={tag}>
            {navItem(
              activeFilter === tag,
              () => setActiveFilter(tag),
              <Tag className="size-3.5 text-[color:var(--color-fg-muted)]" />,
              tag,
              <Badge variant="secondary" className="ml-auto rounded-full px-1.5 text-[10px]">
                {tagCounts[tag]}
              </Badge>,
            )}
          </div>
        ))}

        <Box paddingY="xs" margin="xs" border="top" />

        {navItem(
          activeFilter === 'trash',
          () => setActiveFilter('trash'),
          <Trash2 className="size-4 text-[color:var(--color-danger)]" />,
          '回收站',
          deletedCount > 0 ? (
            <Badge variant="secondary" className="ml-auto rounded-full px-1.5 text-[10px] text-[color:var(--color-danger)]">
              {deletedCount}
            </Badge>
          ) : undefined,
        )}
      </Box>

      {/* 2. Middle Dense Snippet List */}
      <LayoutPanel width="384px" minWidth="320px" border="right" background="subtle" className="flex flex-col">
        <Box padding="sm" border="bottom" background="sunken">
          <VStack gap="xs">
            <HStack align="center" gap="xs">
              <div className="relative flex-1">
                <Search className="absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-[color:var(--color-fg-muted)] pointer-events-none" />
                <Input
                  placeholder="过滤片段..."
                  value={searchQuery}
                  onChange={e => setSearchQuery(e.target.value)}
                  className="h-8 pl-8"
                />
              </div>
              <Button
                variant="default"
                size="icon"
                aria-label="新建片段"
                title="新建片段"
                onClick={handleStartCreateNew}
              >
                <Plus className="size-4" />
              </Button>
            </HStack>
            <HStack align="center" justify="space-between" className="px-1 text-[11px] text-[color:var(--color-fg-muted)]">
              <span>排序方式:</span>
              <NativeSelect
                size="sm"
                value={sortBy}
                onChange={e => setSortBy(e.target.value as SortOption)}
                className="h-8 w-28 text-xs"
                aria-label="排序方式"
              >
                <NativeSelectOption value="updated">更新时间</NativeSelectOption>
                <NativeSelectOption value="usage">使用频率</NativeSelectOption>
                <NativeSelectOption value="alpha">名称首字母</NativeSelectOption>
                <NativeSelectOption value="key">Key 字母</NativeSelectOption>
              </NativeSelect>
            </HStack>
          </VStack>
        </Box>

        <Box flex="1" overflow="auto" padding="2xs">
          {filteredSnippets.length === 0 ? (
            <VStack align="center" justify="center" gap="xs" className="py-12 px-4 text-center text-[color:var(--color-fg-muted)] text-xs">
              <span>暂无匹配的文本片段</span>
              {activeFilter === 'trash' ? (
                <span>回收站为空</span>
              ) : (
                <Button variant="ghost" size="sm" onClick={handleStartCreateNew}>
                  点击新建第一条片段
                </Button>
              )}
            </VStack>
          ) : (
            <VStack gap="2xs" className="p-1.5">
              {filteredSnippets.map(snippet => {
                const isSelected = snippet.id === selectedSnippetId;
                return (
                  <button
                    key={snippet.id}
                    type="button"
                    onClick={() => {
                      setSelectedSnippetId(snippet.id);
                      setIsCreatingNew(false);
                    }}
                    className={`relative w-full flex items-center gap-3 rounded-lg px-3 py-2.5 text-left cursor-default transition-[color,background-color] ${
                      isSelected ? 'bg-accent text-accent-foreground' : 'hover:bg-accent/50'
                    }`}
                  >
                    {isSelected && (
                      <span className="absolute left-0 top-1/2 -translate-y-1/2 h-6 w-0.5 rounded-full bg-primary" aria-hidden />
                    )}
                    <Box
                      width="32px"
                      height="32px"
                      radius="md"
                      className={`flex items-center justify-center font-mono font-bold text-xs shrink-0 ${
                        isSelected ? 'brand-mark' : 'badge-accent'
                      }`}
                    >
                      {snippet.pinned ? <Star className="size-3.5 fill-current" /> : snippet.key.slice(0, 2).toUpperCase()}
                    </Box>
                    <VStack gap="2xs" className="min-w-0 flex-1">
                      <span className="text-[13px] font-semibold truncate">{snippet.title}</span>
                      <HStack align="center" gap="xs">
                        <span className="font-mono text-[11px] text-[color:var(--color-fg-muted)] truncate">{snippet.key}</span>
                        <span className="text-[11px] text-[color:var(--color-fg-muted)] tabular-nums shrink-0">
                          更新于 {new Date(snippet.updatedAt).toLocaleDateString()}
                        </span>
                      </HStack>
                    </VStack>
                    {snippet.sensitive && (
                      <span className="size-1.5 rounded-full bg-[color:var(--color-warning)] shrink-0" aria-label="敏感内容" />
                    )}
                  </button>
                );
              })}
            </VStack>
          )}
        </Box>
      </LayoutPanel>

      {/* 3. Right Editor / Detail View */}
      <LayoutPanel flex="1" background="surface" padding="lg" overflow="auto">
        {selectedSnippet || isCreatingNew ? (
          <VStack gap="md" className="max-w-2xl mx-auto py-2">
            {/* Slim toolbar: status + title left, actions right */}
            <HStack align="center" justify="space-between" gap="sm" className="border-b pb-3">
              <HStack align="center" gap="sm" className="min-w-0">
                <span
                  className="size-2 rounded-full shrink-0"
                  style={{ background: statusTokenConfig[isCreatingNew ? 'active' : selectedSnippet?.deletedAt ? 'danger' : 'success'].color }}
                  aria-hidden
                />
                <h2 className="text-base font-bold text-theme truncate">
                  {isCreatingNew ? '创建新文本片段' : selectedSnippet?.title}
                </h2>
                <span className="text-xs text-muted-foreground shrink-0">
                  {statusTokenConfig[isCreatingNew ? 'active' : selectedSnippet?.deletedAt ? 'danger' : 'success'].label}
                </span>
              </HStack>

              <HStack align="center" gap="xs" className="shrink-0">
                  {selectedSnippet && !isCreatingNew && (
                    <>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleCopy(selectedSnippet)}
                      >
                        {copiedId === selectedSnippet.id ? (
                          <Check className="size-3.5 text-[color:var(--color-success)]" />
                        ) : (
                          <Copy className="size-3.5" />
                        )}
                        {copiedId === selectedSnippet.id ? '已复制' : '仅复制'}
                      </Button>

                      <Button
                        variant="default"
                        size="sm"
                        onClick={() => onPasteSnippet(selectedSnippet)}
                      >
                        快速粘贴
                      </Button>
                    </>
                  )}

                  {selectedSnippet?.deletedAt ? (
                    <>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => onRestoreSnippet(selectedSnippet.id)}
                      >
                        <RotateCcw className="size-3.5" />
                        恢复
                      </Button>
                      <Button
                        variant="destructive"
                        size="sm"
                        onClick={() => onPermanentDeleteSnippet(selectedSnippet.id)}
                      >
                        <Trash2 className="size-3.5" />
                        彻底删除
                      </Button>
                    </>
                  ) : selectedSnippet && !isCreatingNew ? (
                    <Button
                      variant="ghost"
                      size="icon"
                      aria-label="放入回收站"
                      title="放入回收站"
                      onClick={() => onDeleteSnippet(selectedSnippet.id)}
                    >
                      <Trash2 className="size-4 text-[color:var(--color-danger)]" />
                    </Button>
                  ) : null}
                </HStack>
              </HStack>

            {/* Form sheet */}
            <Card>
              <CardContent className="space-y-6 p-6">
                {/* 基本信息 */}
                <section className="space-y-3">
                  <h3 className="text-xs font-medium text-muted-foreground">基本信息</h3>
                  <Grid columns={2} gap="md">
                    <VStack gap="xs">
                      <Label htmlFor="key-input">Key (唯一快速检索标识)</Label>
                      <Input
                        id="key-input"
                        value={keyInput}
                        onChange={e => {
                          setKeyInput(e.target.value);
                          setKeyError('');
                        }}
                        placeholder="例如 email-work, addr-office"
                        className="font-mono"
                        aria-invalid={!!keyError}
                      />
                      {keyError && <p className="text-xs text-destructive">{keyError}</p>}
                    </VStack>
                    <VStack gap="xs">
                      <Label htmlFor="title-input">标题 (展示名称)</Label>
                      <Input
                        id="title-input"
                        value={titleInput}
                        onChange={e => setTitleInput(e.target.value)}
                        placeholder="例如 工作邮箱, 公司地址"
                      />
                    </VStack>
                  </Grid>
                </section>

                <Separator />

                {/* 文本内容 */}
                <section className="space-y-2">
                  <HStack align="center" justify="space-between">
                    <h3 className="text-xs font-medium text-muted-foreground">文本内容</h3>
                    <HStack align="center" gap="sm">
                      {isSensitive && (
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-6 px-2 text-[11px]"
                          onClick={() => setSensitiveRevealed(prev => !prev)}
                        >
                          {sensitiveRevealed ? (
                            <>
                              <EyeOff className="size-3" />
                              <span>隐蔽</span>
                            </>
                          ) : (
                            <>
                              <Eye className="size-3" />
                              <span>显示明文</span>
                            </>
                          )}
                        </Button>
                      )}
                      <span className="text-[11px] font-mono tabular-nums text-muted-foreground">
                        {contentInput.length} 字符
                      </span>
                    </HStack>
                  </HStack>

                  {isSensitive && !sensitiveRevealed ? (
                    <Box
                      padding="xl"
                      radius="md"
                      background="sunken"
                      border="all"
                      className="text-center cursor-pointer"
                      onClick={() => setSensitiveRevealed(true)}
                    >
                      <VStack align="center" justify="center" gap="xs">
                        <Shield className="w-6 h-6 text-[color:var(--color-warning)]" />
                        <span className="text-xs font-semibold text-theme">敏感文本正文默认已安全隐蔽</span>
                        <span className="text-[11px] text-[color:var(--color-fg-muted)]">点击此处显示明文正文</span>
                      </VStack>
                    </Box>
                  ) : (
                    <Textarea
                      id="content-input"
                      value={contentInput}
                      onChange={e => setContentInput(e.target.value)}
                      placeholder="在此输入需要快速粘贴的任意文本片段..."
                      className="min-h-40 font-mono text-sm leading-relaxed"
                    />
                  )}
                </section>

                <Separator />

                {/* 元信息 */}
                <section className="space-y-3">
                  <h3 className="text-xs font-medium text-muted-foreground">元信息</h3>
                  <Grid columns={2} gap="md">
                    <VStack gap="xs">
                      <Label htmlFor="aliases-input">别名 (英文逗号分隔)</Label>
                      <Input
                        id="aliases-input"
                        value={aliasesInput}
                        onChange={e => setAliasesInput(e.target.value)}
                        placeholder="mail, workmail, 邮箱"
                      />
                    </VStack>
                    <VStack gap="xs">
                      <Label htmlFor="tags-input">标签 (英文逗号分隔)</Label>
                      <Input
                        id="tags-input"
                        value={tagsInput}
                        onChange={e => setTagsInput(e.target.value)}
                        placeholder="常用, 公司, 开发"
                      />
                    </VStack>
                  </Grid>
                </section>

                <Separator />

                {/* 行为 */}
                <section className="space-y-3">
                  <h3 className="text-xs font-medium text-muted-foreground">行为</h3>
                  <div className="flex flex-col gap-5 rounded-lg bg-muted/60 p-4 sm:flex-row sm:justify-between">
                    <div className="flex items-center justify-between gap-4">
                      <div>
                        <Label htmlFor="sensitive-switch">敏感内容防护</Label>
                        <p className="text-xs text-muted-foreground">开启后默认隐藏正文内容</p>
                      </div>
                      <Switch id="sensitive-switch" checked={isSensitive} onCheckedChange={setIsSensitive} />
                    </div>
                    <div className="flex items-center justify-between gap-4">
                      <div>
                        <Label htmlFor="pinned-switch">固定到顶部</Label>
                        <p className="text-xs text-muted-foreground">在检索列表中优先靠前显示</p>
                      </div>
                      <Switch id="pinned-switch" checked={isPinned} onCheckedChange={setIsPinned} />
                    </div>
                  </div>
                </section>
              </CardContent>
            </Card>

            {/* Save bar */}
            <HStack align="center" justify="space-between" className="border-t pt-3">
              <span className="text-[11px] text-muted-foreground hidden sm:inline">
                提示：<Kbd>⌘S</Kbd> 快速保存
              </span>
              <HStack align="center" gap="sm">
                <Button variant="default" onClick={handleSave}>
                  <Save className="size-4" />
                  保存片段
                </Button>
              </HStack>
            </HStack>
          </VStack>
        ) : (
          <VStack align="center" justify="center" gap="md" className="h-full px-6 text-center">
            <Box width="52px" height="52px" radius="xl" className="brand-mark flex items-center justify-center">
              <Layers className="size-6" />
            </Box>
            <VStack gap="xs">
              <p className="text-sm font-semibold text-theme">选择一个片段开始编辑</p>
              <p className="text-xs text-muted-foreground">或点击下方按钮创建第一条文本片段</p>
            </VStack>
            <Button variant="default" onClick={handleStartCreateNew}>
              <Plus className="size-4" />
              创建新片段
            </Button>
          </VStack>
        )}
      </LayoutPanel>
    </Layout>
  );
};
