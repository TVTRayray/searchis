import React, { useEffect, useRef, useState } from 'react';
import { FileSpreadsheet, FileText, Plus, Upload, Type } from 'lucide-react';

interface PlusMenuProps {
  onNewText: () => void;
  onNewMarkdown: () => void;
  onNewXlsx: () => void;
  onImport: () => void;
}

/** 中栏“+”菜单：新建文本片段 / 空白 md / 空白 xlsx / 导入文件。 */
export const PlusMenu: React.FC<PlusMenuProps> = ({ onNewText, onNewMarkdown, onNewXlsx, onImport }) => {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const close = (event: MouseEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', close);
    return () => document.removeEventListener('mousedown', close);
  }, [open]);

  const choose = (action: () => void) => () => {
    setOpen(false);
    action();
  };

  return (
    <div
      className="manager-plus-menu"
      ref={rootRef}
      onKeyDown={event => { if (event.key === 'Escape' && open) { event.stopPropagation(); setOpen(false); } }}
    >
      <button
        type="button"
        className="manager-add-button"
        aria-label="新建或导入"
        title="新建或导入"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen(value => !value)}
      >
        <Plus strokeWidth={1.5} className="size-4" aria-hidden />
      </button>
      {open && (
        <div className="manager-popover" role="menu" aria-label="新建或导入">
          <button type="button" role="menuitem" autoFocus onClick={choose(onNewText)}>
            <Type strokeWidth={1.5} className="size-3.5" aria-hidden />新建文本片段
          </button>
          <button type="button" role="menuitem" onClick={choose(onNewMarkdown)}>
            <FileText strokeWidth={1.5} className="size-3.5" aria-hidden />新建空白 md
          </button>
          <button type="button" role="menuitem" onClick={choose(onNewXlsx)}>
            <FileSpreadsheet strokeWidth={1.5} className="size-3.5" aria-hidden />新建空白 xlsx
          </button>
          <button type="button" role="menuitem" onClick={choose(onImport)}>
            <Upload strokeWidth={1.5} className="size-3.5" aria-hidden />导入文件…
          </button>
        </div>
      )}
    </div>
  );
};
