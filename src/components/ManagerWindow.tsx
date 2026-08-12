import React, { useState, useMemo } from 'react';
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
  AlertCircle,
  Shield,
  Eye,
  EyeOff
} from 'lucide-react';
import {
  Box,
  VStack,
  HStack,
  Inline,
  Layout,
  LayoutPanel,
  SideNav,
  SideNavItem,
  List,
  ListItem,
  Badge,
  Token,
  StatusDot,
  StatusToken,
  Button,
  IconButton,
  TextInput,
  TextArea,
  Switch,
  Card,
  Grid,
  Kbd,
} from './astryx';

interface ManagerWindowProps {
  snippets: Snippet[];
  onSaveSnippet: (snippet: Snippet) => void;
  onDeleteSnippet: (id: string) => void;
  onRestoreSnippet: (id: string) => void;
  onPermanentDeleteSnippet: (id: string) => void;
  onCopySnippet: (snippet: Snippet) => void;
  onPasteSnippet: (snippet: Snippet) => void;
  onOpenSettings: () => void;
  /** 检索窗口 Ctrl+E 跳转：编辑指定片段 */
  editRequestId?: string | null;
  /** 检索窗口 Ctrl+N 跳转：以该 Key 预填新建 */
  prefillCreateKey?: string;
  /** 请求消费完成后通知 App 清除 */
  onRequestHandled?: () => void;
}

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
      handleStartCreateNew();
      setKeyInput(prefillCreateKey);
      onRequestHandled?.();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [prefillCreateKey]);

  // 检索窗口 Ctrl+E：定位并选中指定片段（snippets 异步加载完成后重试）
  React.useEffect(() => {
    if (!editRequestId) return;
    const target = snippets.find(s => s.id === editRequestId);
    if (target) {
      setSelectedSnippetId(target.id);
      setIsCreatingNew(false);
      onRequestHandled?.();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editRequestId, snippets]);

  // 异步保存（App 经 IPC 创建后返回真实 UUID）后，临时 id 失效时回退到首项
  React.useEffect(() => {
    if (!isCreatingNew && selectedSnippetId && !snippets.some(s => s.id === selectedSnippetId)) {
      setSelectedSnippetId(snippets[0]?.id ?? null);
    }
  }, [snippets, selectedSnippetId, isCreatingNew]);

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
      lastUsedAt: selectedSnippet ? selectedSnippet.lastUsedAt : null,
      createdAt: selectedSnippet ? selectedSnippet.createdAt : now,
      updatedAt: now,
      deletedAt: selectedSnippet ? selectedSnippet.deletedAt : null,
      revision: selectedSnippet ? selectedSnippet.revision : 0,
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

  return (
    <Layout direction="row" height="100%">
      {/* 1. Left Sidebar Navigation using Astryx SideNav */}
      <SideNav
        width="220px"
        header={
          <HStack align="center" justify="space-between" className="px-1 py-1">
            <span className="text-xs font-bold uppercase tracking-wider text-theme-muted">
              分类目录
            </span>
            <IconButton
              icon={<Settings className="w-3.5 h-3.5" />}
              ariaLabel="打开系统设置"
              onClick={onOpenSettings}
              size="sm"
            />
          </HStack>
        }
      >
        <SideNavItem
          active={activeFilter === 'all'}
          onClick={() => setActiveFilter('all')}
          icon={<Layers className="w-4 h-4 icon-accent" />}
          badge={<Badge size="sm">{snippets.filter(s => !s.deletedAt).length}</Badge>}
        >
          全部片段
        </SideNavItem>

        <SideNavItem
          active={activeFilter === 'pinned'}
          onClick={() => setActiveFilter('pinned')}
          icon={<Star className="w-4 h-4 text-warning" />}
          badge={<Badge variant="warning" size="sm">{pinnedCount}</Badge>}
        >
          固定片段
        </SideNavItem>

        <SideNavItem
          active={activeFilter === 'recent'}
          onClick={() => setActiveFilter('recent')}
          icon={<Clock className="w-4 h-4 text-theme-muted" />}
        >
          最近使用
        </SideNavItem>

        <Box paddingY="xs" margin="xs" border="top" />

        <Box paddingX="xs" paddingY="2xs" className="text-[10px] font-bold uppercase text-theme-muted tracking-wider">
          标签分类
        </Box>
        {allTags.map(tag => (
          <SideNavItem
            key={tag}
            active={activeFilter === tag}
            onClick={() => setActiveFilter(tag)}
            icon={<Tag className="w-3.5 h-3.5 text-theme-muted" />}
            badge={<Badge variant="neutral" size="sm">{tagCounts[tag]}</Badge>}
          >
            {tag}
          </SideNavItem>
        ))}

        <Box paddingY="xs" margin="xs" border="top" />

        <SideNavItem
          active={activeFilter === 'trash'}
          onClick={() => setActiveFilter('trash')}
          icon={<Trash2 className="w-4 h-4 text-danger" />}
          badge={deletedCount > 0 ? <Badge variant="danger" size="sm">{deletedCount}</Badge> : undefined}
        >
          回收站
        </SideNavItem>
      </SideNav>

      {/* 2. Middle Dense Snippet List using Astryx LayoutPanel & List */}
      <LayoutPanel width="320px" minWidth="280px" border="right" background="subtle" className="flex flex-col">
        <Box padding="sm" border="bottom" background="sunken">
          <VStack gap="xs">
            <HStack align="center" justify="space-between">
              <TextInput
                placeholder="过滤片段..."
                leftIcon={<Search className="w-3.5 h-3.5" />}
                value={searchQuery}
                onChange={e => setSearchQuery(e.target.value)}
              />
              <IconButton
                icon={<Plus className="w-4 h-4" />}
                ariaLabel="新建片段"
                variant="primary"
                onClick={handleStartCreateNew}
                size="md"
              />
            </HStack>
            <HStack align="center" justify="space-between" className="px-1 text-[11px] text-theme-muted">
              <span>排序方式:</span>
              <select
                value={sortBy}
                onChange={e => setSortBy(e.target.value as SortOption)}
                className="bg-transparent border-none text-theme font-medium outline-none cursor-pointer"
              >
                <option value="updated">更新时间</option>
                <option value="usage">使用频率</option>
                <option value="alpha">名称首字母</option>
                <option value="key">Key 字母</option>
              </select>
            </HStack>
          </VStack>
        </Box>

        <Box flex="1" overflow="auto" padding="2xs">
          {filteredSnippets.length === 0 ? (
            <VStack align="center" justify="center" gap="xs" className="py-12 px-4 text-center text-theme-muted text-xs">
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
            <List divided={false}>
              {filteredSnippets.map(snippet => {
                const isSelected = snippet.id === selectedSnippetId;
                return (
                  <ListItem
                    key={snippet.id}
                    active={isSelected}
                    onClick={() => {
                      setSelectedSnippetId(snippet.id);
                      setIsCreatingNew(false);
                    }}
                    icon={
                      <Box
                        width="26px"
                        height="26px"
                        radius="sm"
                        className={`flex items-center justify-center font-mono font-bold text-xs shrink-0 ${
                          isSelected ? 'brand-mark' : 'badge-accent'
                        }`}
                      >
                        {snippet.pinned ? <Star className="w-3 h-3 fill-current" /> : snippet.key.slice(0, 2).toUpperCase()}
                      </Box>
                    }
                    title={snippet.title}
                    meta={<Token size="sm">{snippet.key}</Token>}
                    subtitle={`更新于 ${new Date(snippet.updatedAt).toLocaleDateString()}`}
                    extra={
                      snippet.sensitive ? <StatusDot status="warning" size="sm" label="" /> : undefined
                    }
                  />
                );
              })}
            </List>
          )}
        </Box>
      </LayoutPanel>

      {/* 3. Right Editor / Detail View using Astryx LayoutPanel */}
      <LayoutPanel flex="1" background="surface" padding="xl" overflow="auto">
        {selectedSnippet || isCreatingNew ? (
          <VStack gap="lg" className="max-w-3xl mx-auto">
            {/* Header Toolbar */}
            <Box border="bottom" paddingY="md">
              <HStack align="center" justify="space-between">
                <HStack align="center" gap="sm">
                <StatusToken
                  status={isCreatingNew ? 'active' : selectedSnippet?.deletedAt ? 'danger' : 'success'}
                  label={isCreatingNew ? '新建模式' : selectedSnippet?.deletedAt ? '已删除' : '已加密保存'}
                />
                <h2 className="text-base font-bold text-theme">
                  {isCreatingNew ? '创建新文本片段' : `编辑片段: ${selectedSnippet?.title}`}
                </h2>
              </HStack>

              <HStack align="center" gap="xs">
                {selectedSnippet && !isCreatingNew && (
                  <>
                    <Button
                      variant="secondary"
                      size="sm"
                      icon={copiedId === selectedSnippet.id ? <Check className="w-3.5 h-3.5 icon-success" /> : <Copy className="w-3.5 h-3.5" />}
                      onClick={() => handleCopy(selectedSnippet)}
                    >
                      {copiedId === selectedSnippet.id ? '已复制' : '仅复制'}
                    </Button>

                    <Button
                      variant="primary"
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
                      variant="success"
                      size="sm"
                      icon={<RotateCcw className="w-3.5 h-3.5" />}
                      onClick={() => onRestoreSnippet(selectedSnippet.id)}
                    >
                      恢复
                    </Button>
                    <Button
                      variant="danger"
                      size="sm"
                      icon={<Trash2 className="w-3.5 h-3.5" />}
                      onClick={() => onPermanentDeleteSnippet(selectedSnippet.id)}
                    >
                      彻底删除
                    </Button>
                  </>
                ) : selectedSnippet && !isCreatingNew ? (
                  <IconButton
                    icon={<Trash2 className="w-4 h-4 text-danger" />}
                    ariaLabel="放入回收站"
                    onClick={() => onDeleteSnippet(selectedSnippet.id)}
                  />
                ) : null}
              </HStack>
              </HStack>
            </Box>

            {/* Editor Inputs using Astryx Form Controls */}
            <VStack gap="md">
              <Grid columns={2} gap="md">
                <TextInput
                  label="Key (唯一快速检索标识)"
                  value={keyInput}
                  onChange={e => {
                    setKeyInput(e.target.value);
                    setKeyError('');
                  }}
                  placeholder="例如 email-work, addr-office"
                  mono
                  error={keyError}
                />
                <TextInput
                  label="标题 (展示名称)"
                  value={titleInput}
                  onChange={e => setTitleInput(e.target.value)}
                  placeholder="例如 工作邮箱, 公司地址"
                />
              </Grid>

              <VStack gap="xs">
                <HStack align="center" justify="space-between">
                  <span className="text-xs font-semibold text-theme-secondary">文本内容</span>
                  {isSensitive && (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => setSensitiveRevealed(prev => !prev)}
                    >
                      {sensitiveRevealed ? (
                        <>
                          <EyeOff className="w-3.5 h-3.5" />
                          <span>隐蔽</span>
                        </>
                      ) : (
                        <>
                          <Eye className="w-3.5 h-3.5" />
                          <span>显示明文</span>
                        </>
                      )}
                    </Button>
                  )}
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
                      <Shield className="w-6 h-6 icon-warning" />
                      <span className="text-xs font-semibold text-theme">敏感文本正文默认已安全隐蔽</span>
                      <span className="text-[11px] text-theme-muted">点击此处显示明文正文</span>
                    </VStack>
                  </Box>
                ) : (
                  <TextArea
                    value={contentInput}
                    onChange={e => setContentInput(e.target.value)}
                    placeholder="在此输入需要快速粘贴的任意文本片段..."
                    rows={7}
                  />
                )}
              </VStack>

              <Grid columns={2} gap="md">
                <TextInput
                  label="别名 (英文逗号分隔)"
                  value={aliasesInput}
                  onChange={e => setAliasesInput(e.target.value)}
                  placeholder="mail, workmail, 邮箱"
                />
                <TextInput
                  label="标签 (英文逗号分隔)"
                  value={tagsInput}
                  onChange={e => setTagsInput(e.target.value)}
                  placeholder="常用, 公司, 开发"
                />
              </Grid>

              {/* Settings Card for Toggles */}
              <Card padding="md" title="行为控制选项" subtitle="设置敏感隐私防护与置顶状态">
                <HStack align="center" justify="space-around" className="py-1">
                  <Switch
                    checked={isSensitive}
                    onChange={setIsSensitive}
                    label="敏感内容防护"
                    description="开启后默认隐藏正文内容"
                  />
                  <Switch
                    checked={isPinned}
                    onChange={setIsPinned}
                    label="固定到顶部"
                    description="在检索列表中优先靠前显示"
                  />
                </HStack>
              </Card>

              {/* Save Action */}
              <HStack justify="flex-end" gap="sm" className="pt-2">
                <Button
                  variant="primary"
                  size="lg"
                  icon={<Save className="w-4 h-4" />}
                  onClick={handleSave}
                >
                  保存片段数据
                </Button>
              </HStack>
            </VStack>
          </VStack>
        ) : (
          <VStack align="center" justify="center" className="h-full text-theme-muted text-sm space-y-3">
            <Layers className="w-10 h-10 icon-accent" />
            <p>请在左侧选择要查看或编辑的片段</p>
            <Button variant="primary" icon={<Plus className="w-4 h-4" />} onClick={handleStartCreateNew}>
              创建新片段
            </Button>
          </VStack>
        )}
      </LayoutPanel>
    </Layout>
  );
};
