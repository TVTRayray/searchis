import React from 'react';
import { Keyboard } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Kbd } from '@/components/ui/kbd';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

interface KeyboardShortcutsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const KeyboardShortcutsModal: React.FC<KeyboardShortcutsModalProps> = ({
  isOpen,
  onClose,
}) => {
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
    <Dialog open={isOpen} onOpenChange={(open) => { if (!open) onClose(); }}>
      <DialogContent className="max-w-lg gap-0 p-0 overflow-hidden">
        <DialogHeader className="px-6 py-4 border-b text-left">
          <DialogTitle className="flex items-center gap-2 text-base">
            <Keyboard className="size-4 text-[color:var(--color-accent-fg)]" />
            Searchis 核心键盘交互速查表
          </DialogTitle>
        </DialogHeader>

        <div className="max-h-[460px] overflow-y-auto px-2 py-2">
          {shortcuts.map((item) => (
            <div
              key={item.key}
              className="flex items-center justify-between gap-4 rounded-lg px-3 py-2 transition-[color,background-color] hover:bg-accent/50"
            >
              <span className="text-xs text-theme-secondary font-medium">{item.desc}</span>
              <Kbd>{item.key}</Kbd>
            </div>
          ))}
        </div>

        <DialogFooter className="px-6 py-3 border-t">
          <Button onClick={onClose}>
            知道了 (Esc)
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
