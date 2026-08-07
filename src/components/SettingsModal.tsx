import React from 'react';
import { SettingsConfig } from '../types/snippet';
import {
  Settings,
  Keyboard,
  Sun,
  Moon,
  Monitor,
  HardDrive,
  Download,
  Upload,
  RotateCcw,
  Zap,
  Volume2,
  VolumeX,
  X
} from 'lucide-react';
import {
  Box,
  VStack,
  HStack,
  Inline,
  Card,
  Button,
  IconButton,
  Switch,
  Kbd,
  useTheme,
} from './astryx';

interface SettingsModalProps {
  config: SettingsConfig;
  onUpdateConfig: (updater: (prev: SettingsConfig) => SettingsConfig) => void;
  onExportData: () => void;
  onImportData: (e: React.ChangeEvent<HTMLInputElement>) => void;
  onResetSampleData: () => void;
  onClose: () => void;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({
  config,
  onUpdateConfig,
  onExportData,
  onImportData,
  onResetSampleData,
  onClose,
}) => {
  const { setTheme } = useTheme();

  const handleThemeChange = (newTheme: 'light' | 'dark' | 'system') => {
    onUpdateConfig(prev => ({ ...prev, theme: newTheme }));
    setTheme(newTheme);
  };

  return (
    <Box padding="md" className="max-w-2xl mx-auto select-none">
      <Box
        radius="xl"
        shadow="window"
        background="surface"
        border="all"
        overflow="hidden"
        className="raycast-window animate-pop-in"
      >
        {/* Title Bar */}
        <Box paddingX="lg" paddingY="md" background="titlebar" border="bottom">
          <HStack align="center" justify="space-between">
            <HStack align="center" gap="xs">
              <Settings className="w-4 h-4 icon-accent" />
              <span className="text-sm font-bold text-theme">Searchis 偏好设置</span>
            </HStack>
            <IconButton
              icon={<X className="w-4 h-4" />}
              ariaLabel="关闭设置窗口"
              onClick={onClose}
            />
          </HStack>
        </Box>

        {/* Settings Body using Astryx Cards */}
        <Box padding="xl" className="max-h-[540px] overflow-y-auto">
          <VStack gap="lg">
            {/* Section 1: Hotkey & Startup */}
          <Card
            title={
              <HStack align="center" gap="xs">
                <Keyboard className="w-4 h-4 icon-accent" />
                <span>全局快捷键与呼出</span>
              </HStack>
            }
          >
            <VStack gap="md">
              <HStack align="center" justify="space-between">
                <VStack gap="3xs">
                  <span className="text-xs font-semibold text-theme">全局呼出快捷键</span>
                  <span className="text-[11px] text-theme-muted">按下组合键无缝在桌面层级置顶快速检索浮层</span>
                </VStack>
                <Inline gap="3xs">
                  <Kbd>⌥</Kbd>
                  <Kbd>Space</Kbd>
                </Inline>
              </HStack>

              <Box border="top" paddingY="2xs" />

              <Switch
                checked={config.launchAtLogin}
                onChange={checked => onUpdateConfig(prev => ({ ...prev, launchAtLogin: checked }))}
                label="开机自动启动"
                description="系统登录后后台静默运行系统托盘与全局按键监听"
              />
            </VStack>
          </Card>

          {/* Section 2: Paste & Audio */}
          <Card
            title={
              <HStack align="center" gap="xs">
                <Zap className="w-4 h-4 icon-accent" />
                <span>自动粘贴与提示音效</span>
              </HStack>
            }
          >
            <VStack gap="md">
              <Switch
                checked={config.autoPaste}
                onChange={checked => onUpdateConfig(prev => ({ ...prev, autoPaste: checked }))}
                label="自动模拟粘贴"
                description="在快速检索窗口按 Enter 选定片段后，自动复制并发送 Cmd+V 粘贴到上一个应用"
              />

              <Box border="top" paddingY="2xs" />

              <HStack align="center" justify="space-between">
                <VStack gap="3xs">
                  <span className="text-xs font-semibold text-theme">操作音效反馈</span>
                  <span className="text-[11px] text-theme-muted">复制或成功粘贴时播放极简轻柔提示音</span>
                </VStack>
                <Button
                  variant={config.playAudioFeedback ? 'primary' : 'secondary'}
                  size="sm"
                  icon={config.playAudioFeedback ? <Volume2 className="w-3.5 h-3.5" /> : <VolumeX className="w-3.5 h-3.5" />}
                  onClick={() => onUpdateConfig(prev => ({ ...prev, playAudioFeedback: !prev.playAudioFeedback }))}
                >
                  {config.playAudioFeedback ? '已开启' : '已关闭'}
                </Button>
              </HStack>
            </VStack>
          </Card>

          {/* Section 3: Theme Appearance */}
          <Card
            title={
              <HStack align="center" gap="xs">
                <Sun className="w-4 h-4 icon-accent" />
                <span>外观主题模式 (支持深浅两色)</span>
              </HStack>
            }
          >
            <HStack align="center" justify="space-between">
              <VStack gap="3xs">
                <span className="text-xs font-semibold text-theme">界面色彩风格</span>
                <span className="text-[11px] text-theme-muted">完美适配浅色 (Porcelain) 与深色 (Obsidian)</span>
              </VStack>

              <Inline gap="3xs" className="p-1 rounded-xl theme-surface-subtle border theme-divider">
                {([
                  ['light', Sun, '浅色'],
                  ['dark', Moon, '深色'],
                  ['system', Monitor, '跟随系统'],
                ] as const).map(([t, Icon, label]) => (
                  <button
                    key={t}
                    type="button"
                    onClick={() => handleThemeChange(t)}
                    className={`flex items-center gap-1 px-2.5 py-1 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
                      config.theme === t ? 'nav-active shadow-xs' : 'interactive-muted'
                    }`}
                  >
                    <Icon className="w-3.5 h-3.5" />
                    <span>{label}</span>
                  </button>
                ))}
              </Inline>
            </HStack>
          </Card>

          {/* Section 4: Local Data Backup */}
          <Card
            title={
              <HStack align="center" gap="xs">
                <HardDrive className="w-4 h-4 icon-accent" />
                <span>数据备份与还原 (单机本地优先)</span>
              </HStack>
            }
          >
            <VStack gap="md">
              <HStack align="center" justify="space-between">
                <VStack gap="3xs">
                  <span className="text-xs font-semibold text-theme">导出全量 JSON 数据</span>
                  <span className="text-[11px] text-theme-muted">安全备份文本片段库到本地 UTF-8 明文 JSON 文件</span>
                </VStack>
                <Button
                  variant="secondary"
                  size="sm"
                  icon={<Download className="w-3.5 h-3.5" />}
                  onClick={onExportData}
                >
                  导出 JSON
                </Button>
              </HStack>

              <Box border="top" paddingY="2xs" />

              <HStack align="center" justify="space-between">
                <VStack gap="3xs">
                  <span className="text-xs font-semibold text-theme">导入 JSON 备份数据</span>
                  <span className="text-[11px] text-theme-muted">校验格式与 Schema 后合并恢复片段数据</span>
                </VStack>
                <label className="btn-secondary flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold cursor-pointer transition-all">
                  <Upload className="w-3.5 h-3.5" />
                  <span>选择文件导入</span>
                  <input type="file" accept=".json" onChange={onImportData} className="hidden" />
                </label>
              </HStack>

              <Box border="top" paddingY="2xs" />

              <HStack align="center" justify="space-between">
                <VStack gap="3xs">
                  <span className="text-xs font-semibold text-danger">重置初始内置数据</span>
                  <span className="text-[11px] text-theme-muted">恢复默认提供的常用 Key 文本演示模板</span>
                </VStack>
                <Button
                  variant="danger"
                  size="sm"
                  icon={<RotateCcw className="w-3.5 h-3.5" />}
                  onClick={onResetSampleData}
                >
                  重置示例数据
                </Button>
              </HStack>
            </VStack>
          </Card>
          </VStack>
        </Box>
      </Box>
    </Box>
  );
};
