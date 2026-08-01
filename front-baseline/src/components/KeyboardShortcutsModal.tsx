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
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40">
      <div className="w-full max-w-lg rounded-lg bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 shadow-lg overflow-hidden">
        
        {/* Title */}
        <div className="h-11 px-4 bg-zinc-50 dark:bg-zinc-950 border-b border-zinc-200 dark:border-zinc-800 flex items-center justify-between">
          <span className="text-sm font-bold text-zinc-800 dark:text-zinc-200 flex items-center gap-2">
            <Keyboard className="w-4 h-4 text-blue-500" />
            <span>Searchis 核心键盘交互速查表</span>
          </span>
          <button
            onClick={onClose}
            className="p-1 text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Content */}
        <div className="p-5 space-y-2 max-h-[460px] overflow-y-auto">
          {shortcuts.map((item, idx) => (
            <div
              key={idx}
              className="flex items-center justify-between p-3 rounded-md bg-zinc-50 dark:bg-zinc-950 border border-zinc-200 dark:border-zinc-800 text-sm"
            >
              <span className="text-zinc-700 dark:text-zinc-300 font-medium">
                {item.desc}
              </span>
              <span className="font-mono text-xs font-bold text-blue-600 dark:text-blue-400 bg-white dark:bg-zinc-800 border border-zinc-300 dark:border-zinc-700 px-2 py-1 rounded">
                {item.key}
              </span>
            </div>
          ))}
        </div>

        {/* Footer */}
        <div className="px-5 py-3 bg-zinc-50 dark:bg-zinc-950 border-t border-zinc-200 dark:border-zinc-800 text-right">
          <button
            onClick={onClose}
            className="px-4 py-1.5 rounded-md bg-blue-600 hover:bg-blue-500 text-white text-sm font-semibold"
          >
            知道了 (Esc)
          </button>
        </div>

      </div>
    </div>
  );
};
