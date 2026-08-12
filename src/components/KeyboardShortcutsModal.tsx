import React from 'react';
import { X, Keyboard } from 'lucide-react';
import {
  Box,
  VStack,
  HStack,
  IconButton,
  Button,
  Kbd,
  List,
  ListItem,
} from './astryx';

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
    <Box
      padding="md"
      className="fixed inset-0 z-50 flex items-center justify-center theme-backdrop backdrop-blur-xs select-none"
      onClick={onClose}
    >
      <Box
        width="100%"
        width-max="512px"
        radius="xl"
        shadow="window"
        background="surface"
        border="all"
        overflow="hidden"
        className="raycast-window max-w-lg animate-pop-in"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Title bar */}
        <Box paddingX="lg" paddingY="md" background="titlebar" border="bottom">
          <HStack align="center" justify="space-between">
            <HStack align="center" gap="xs">
              <Keyboard className="w-4 h-4 icon-accent" />
              <span className="text-sm font-bold text-theme">Searchis 核心键盘交互速查表</span>
            </HStack>
            <IconButton
              icon={<X className="w-4 h-4" />}
              ariaLabel="关闭速查表"
              onClick={onClose}
            />
          </HStack>
        </Box>

        {/* Shortcuts list using Astryx List */}
        <Box padding="md" overflow="auto" className="max-h-[460px]">
          <List divided>
            {shortcuts.map((item) => (
              <ListItem
                key={item.key}
                title={<span className="text-xs text-theme-secondary font-medium">{item.desc}</span>}
                extra={<Kbd>{item.key}</Kbd>}
              />
            ))}
          </List>
        </Box>

        {/* Footer */}
        <Box paddingX="lg" paddingY="md" background="titlebar" border="top">
          <HStack justify="flex-end">
            <Button variant="primary" size="sm" onClick={onClose}>
              知道了 (Esc)
            </Button>
          </HStack>
        </Box>
      </Box>
    </Box>
  );
};
