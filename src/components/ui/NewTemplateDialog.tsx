import React, { useEffect, useRef, useState } from 'react';

interface NewTemplateDialogProps {
  kind: 'md' | 'xlsx';
  /** 返回错误文案；成功返回 null。 */
  onSubmit: (title: string, key: string) => Promise<string | null>;
  onClose: () => void;
}

/** 由标题推导 Key 建议：保留字母数字（含中文），其余换成 `-`。 */
const suggestKey = (title: string): string =>
  title
    .toLowerCase()
    .replace(/[^\p{L}\p{N}_.]+/gu, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 60);

export const NewTemplateDialog: React.FC<NewTemplateDialogProps> = ({ kind, onSubmit, onClose }) => {
  const [title, setTitle] = useState('');
  const [key, setKey] = useState('');
  const [keyTouched, setKeyTouched] = useState(false);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const titleRef = useRef<HTMLInputElement>(null);

  useEffect(() => { titleRef.current?.focus(); }, []);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (busy) return;
    setBusy(true);
    const message = await onSubmit(title, key);
    setBusy(false);
    if (message) setError(message);
  };

  return (
    <div className="manager-confirm-backdrop" role="presentation">
      <form
        className="manager-confirm-dialog manager-group-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="new-template-title"
        onSubmit={submit}
        onKeyDown={event => { if (event.key === 'Escape') { event.stopPropagation(); onClose(); } }}
      >
        <h2 id="new-template-title">新建空白 {kind === 'md' ? 'Markdown' : 'Excel'} 文件</h2>
        <label className="manager-field">
          <span>标题 <small>同时作为文件名（.{kind}）</small></span>
          <input
            ref={titleRef}
            type="text"
            value={title}
            maxLength={100}
            aria-label="新模板标题"
            className="manager-control"
            placeholder="例如 周报模板"
            onChange={event => {
              setTitle(event.target.value);
              if (!keyTouched) setKey(suggestKey(event.target.value));
              setError('');
            }}
          />
        </label>
        <label className="manager-field">
          <span>Key <small>唯一快速检索标识</small></span>
          <input
            type="text"
            value={key}
            aria-label="新模板 Key"
            className={`manager-control manager-mono ${error ? 'has-error' : ''}`}
            aria-invalid={!!error}
            onChange={event => { setKey(event.target.value); setKeyTouched(true); setError(''); }}
          />
        </label>
        {error && <em className="manager-field-error" role="alert">{error}</em>}
        <div className="manager-confirm-actions">
          <button type="button" className="manager-button manager-button-secondary" onClick={onClose}>取消</button>
          <button type="submit" className="manager-button manager-button-primary" disabled={busy}>创建</button>
        </div>
      </form>
    </div>
  );
};
