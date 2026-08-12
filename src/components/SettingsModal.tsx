import React, { useEffect, useState } from 'react';
import { SettingsConfig } from '../api/snippets';
import { settingsApi } from '../api/snippets';
import {
  Settings, Keyboard, Sun, Moon, Monitor,
  HardDrive, Download, Upload, RotateCcw, Zap, X
} from 'lucide-react';
import {
  Box, VStack, HStack, Inline, Card, Button, IconButton,
  Switch, Kbd, useTheme,
} from './astryx';

interface SettingsModalProps {
  config: SettingsConfig;
  onUpdateConfig: (key: string, value: unknown) => void;
  onExportData: () => void;
  onImportData: (e: React.ChangeEvent<HTMLInputElement>) => void;
  onResetSampleData: () => void;
  onClose: () => void;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({
  config, onUpdateConfig, onExportData, onImportData, onResetSampleData, onClose,
}) => {
  const { setTheme } = useTheme();
  const [autostart, setAutostart] = useState(false);

  useEffect(() => {
    settingsApi.get().then(res => setAutostart(res.autostartActual)).catch(() => {});
  }, []);

  const handleThemeChange = (newTheme: 'light' | 'dark' | 'system') => {
    onUpdateConfig('theme', newTheme);
    setTheme(newTheme);
  };

  return (
    <Box padding="md" className="max-w-2xl mx-auto select-none">
      <Box radius="xl" shadow="window" background="surface" border="all" overflow="hidden"
        className="raycast-window animate-pop-in">
        <Box paddingX="lg" paddingY="md" background="titlebar" border="bottom">
          <HStack align="center" justify="space-between">
            <HStack align="center" gap="xs">
              <Settings className="w-4 h-4 icon-accent" />
              <span className="text-sm font-bold text-theme">Searchis 偏好设置</span>
            </HStack>
            <IconButton icon={<X className="w-4 h-4" />} ariaLabel="关闭设置窗口" onClick={onClose} />
          </HStack>
        </Box>

        <Box padding="xl" className="max-h-[540px] overflow-y-auto">
          <VStack gap="lg">
            {/* 快捷键 */}
            <Card title={<HStack align="center" gap="xs"><Keyboard className="w-4 h-4 icon-accent" /><span>全局快捷键与呼出</span></HStack>}>
              <VStack gap="md">
                <HStack align="center" justify="space-between">
                  <VStack gap="3xs">
                    <span className="text-xs font-semibold text-theme">全局呼出快捷键</span>
                    <span className="text-[11px] text-theme-muted">设置后重启应用生效</span>
                  </VStack>
                  <HStack align="center" gap="xs">
                    <input
                      type="text"
                      value={config.globalShortcut}
                      onChange={e => onUpdateConfig('globalShortcut', e.target.value)}
                      className="input-theme w-32 px-2 py-1 rounded text-xs text-center font-mono"
                      placeholder="例: Alt+O"
                      aria-label="全局呼出快捷键"
                    />
                  </HStack>
                </HStack>
                <Box border="top" paddingY="2xs" />
                <Switch
                  checked={config.launchAtLogin}
                  onChange={v => {
                    onUpdateConfig('launchAtLogin', v);
                    // 同时操作 XDG Autostart .desktop 文件
                    settingsApi.autostartSet(v).catch(() => {});
                    // 更新实际状态显示
                    setAutostart(v);
                  }}
                  label="开机自动启动 (XDG Autostart)" description={`当前状态: ${autostart ? '已启用' : '未启用'}`} />
              </VStack>
            </Card>

            {/* 自动粘贴 */}
            <Card title={<HStack align="center" gap="xs"><Zap className="w-4 h-4 icon-accent" /><span>粘贴与恢复</span></HStack>}>
              <VStack gap="md">
                <Switch checked={config.autoPaste} onChange={v => onUpdateConfig('autoPaste', v)}
                  label="自动粘贴" description="Enter 选定后自动复制并发送 Ctrl+V 粘贴到上一个应用" />
                <Box border="top" paddingY="2xs" />
                <Box padding="md" radius="md" background="subtle" border="all">
                  <span className="text-xs text-theme-muted">
                    剪贴板恢复功能在 V1 中不启用（PRD 硬约束 restoreClipboard=false）。
                  </span>
                </Box>
              </VStack>
            </Card>

            {/* 主题 */}
            <Card title={<HStack align="center" gap="xs"><Sun className="w-4 h-4 icon-accent" /><span>外观主题</span></HStack>}>
              <HStack align="center" justify="space-between">
                <span className="text-xs font-semibold text-theme">界面色彩风格</span>
                <Inline gap="3xs" className="p-1 rounded-xl theme-surface-subtle border theme-divider">
                  {([ ['light', Sun, '浅色'] as const, ['dark', Moon, '深色'] as const, ['system', Monitor, '跟随系统'] as const ]).map(([t, Icon, label]) => (
                    <button key={t} type="button" onClick={() => handleThemeChange(t)}
                      className={`flex items-center gap-1 px-2.5 py-1 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
                        config.theme === t ? 'nav-active shadow-xs' : 'interactive-muted'}`}>
                      <Icon className="w-3.5 h-3.5" /><span>{label}</span>
                    </button>
                  ))}
                </Inline>
              </HStack>
            </Card>

            {/* 最大结果数 */}
            <Card title={<HStack align="center" gap="xs"><Settings className="w-4 h-4 icon-accent" /><span>检索与数据</span></HStack>}>
              <VStack gap="md">
                <HStack align="center" justify="space-between">
                  <span className="text-xs font-semibold text-theme">最大结果数</span>
                  <select value={config.maxResultsCount} onChange={e => onUpdateConfig('maxResultsCount', parseInt(e.target.value))}
                    className="bg-transparent border-none text-theme font-medium outline-none cursor-pointer">
                    {[5,10,20,50,100].map(n => <option key={n} value={n}>{n}</option>)}
                  </select>
                </HStack>
                <Box border="top" paddingY="2xs" />
                <HStack align="center" justify="space-between">
                  <span className="text-xs font-semibold text-theme">回收站自动清理</span>
                  <select value={config.trashAutoPurgeDays ?? ''} onChange={e => {
                    const v = e.target.value ? parseInt(e.target.value) : null;
                    onUpdateConfig('trashAutoPurgeDays', v);
                  }}
                    className="bg-transparent border-none text-theme font-medium outline-none cursor-pointer">
                    <option value="">关闭</option>
                    <option value="7">7 天</option>
                    <option value="30">30 天</option>
                    <option value="90">90 天</option>
                  </select>
                </HStack>
              </VStack>
            </Card>

            {/* 数据备份 */}
            <Card title={<HStack align="center" gap="xs"><HardDrive className="w-4 h-4 icon-accent" /><span>数据备份与还原</span></HStack>}>
              <VStack gap="md">
                <HStack align="center" justify="space-between">
                  <span className="text-xs font-semibold text-theme">导出 JSON 备份</span>
                  <Button variant="secondary" size="sm" icon={<Download className="w-3.5 h-3.5" />} onClick={onExportData}>导出</Button>
                </HStack>
                <Box border="top" paddingY="2xs" />
                <HStack align="center" justify="space-between">
                  <span className="text-xs font-semibold text-theme">导入 JSON 备份</span>
                  <label className="btn-secondary flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold cursor-pointer">
                    <Upload className="w-3.5 h-3.5" /><span>选择文件</span>
                    <input type="file" accept=".json" onChange={onImportData} className="hidden" />
                  </label>
                </HStack>
                <Box border="top" paddingY="2xs" />
                <HStack align="center" justify="space-between">
                  <span className="text-xs font-semibold text-danger">重置内置示例数据</span>
                  <Button variant="danger" size="sm" icon={<RotateCcw className="w-3.5 h-3.5" />} onClick={onResetSampleData}>重置</Button>
                </HStack>
              </VStack>
            </Card>
          </VStack>
        </Box>
      </Box>
    </Box>
  );
};
