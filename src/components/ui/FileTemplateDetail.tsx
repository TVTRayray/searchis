import React, { useEffect, useMemo, useRef, useState } from 'react';
import { AlertTriangle, AppWindow, Copy, Download, ExternalLink, RotateCcw, Trash2 } from 'lucide-react';
import { OpenWithCandidate } from '../../api/snippets';
import { Snippet } from '../../types/snippet';
import { SelectField } from './SelectField';
import { FileTypeIcon, fileCategory, formatBytes, templateFileName } from './FileTypeIcon';

export interface TemplateFormFields {
  title: string;
  key: string;
  groupId: string | null;
  tags: string[];
  aliases: string[];
  pinned: boolean;
}

interface FileTemplateDetailProps {
  snippet: Snippet;
  groupOptions: Array<{ value: string; label: string }>;
  /** 该扩展名上次选择的打开方式（desktop id）。 */
  rememberedDesktopId?: string;
  /** 返回错误文案；成功返回 null。 */
  onSave: (fields: TemplateFormFields) => Promise<string | null>;
  onOpen: (desktopId?: string) => void;
  onLoadOpenWith: () => Promise<OpenWithCandidate[]>;
  onTrash: () => void;
  onRestore: () => void;
  onPermanentDelete: () => void;
}

const splitList = (value: string) => value.split(',').map(item => item.trim()).filter(Boolean);

/** 右栏文件模板视图：元数据表单、文件信息卡片、外部打开；不预览内容、没有敏感开关。 */
export const FileTemplateDetail: React.FC<FileTemplateDetailProps> = ({
  snippet,
  groupOptions,
  rememberedDesktopId,
  onSave,
  onOpen,
  onLoadOpenWith,
  onTrash,
  onRestore,
  onPermanentDelete,
}) => {
  const [title, setTitle] = useState(snippet.title);
  const [key, setKey] = useState(snippet.key);
  const [groupId, setGroupId] = useState(snippet.groupId ?? '');
  const [tags, setTags] = useState(snippet.tags.join(', '));
  const [aliases, setAliases] = useState(snippet.aliases.join(', '));
  const [pinned, setPinned] = useState(snippet.pinned);
  const [error, setError] = useState('');
  const [saving, setSaving] = useState(false);
  const [menu, setMenu] = useState<{ loading: boolean; items: OpenWithCandidate[]; error: string } | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  // 条目切换或后端回写（revision 变化）后，用最新数据重置表单。
  useEffect(() => {
    setTitle(snippet.title);
    setKey(snippet.key);
    setGroupId(snippet.groupId ?? '');
    setTags(snippet.tags.join(', '));
    setAliases(snippet.aliases.join(', '));
    setPinned(snippet.pinned);
    setError('');
    setMenu(null);
  }, [snippet.id, snippet.revision, snippet.title, snippet.key, snippet.groupId, snippet.tags, snippet.aliases, snippet.pinned]);

  useEffect(() => {
    if (!menu) return;
    const close = (event: MouseEvent) => {
      if (!menuRef.current?.contains(event.target as Node)) setMenu(null);
    };
    document.addEventListener('mousedown', close);
    return () => document.removeEventListener('mousedown', close);
  }, [menu]);

  const trashed = !!snippet.deletedAt;
  const fileName = templateFileName(snippet.title, snippet.fileExt);
  const dirty = useMemo(
    () => title !== snippet.title || key !== snippet.key || (groupId || null) !== snippet.groupId
      || tags !== snippet.tags.join(', ') || aliases !== snippet.aliases.join(', ') || pinned !== snippet.pinned,
    [title, key, groupId, tags, aliases, pinned, snippet],
  );

  const save = async () => {
    if (saving) return;
    setSaving(true);
    const message = await onSave({
      title: title.trim(),
      key: key.trim(),
      groupId: groupId || null,
      tags: splitList(tags),
      aliases: splitList(aliases),
      pinned,
    });
    setSaving(false);
    setError(message ?? '');
  };

  const openMenu = async () => {
    if (menu) {
      setMenu(null);
      return;
    }
    setMenu({ loading: true, items: [], error: '' });
    try {
      const items = await onLoadOpenWith();
      setMenu({ loading: false, items, error: '' });
    } catch (e) {
      setMenu({ loading: false, items: [], error: (e as { message?: string })?.message ?? '无法读取可用应用' });
    }
  };

  return (
    <div className="manager-editor-inner" data-testid="file-template-detail">
      <div className="manager-editor-toolbar">
        <div className="manager-editor-title-group">
          <span className="manager-snippet-mark file-mark" data-category={fileCategory(snippet.fileExt)} aria-hidden>
            <FileTypeIcon ext={snippet.fileExt} className="size-4" />
          </span>
          <div>
            <h1>{snippet.title}</h1>
            <span>
              {snippet.fileMissing ? '文件缺失' : trashed ? '已删除' : '文件模板'} · 文件以明文保存在模板目录
            </span>
          </div>
        </div>
        <div className="manager-editor-actions">
          {trashed ? (
            <>
              <button type="button" className="manager-button manager-button-secondary" onClick={onRestore}>
                <RotateCcw strokeWidth={1.5} className="size-3.5" aria-hidden />恢复
              </button>
              <button type="button" className="manager-button manager-button-danger" onClick={onPermanentDelete}>
                <Trash2 strokeWidth={1.5} className="size-3.5" aria-hidden />彻底删除
              </button>
            </>
          ) : (
            <button type="button" className="manager-icon-button manager-danger-icon" aria-label="放入回收站" title="放入回收站" onClick={onTrash}>
              <Trash2 strokeWidth={1.5} className="size-4" aria-hidden />
            </button>
          )}
        </div>
      </div>

      {snippet.fileMissing && (
        <div className="manager-missing-banner" role="status">
          <AlertTriangle strokeWidth={1.5} className="size-4" aria-hidden />
          <span>文件缺失：模板目录里找不到这个文件。放回原位后重新扫描即可自动恢复。</span>
        </div>
      )}

      <div className="manager-file-card" aria-label="文件信息">
        <span className="manager-snippet-mark file-mark" data-category={fileCategory(snippet.fileExt)} aria-hidden>
          <FileTypeIcon ext={snippet.fileExt} className="size-4" />
        </span>
        <div className="manager-file-meta">
          <strong data-testid="file-name">{fileName}</strong>
          <span>
            {formatBytes(snippet.fileSize)}
            {snippet.fileMtime ? ` · 修改于 ${new Date(snippet.fileMtime).toLocaleString()}` : ''}
            {snippet.fileAutoImported ? ' · 自动导入' : ''}
          </span>
        </div>
        {!trashed && (
          <div className="manager-file-actions">
            <button type="button" className="manager-button manager-button-primary" onClick={() => onOpen()}>
              <ExternalLink strokeWidth={1.5} className="size-3.5" aria-hidden />打开
            </button>
            <div className="manager-plus-menu" ref={menuRef}>
              <button
                type="button"
                className="manager-button manager-button-secondary"
                aria-haspopup="menu"
                aria-expanded={!!menu}
                onClick={openMenu}
              >
                <AppWindow strokeWidth={1.5} className="size-3.5" aria-hidden />打开方式…
              </button>
              {menu && (
                <div
                  className="manager-popover manager-popover-right"
                  role="menu"
                  aria-label="打开方式"
                  onKeyDown={event => { if (event.key === 'Escape') { event.stopPropagation(); setMenu(null); } }}
                >
                  {menu.loading && <div className="manager-popover-note">正在查找应用…</div>}
                  {menu.error && <div className="manager-popover-note" role="alert">{menu.error}</div>}
                  {!menu.loading && !menu.error && menu.items.length === 0 && (
                    <div className="manager-popover-note">没有找到可打开此类型的应用。</div>
                  )}
                  {menu.items.map(item => (
                    <button
                      key={item.desktopId}
                      type="button"
                      role="menuitem"
                      aria-current={item.desktopId === rememberedDesktopId ? 'true' : undefined}
                      className={item.desktopId === rememberedDesktopId ? 'is-default' : ''}
                      onClick={() => { setMenu(null); onOpen(item.desktopId); }}
                    >
                      <span>{item.name}</span>
                      {item.desktopId === rememberedDesktopId && <small>上次使用</small>}
                    </button>
                  ))}
                </div>
              )}
            </div>
            <button type="button" className="manager-button manager-button-secondary" disabled title="导出将在后续版本提供">
              <Download strokeWidth={1.5} className="size-3.5" aria-hidden />导出
            </button>
            <button type="button" className="manager-button manager-button-secondary" disabled title="带格式复制将在后续版本提供">
              <Copy strokeWidth={1.5} className="size-3.5" aria-hidden />带格式复制
            </button>
          </div>
        )}
      </div>

      <div className="manager-form-card">
        <section className="manager-form-section">
          <div className="manager-section-heading"><h2>基本信息</h2><span>标题就是文件名，改标题会同步重命名文件。</span></div>
          <div className="manager-form-grid">
            <label className="manager-field">
              <span>标题 <small>同时是文件名（不含扩展名）</small></span>
              <input type="text" className="manager-control" value={title} maxLength={100} disabled={trashed}
                aria-label="模板标题" onChange={event => setTitle(event.target.value)} />
            </label>
            <label className="manager-field">
              <span>Key <small>唯一快速检索标识</small></span>
              <input type="text" className={`manager-control manager-mono ${error ? 'has-error' : ''}`} value={key} disabled={trashed}
                aria-label="模板 Key" aria-invalid={!!error} onChange={event => { setKey(event.target.value); setError(''); }} />
            </label>
            <div className="manager-field">
              <span>分组 <small>移动分组会同步移动文件</small></span>
              <SelectField value={groupId} options={groupOptions} onChange={setGroupId} ariaLabel="分组" />
            </div>
            <label className="manager-field">
              <span>标签 <small>英文逗号分隔</small></span>
              <input type="text" className="manager-control" value={tags} disabled={trashed}
                aria-label="标签" onChange={event => setTags(event.target.value)} />
            </label>
            <label className="manager-field">
              <span>别名 <small>英文逗号分隔</small></span>
              <input type="text" className="manager-control" value={aliases} disabled={trashed}
                aria-label="别名" onChange={event => setAliases(event.target.value)} />
            </label>
          </div>
        </section>
        <section className="manager-form-section">
          <div className="manager-toggle-grid">
            <label className="manager-toggle-row">
              <span><strong>固定到顶部</strong><small>在检索列表中优先靠前显示</small></span>
              <input type="checkbox" className="manager-switch" checked={pinned} disabled={trashed}
                aria-label="固定到顶部" onChange={event => setPinned(event.target.checked)} />
            </label>
          </div>
        </section>
      </div>

      {error && <em className="manager-field-error" role="alert">{error}</em>}
      {!trashed && (
        <div className="manager-save-bar">
          <span>{dirty ? '有未保存的修改。' : '保存会同步更新文件名与所在目录。'}</span>
          <button type="button" className="manager-button manager-button-secondary manager-save-button" disabled={saving || (!dirty && !snippet.fileAutoImported)} onClick={save}>
            保存模板信息
          </button>
        </div>
      )}
    </div>
  );
};
