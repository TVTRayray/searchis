import React from 'react';
import { Keyboard, X } from 'lucide-react';

interface KeyboardShortcutsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

interface ShortcutItem {
  key: string;
  desc: string;
}

interface ShortcutGroup {
  title: string;
  items: ShortcutItem[];
}

/** 按“通用 / 文本片段 / 文件模板”分组，与快搜窗口底栏及 PRD FR-PCK-02 v0.4 一致。 */
const shortcutGroups: ShortcutGroup[] = [
  {
    title: '通用',
    items: [
      { key: 'Alt + O', desc: '全局呼出 / 隐藏 Searchis 快速检索窗口' },
      { key: '↑ / ↓ 或 J / K', desc: '在匹配结果列表中上下移动选中项' },
      { key: 'Ctrl + E', desc: '编辑当前选中的条目' },
      { key: 'Ctrl + N', desc: '以当前输入的搜索词作为 Key 新建片段' },
      { key: 'Esc', desc: '关闭窗口' },
    ],
  },
  {
    title: '文本片段',
    items: [
      { key: 'Enter (↵)', desc: '复制到剪贴板并关闭窗口' },
      { key: 'Ctrl + Enter (Ctrl↵)', desc: '复制并自动粘贴到原窗口（可在设置中关闭自动粘贴）' },
      { key: 'Ctrl + 1～9', desc: '直接粘贴对应序号的文本片段' },
    ],
  },
  {
    title: '文件模板',
    items: [
      { key: 'Enter (↵)', desc: '带格式复制（可直接粘贴到 WPS、文件管理器等）并关闭窗口' },
      { key: 'Ctrl + Enter (Ctrl↵)', desc: '带格式复制并自动粘贴到原窗口' },
      { key: 'Shift + Enter (Shift↵)', desc: '导出一份副本到导出目录，窗口保持打开' },
      { key: 'Ctrl + O', desc: '用默认软件打开文件并关闭窗口' },
      { key: 'Ctrl + 1～9', desc: '带格式复制并自动粘贴对应序号的文件模板' },
    ],
  },
];

export const KeyboardShortcutsModal: React.FC<KeyboardShortcutsModalProps> = ({
  isOpen,
  onClose,
}) => {
  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-3 theme-backdrop select-none"
      onClick={onClose}
    >
      <section
        className="raycast-window max-w-lg w-full overflow-hidden animate-pop-in"
        role="dialog"
        aria-modal="true"
        aria-labelledby="keyboard-shortcuts-title"
        onClick={event => event.stopPropagation()}
      >
        <header className="flex items-center justify-between px-5 py-3 theme-titlebar border-b">
          <div className="flex items-center gap-2">
            <Keyboard strokeWidth={1.5} className="w-4 h-4 icon-accent" aria-hidden />
            <h2 id="keyboard-shortcuts-title" className="modal-heading">Searchis 核心键盘交互速查表</h2>
          </div>
          <button type="button" className="manager-icon-button" aria-label="关闭速查表" title="关闭速查表" onClick={onClose}>
            <X strokeWidth={1.5} className="w-4 h-4" aria-hidden />
          </button>
        </header>

        <div className="max-h-[460px] overflow-auto">
          {shortcutGroups.map(group => (
            <section key={group.title} aria-label={group.title} data-testid="shortcut-group">
              <h3 className="px-5 pt-3 pb-1 text-[11.5px] font-semibold text-theme-secondary">{group.title}</h3>
              <ul className="divide-y theme-divider-subtle" style={{ margin: 0, padding: 0, listStyle: 'none' }}>
                {group.items.map(item => (
                  <li key={item.key} className="w-full flex items-center justify-between gap-3 px-5 py-2 text-[12.5px]">
                    <span className="text-theme-secondary">{item.desc}</span>
                    <kbd className="raycast-kbd">{item.key}</kbd>
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </div>

        <footer className="flex justify-end px-5 py-3 theme-titlebar border-t">
          <button type="button" className="btn-primary" onClick={onClose}>
            知道了 (Esc)
          </button>
        </footer>
      </section>
    </div>
  );
};
