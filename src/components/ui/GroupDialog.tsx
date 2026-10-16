import React, { useEffect, useMemo, useRef, useState } from 'react';
import { Group } from '../../types/snippet';
import { SelectField } from './SelectField';

interface GroupDialogProps {
  /** 编辑已有分组；为空表示新建。 */
  group: Group | null;
  initialParentId: string | null;
  groups: Group[];
  onSubmit: (name: string, parentId: string | null) => Promise<string | null>;
  onClose: () => void;
}

/** 新建 / 重命名 / 移动分组。错误就地显示，不写入时保留输入。 */
export const GroupDialog: React.FC<GroupDialogProps> = ({ group, initialParentId, groups, onSubmit, onClose }) => {
  const [name, setName] = useState(group?.name ?? '');
  const [parentId, setParentId] = useState<string>(group ? group.parentId ?? '' : initialParentId ?? '');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  // 可选上级：排除自身与子孙；子树移动后不超过 3 层。
  const parentOptions = useMemo(() => {
    const byId = new Map(groups.map(item => [item.id, item]));
    const descendants = new Set<string>();
    let subtreeHeight = 1;
    if (group) {
      const collect = (id: string, level: number) => {
        subtreeHeight = Math.max(subtreeHeight, level);
        for (const child of groups) {
          if (child.parentId === id) {
            descendants.add(child.id);
            collect(child.id, level + 1);
          }
        }
      };
      collect(group.id, 1);
    }
    const options = [{ value: '', label: '（顶层）' }];
    const walk = (parent: string | null) => {
      for (const item of groups) {
        if ((item.parentId ?? null) !== parent) continue;
        const blocked = group && (item.id === group.id || descendants.has(item.id));
        if (!blocked && item.depth + subtreeHeight <= 3) {
          options.push({ value: item.id, label: `${'　'.repeat(item.depth - 1)}${item.name}` });
        }
        if (!blocked) walk(item.id);
      }
    };
    walk(null);
    return { options, byId };
  }, [groups, group]);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (busy) return;
    setBusy(true);
    const message = await onSubmit(name, parentId || null);
    setBusy(false);
    if (message) setError(message);
  };

  return (
    <div className="manager-confirm-backdrop" role="presentation">
      <form
        className="manager-confirm-dialog manager-group-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="group-dialog-title"
        onSubmit={submit}
        onKeyDown={event => { if (event.key === 'Escape') { event.stopPropagation(); onClose(); } }}
      >
        <h2 id="group-dialog-title">{group ? '重命名或移动分组' : '新建分组'}</h2>
        <label className="manager-field">
          <span>名称</span>
          <input
            ref={inputRef}
            type="text"
            value={name}
            maxLength={64}
            onChange={event => { setName(event.target.value); setError(''); }}
            className={`manager-control ${error ? 'has-error' : ''}`}
            aria-label="分组名称"
            aria-invalid={!!error}
            placeholder="例如 运维、财务"
          />
        </label>
        <div className="manager-field">
          <span>上级分组</span>
          <SelectField
            value={parentId}
            options={parentOptions.options}
            onChange={setParentId}
            ariaLabel="上级分组"
          />
        </div>
        {error && <em className="manager-field-error" role="alert">{error}</em>}
        <div className="manager-confirm-actions">
          <button type="button" className="manager-button manager-button-secondary" onClick={onClose}>取消</button>
          <button type="submit" className="manager-button manager-button-primary" disabled={busy}>{group ? '保存' : '创建'}</button>
        </div>
      </form>
    </div>
  );
};
