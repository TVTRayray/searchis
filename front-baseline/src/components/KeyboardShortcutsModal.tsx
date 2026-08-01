import React from 'react';
import { X, Keyboard } from 'lucide-react';

interface KeyboardShortcutsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const KeyboardShortcutsModal: React.FC<KeyboardShortcutsModalProps> = ({
  isOpen,
  onClose,
}) => {
  if (!isOpen) return null;

  const shortcuts = [
    { key: '⌥ + Space', desc: '全局呼出 / 隐藏 Searchis 快速检索窗口' },
    { key: '↑ / ↓ 或 J / K', desc: '在匹配结果列表中上下移动选中项' },
    { key: 'Enter (↵)', desc: '自动写入剪贴板、发送粘贴按键并关闭窗口' },
    { key: '⌘ + Enter (⌘↵)', desc: '仅复制到剪贴板，不触发自动粘贴与关闭' },
    { key: '⌘ + 1～9', desc: '按下数字键直接选择对应索引的文本片段' },
    { key: '⌘ + E', desc: '快捷打开当前选中片段的编辑界面' },
    { key: '⌘ + N', desc: '以当前输入的搜索词作为 Key 新建片段' },
    { key: 'Tab', desc: '展开或收起右侧文本片段预览与详细参数' },
    { key: 'Esc', desc: '关闭窗口' },
  ];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 theme-backdrop backdrop-blur-xs animate-pop-in">
      <div className="w-full max-w-lg rounded-2xl raycast-window overflow-hidden">
        <div className="h-11 px-4 theme-titlebar border-b theme-divider flex items-center justify-between">
          <span className="text-sm font-bold text-theme flex items-center gap-2">
            <Keyboard className="w-4 h-4 icon-accent" />
            <span>Searchis 核心键盘交互速查表</span>
          </span>
          <button onClick={onClose} className="p-1 rounded-md interactive-muted" aria-label="关闭快捷键速查表">
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="p-5 space-y-2 max-h-[460px] overflow-y-auto">
          {shortcuts.map((item) => (
            <div key={item.key} className="flex items-center justify-between gap-4 p-3 rounded-xl theme-surface-subtle border theme-divider-subtle text-sm">
              <span className="text-theme-secondary font-medium">{item.desc}</span>
              <kbd className="raycast-kbd whitespace-nowrap">{item.key}</kbd>
            </div>
          ))}
        </div>

        <div className="px-5 py-3 theme-titlebar border-t theme-divider text-right">
          <button onClick={onClose} className="btn-primary px-4 py-1.5 rounded-xl text-sm font-semibold transition-colors">
            知道了 (Esc)
          </button>
        </div>
      </div>
    </div>
  );
};
