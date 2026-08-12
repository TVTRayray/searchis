import React from 'react';
import type { SettingsConfig } from '../api/snippets';
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
  X,
} from 'lucide-react';
import { Box, VStack, HStack, Inline } from './layout';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { Kbd } from '@/components/ui/kbd';
import { Separator } from '@/components/ui/separator';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { useTheme } from './ThemeProvider';

interface SettingsModalProps {
  config: SettingsConfig;
  onUpdateConfig: (key: string, value: unknown) => void;
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
  const { setTheme, setAccent, effectiveTheme } = useTheme();

  const handleThemeChange = (newTheme: 'light' | 'dark' | 'system') => {
    onUpdateConfig("theme", newTheme);
    setTheme(newTheme);
  };

  const handleAccentChange = (newAccent: 'coral' | 'cyan' | 'amber') => {
    onUpdateConfig("accent", newAccent);
    setAccent(newAccent);
  };

  const ACCENTS: { id: 'coral' | 'cyan' | 'amber'; label: string; color: string }[] = [
    { id: 'coral', label: '珊瑚', color: 'oklch(56% 0.13 35)' },
    { id: 'cyan', label: '青色', color: 'oklch(72% 0.170 200)' },
    { id: 'amber', label: '琥珀', color: 'oklch(55% 0.15 60)' },
  ];

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
              <Settings className="w-4 h-4 text-[color:var(--color-accent-fg)]" />
              <span className="text-lg font-semibold text-theme">Searchis 偏好设置</span>
            </HStack>
            <Button
              variant="ghost"
              size="icon"
              aria-label="关闭设置窗口"
              title="关闭设置窗口"
              onClick={onClose}
            >
              <X className="size-4" />
            </Button>
          </HStack>
        </Box>

        {/* Settings Body */}
        <Box padding="xl" className="max-h-[540px] overflow-y-auto">
          <VStack gap="lg">
            {/* Section 1: Hotkey & Startup */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2 text-sm">
                  <Keyboard className="size-4 text-[color:var(--color-accent-fg)]" />
                  全局快捷键与呼出
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <HStack align="center" justify="space-between">
                  <VStack gap="3xs">
                    <span className="text-xs font-semibold text-theme">全局呼出快捷键</span>
                    <span className="text-xs text-theme-muted">按下组合键无缝在桌面层级置顶快速检索浮层</span>
                  </VStack>
                  <Inline gap="3xs">
                    <Kbd>⌥</Kbd>
                    <Kbd>Space</Kbd>
                  </Inline>
                </HStack>

                <Separator />

                <div className="flex items-center justify-between gap-4">
                  <VStack gap="3xs">
                    <span className="text-xs font-semibold text-theme">开机自动启动</span>
                    <span className="text-xs text-theme-muted">系统登录后后台静默运行系统托盘与全局按键监听</span>
                  </VStack>
                  <Switch
                    checked={config.launchAtLogin}
                    onCheckedChange={checked => onUpdateConfig("launchAtLogin", checked)}
                  />
                </div>
              </CardContent>
            </Card>

            {/* Section 2: Paste & Audio */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2 text-sm">
                  <Zap className="size-4 text-[color:var(--color-accent-fg)]" />
                  自动粘贴与提示音效
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="flex items-center justify-between gap-4">
                  <VStack gap="3xs">
                    <span className="text-xs font-semibold text-theme">自动模拟粘贴</span>
                    <span className="text-xs text-theme-muted">在快速检索窗口按 Enter 选定片段后，自动复制并发送 Cmd+V 粘贴到上一个应用</span>
                  </VStack>
                  <Switch
                    checked={config.autoPaste}
                    onCheckedChange={checked => onUpdateConfig("autoPaste", checked)}
                  />
                </div>

                <Separator />

                <HStack align="center" justify="space-between">
                  <VStack gap="3xs">
                    <span className="text-xs font-semibold text-theme">操作音效反馈</span>
                    <span className="text-xs text-theme-muted">复制或成功粘贴时播放极简轻柔提示音</span>
                  </VStack>
                  <Button
                    variant={config.playAudioFeedback ? 'default' : 'outline'}
                    size="sm"
                    onClick={() => onUpdateConfig("playAudioFeedback", !config.playAudioFeedback)}
                  >
                    {config.playAudioFeedback ? <Volume2 className="size-3.5" /> : <VolumeX className="size-3.5" />}
                    {config.playAudioFeedback ? '已开启' : '已关闭'}
                  </Button>
                </HStack>
              </CardContent>
            </Card>

            {/* Section 3: Theme Appearance */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2 text-sm">
                  <Sun className="size-4 text-[color:var(--color-accent-fg)]" />
                  外观主题模式 (支持深浅两色)
                </CardTitle>
              </CardHeader>
              <CardContent>
                <HStack align="center" justify="space-between">
                  <VStack gap="3xs">
                    <span className="text-xs font-semibold text-theme">界面色彩风格</span>
                    <span className="text-xs text-theme-muted">完美适配浅色 (Bloom) 与深色 (Aurora)</span>
                  </VStack>

                  <Inline gap="2xs" className="p-1 rounded-lg bg-muted">
                    {([
                      ['light', Sun, '浅色'],
                      ['dark', Moon, '深色'],
                      ['system', Monitor, '跟随系统'],
                    ] as const).map(([t, Icon, label]) => (
                      <Button
                        key={t}
                        type="button"
                        size="sm"
                        variant={config.theme === t ? 'secondary' : 'ghost'}
                        className="h-7 px-2.5 text-xs gap-1 rounded-md"
                        onClick={() => handleThemeChange(t)}
                      >
                        <Icon className="size-3.5" />
                        <span>{label}</span>
                      </Button>
                    ))}
                  </Inline>
                </HStack>

                <Separator />

                <HStack align="center" justify="space-between">
                  <VStack gap="3xs">
                    <span className="text-xs font-semibold text-theme">强调色预设</span>
                    <span className="text-xs text-theme-muted">珊瑚 (浅色默认) / 青色 (深色默认) / 琥珀</span>
                  </VStack>
                  <Inline gap="xs" align="center">
                    {ACCENTS.map(a => (
                      <button
                        key={a.id}
                        type="button"
                        title={a.label}
                        aria-label={`强调色 ${a.label}`}
                        aria-pressed={(config.accent ?? undefined) === a.id || (!config.accent && ((effectiveTheme === 'dark' && a.id === 'cyan') || (effectiveTheme === 'light' && a.id === 'coral')))}
                        onClick={() => handleAccentChange(a.id)}
                        className={`h-6 w-6 rounded-full border-2 transition-[border-color,transform] ${
                          (config.accent ?? undefined) === a.id
                            ? 'border-[color:var(--color-fg)] scale-110'
                            : 'border-transparent hover:scale-110'
                        }`}
                        style={{ background: a.color }}
                      />
                    ))}
                  </Inline>
                </HStack>
              </CardContent>
            </Card>

            {/* Section 4: Local Data Backup */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2 text-sm">
                  <HardDrive className="size-4 text-[color:var(--color-accent-fg)]" />
                  数据备份与还原 (单机本地优先)
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <HStack align="center" justify="space-between">
                  <VStack gap="3xs">
                    <span className="text-xs font-semibold text-theme">导出全量 JSON 数据</span>
                    <span className="text-xs text-theme-muted">安全备份文本片段库到本地 UTF-8 明文 JSON 文件</span>
                  </VStack>
                  <Button variant="outline" size="sm" onClick={onExportData}>
                    <Download className="size-3.5" />
                    导出 JSON
                  </Button>
                </HStack>

                <Separator />

                <HStack align="center" justify="space-between">
                  <VStack gap="3xs">
                    <span className="text-xs font-semibold text-theme">导入 JSON 备份数据</span>
                    <span className="text-xs text-theme-muted">校验格式与 Schema 后合并恢复片段数据</span>
                  </VStack>
                  <Button asChild variant="outline" size="sm">
                    <label className="cursor-pointer">
                      <Upload className="size-3.5" />
                      <span>选择文件导入</span>
                      <input type="file" accept=".json" onChange={onImportData} className="hidden" />
                    </label>
                  </Button>
                </HStack>

                <Separator />

                <HStack align="center" justify="space-between">
                  <VStack gap="3xs">
                    <span className="text-xs font-semibold text-[color:var(--color-danger)]">重置初始内置数据</span>
                    <span className="text-xs text-theme-muted">恢复默认提供的常用 Key 文本演示模板</span>
                  </VStack>
                  <Button variant="destructive" size="sm" onClick={onResetSampleData}>
                    <RotateCcw className="size-3.5" />
                    重置示例数据
                  </Button>
                </HStack>
              </CardContent>
            </Card>
          </VStack>
        </Box>
      </Box>
    </Box>
  );
};
