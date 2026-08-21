import React from 'react';
import { Keyboard, X } from 'lucide-react';

interface KeyboardShortcutsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

const shortcuts = [
  { key: 'Alt + O', desc: '全局呼出 / 隐藏 Searchis 快速检索窗口' },
  { key: '↑ / ↓ 或 J / K', desc: '在匹配结果列表中上下移动选中项' },
  { key: 'Enter (↵)', desc: '自动写入剪贴板、发送粘贴按键并关闭窗口' },
  { key: 'Ctrl + Enter (Ctrl↵)', desc: '仅复制到剪贴板，不触发自动粘贴与关闭' },
  { key: 'Ctrl + 1～9', desc: '按下数字键直接选择对应索引的文本片段' },
  { key: 'Ctrl + E', desc: '快捷打开当前选中片段的编辑界面' },
  { key: 'Ctrl + N', desc: '以当前输入的搜索词作为 Key 新建片段' },
  { key: 'Esc', desc: '关闭窗口' },
];

export const KeyboardShortcutsModal: React.FC<KeyboardShortcutsModalProps> = ({
  isOpen,
  onClose,
}) => {
  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-3 theme-backdrop backdrop-blur-xs select-none"
      onClick={onClose}
    >
      <section
        className="raycast-window max-w-lg w-full rounded-xl overflow-hidden animate-pop-in"
        role="dialog"
        aria-modal="true"
        aria-labelledby="keyboard-shortcuts-title"
        onClick={event => event.stopPropagation()}
      >
        <header className="flex items-center justify-between px-5 py-3 theme-titlebar border-b">
          <div className="flex items-center gap-2">
            <Keyboard className="w-4 h-4 icon-accent" aria-hidden />
            <h2 id="keyboard-shortcuts-title" className="text-sm font-bold text-theme">Searchis 核心键盘交互速查表</h2>
          </div>
          <button type="button" className="manager-icon-button" aria-label="关闭速查表" title="关闭速查表" onClick={onClose}>
            <X className="w-4 h-4" aria-hidden />
          </button>
        </header>

        <ul className="max-h-[460px] overflow-auto divide-y theme-divider-subtle" style={{ margin: 0, padding: 0, listStyle: 'none' }}>
          {shortcuts.map(item => (
            <li key={item.key} className="w-full flex items-center justify-between gap-3 px-3 py-2.5 text-xs">
              <span className="text-xs text-theme-secondary font-medium">{item.desc}</span>
              <kbd className="raycast-kbd">{item.key}</kbd>
            </li>
          ))}
        </ul>

        <footer className="flex justify-end px-5 py-3 theme-titlebar border-t">
          <button type="button" className="btn-primary inline-flex items-center justify-center gap-1.5 cursor-pointer transition-all px-2.5 py-1 text-xs rounded-md" onClick={onClose}>
            知道了 (Esc)
          </button>
        </footer>
      </section>
    </div>
  );
};
