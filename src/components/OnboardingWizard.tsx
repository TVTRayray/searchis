import React, { useState } from 'react';
import {
  Zap,
  Keyboard,
  ShieldCheck,
  Plus,
  CheckCircle2,
  ArrowRight,
  ArrowLeft,
  CornerDownLeft,
  Check,
  Terminal,
} from 'lucide-react';
import { Box, VStack, HStack, Inline } from './layout';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Kbd } from '@/components/ui/kbd';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Label } from '@/components/ui/label';
import { Card, CardContent } from '@/components/ui/card';

interface OnboardingWizardProps {
  onComplete: () => void;
  onCreateSnippet: (key: string, title: string, content: string) => void;
}

const statusToken = (color: string, label: string) => (
  <Badge variant="outline" className="gap-1.5 rounded-full text-[11px] font-medium">
    <span className="size-1.5 rounded-full" style={{ background: color }} aria-hidden />
    {label}
  </Badge>
);

export const OnboardingWizard: React.FC<OnboardingWizardProps> = ({
  onComplete,
  onCreateSnippet,
}) => {
  const [step, setStep] = useState(1);

  // Step 2 shortcut config
  const [shortcutKey, setShortcutKey] = useState('Option + Space');
  const [isRecordingShortcut, setIsRecordingShortcut] = useState(false);

  // Step 4 snippet builder
  const [demoKey, setDemoKey] = useState('hello-searchis');
  const [demoTitle, setDemoTitle] = useState('问候名片');
  const [demoContent, setDemoContent] = useState('你好！很高兴认识你。这是我通过 Searchis 快速粘贴的第一条文本片段！🚀');

  // Step 5 interactive test
  const [testInput, setTestInput] = useState('');
  const [testSuccess, setTestSuccess] = useState(false);

  const nextStep = () => {
    if (step === 4) {
      onCreateSnippet(demoKey, demoTitle, demoContent);
    }
    setStep(prev => prev + 1);
  };

  const prevStep = () => {
    setStep(prev => Math.max(1, prev - 1));
  };

  const handleTestKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && testInput.trim().toLowerCase() === demoKey.toLowerCase()) {
      e.preventDefault();
      setTestSuccess(true);
    }
  };

  return (
    <Box padding="md" className="max-w-3xl mx-auto select-none">
      <Box
        radius="xl"
        shadow="window"
        background="surface"
        border="all"
        overflow="hidden"
        className="raycast-window min-h-[520px] flex flex-col justify-between"
      >
        {/* Title bar */}
        <Box paddingX="lg" paddingY="sm" background="titlebar" border="bottom" className="text-center">
          <span className="text-xs font-bold text-theme-secondary">
            Searchis 首次使用向导 (步骤 {step} / 6)
          </span>
        </Box>

        {/* Step Indicator Bar */}
        <Box paddingX="xl" paddingY="md">
          <HStack align="center" justify="space-between" className="relative">
            <Box
              className="absolute left-0 top-1/2 transform -translate-y-1/2 w-full -z-10"
              style={{ height: '2px', backgroundColor: 'var(--color-border-strong)' }}
            />
            {[1, 2, 3, 4, 5, 6].map(i => (
              <Box
                key={i}
                width="32px"
                height="32px"
                radius="full"
                className={`flex items-center justify-center text-xs font-bold transition-[color,background-color,box-shadow] ${
                  i === step
                    ? 'btn-primary'
                    : i < step
                    ? 'btn-success'
                    : 'theme-surface-subtle text-theme-muted border theme-divider'
                }`}
              >
                {i < step ? <Check className="w-4 h-4" /> : i}
              </Box>
            ))}
          </HStack>
        </Box>

        {/* Step Content Body */}
        <Box padding="xl" className="flex-1 flex flex-col justify-center">
          {step === 1 && (
            <VStack align="center" justify="center" gap="md" className="text-center max-w-md mx-auto">
              <Box width="64px" height="64px" radius="xl" className="brand-mark flex items-center justify-center">
                <Zap className="w-8 h-8" />
              </Box>
              <VStack gap="2xs">
                <h2 className="text-xl font-bold text-theme">欢迎体验 Searchis</h2>
                <p className="text-xs text-theme-secondary leading-relaxed">
                  一个本地优先、键盘优先的文本片段检索与快速粘贴工具。
                  <br />
                  无需鼠标翻找，只需按全局快捷键呼出，输入 2～5 个 Key 字符并回车，即刻写入剪贴板并粘贴！
                </p>
              </VStack>

              <Inline gap="md" align="center" className="pt-2">
                {statusToken('var(--color-accent)', '键盘驱动')}
                {statusToken('var(--color-accent)', '毫秒搜索')}
                {statusToken('var(--color-success)', 'SQLCipher 本地存储')}
              </Inline>
            </VStack>
          )}

          {step === 2 && (
            <VStack align="center" justify="center" gap="md" className="text-center max-w-md mx-auto">
              <Box width="48px" height="48px" radius="full" className="badge-accent flex items-center justify-center">
                <Keyboard className="w-6 h-6" />
              </Box>
              <VStack gap="2xs">
                <h3 className="text-base font-bold text-theme">配置全局呼出快捷键</h3>
                <p className="text-xs text-theme-muted">无论当前在前台使用什么应用，按下该组合键即可立刻置顶呼出搜索窗</p>
              </VStack>

              <Card className="w-full">
                <CardContent className="py-6">
                  <VStack gap="sm" align="center">
                    <span className="text-xs font-semibold text-theme-secondary">当前唤醒快捷键:</span>
                    <Kbd className="px-4 py-2 text-sm">{shortcutKey}</Kbd>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setIsRecordingShortcut(prev => !prev)}
                    >
                      {isRecordingShortcut ? '请按下您想设置的按键...' : '更改快捷键'}
                    </Button>
                  </VStack>
                </CardContent>
              </Card>
            </VStack>
          )}

          {step === 3 && (
            <VStack gap="md" className="max-w-md mx-auto">
              <VStack align="center" gap="2xs" className="text-center">
                <ShieldCheck className="w-8 h-8 text-[color:var(--color-success)]" />
                <h3 className="text-base font-bold text-theme">Linux / X11 系统权限状态</h3>
                <p className="text-xs text-theme-muted">自动发送 Cmd+V 粘贴按键与获取窗口句柄需要以下权限：</p>
              </VStack>

              <Box className="w-full border rounded-lg overflow-hidden">
                <div className="flex items-center justify-between gap-4 px-4 py-3">
                  <VStack gap="2xs">
                    <span className="text-xs font-semibold text-theme">X11 / KGlobalAccel 按键注入</span>
                    <span className="text-xs text-theme-muted">用于向目标应用窗口自动模拟粘贴</span>
                  </VStack>
                  <Badge className="gap-1 rounded-full text-[11px] bg-[color:var(--color-success)] text-[color:var(--color-fg-inverse)]">
                    已授权
                    <Check className="size-3" />
                  </Badge>
                </div>
                <div className="flex items-center justify-between gap-4 px-4 py-3 border-t">
                  <VStack gap="2xs">
                    <span className="text-xs font-semibold text-theme">剪贴板暂存与恢复</span>
                    <span className="text-xs text-theme-muted">用于安全暂存选中的文本片段</span>
                  </VStack>
                  <Badge className="gap-1 rounded-full text-[11px] bg-[color:var(--color-success)] text-[color:var(--color-fg-inverse)]">
                    已授权
                    <Check className="size-3" />
                  </Badge>
                </div>
              </Box>
            </VStack>
          )}

          {step === 4 && (
            <VStack gap="md" className="max-w-md mx-auto">
              <VStack align="center" gap="2xs" className="text-center">
                <Plus className="w-7 h-7 text-[color:var(--color-accent-fg)]" />
                <h3 className="text-base font-bold text-theme">创建你的第一条片段</h3>
                <p className="text-xs text-theme-muted">设置简短好记的 Key 标识，并填入要粘贴的内容</p>
              </VStack>

              <Card>
                <CardContent className="space-y-4">
                  <VStack gap="xs">
                    <Label htmlFor="demo-key">Key (检索标识)</Label>
                    <Input id="demo-key" value={demoKey} onChange={e => setDemoKey(e.target.value)} className="font-mono" />
                  </VStack>
                  <VStack gap="xs">
                    <Label htmlFor="demo-title">标题</Label>
                    <Input id="demo-title" value={demoTitle} onChange={e => setDemoTitle(e.target.value)} />
                  </VStack>
                  <VStack gap="xs">
                    <Label htmlFor="demo-content">文本内容</Label>
                    <Textarea id="demo-content" value={demoContent} onChange={e => setDemoContent(e.target.value)} rows={3} />
                  </VStack>
                </CardContent>
              </Card>
            </VStack>
          )}

          {step === 5 && (
            <VStack align="center" gap="md" className="max-w-md mx-auto text-center">
              <Box width="44px" height="44px" radius="full" className="badge-accent flex items-center justify-center">
                <Terminal className="w-5 h-5" />
              </Box>
              <VStack gap="2xs">
                <h3 className="text-base font-bold text-theme">演练测试快速粘贴</h3>
                <p className="text-xs text-theme-muted">
                  在下方输入框中输入 Key <Kbd>{demoKey}</Kbd> 并按 <Kbd>↵ 回车</Kbd>：
                </p>
              </VStack>

              <Card className="w-full">
                <CardContent className="space-y-4">
                  <Input
                    type="text"
                    value={testInput}
                    onChange={e => setTestInput(e.target.value)}
                    onKeyDown={handleTestKeyDown}
                    placeholder={`输入 ${demoKey} 后按回车...`}
                    className="font-mono text-center"
                  />

                  {testSuccess && (
                    <Box padding="sm" radius="md" background="subtle" border="all" className="status-success">
                      <VStack gap="xs" align="center">
                        <HStack align="center" gap="xs">
                          <CheckCircle2 className="w-4 h-4" />
                          <span className="font-bold text-xs">自动粘贴测试成功！</span>
                        </HStack>
                        <span className="text-xs font-mono">{demoContent}</span>
                      </VStack>
                    </Box>
                  )}
                </CardContent>
              </Card>
            </VStack>
          )}

          {step === 6 && (
            <VStack align="center" justify="center" gap="md" className="text-center max-w-md mx-auto">
              <Box width="64px" height="64px" radius="full" className="btn-success flex items-center justify-center">
                <CheckCircle2 className="w-10 h-10" />
              </Box>
              <VStack gap="2xs">
                <h2 className="text-xl font-bold text-theme">恭喜！向导全部完成！</h2>
                <p className="text-xs text-theme-secondary">
                  你已经完整掌握 Searchis 的键盘交互流程。即刻开启高效粘贴体验！
                </p>
              </VStack>
            </VStack>
          )}
        </Box>

        {/* Footer controls */}
        <Box paddingX="lg" paddingY="md" background="titlebar" border="top">
          <HStack align="center" justify="space-between">
            <Button
              variant="outline"
              size="sm"
              disabled={step === 1}
              onClick={prevStep}
            >
              <ArrowLeft className="size-4" />
              上一步
            </Button>

            {step < 6 ? (
              <Button
                variant="default"
                size="sm"
                onClick={nextStep}
              >
                下一步
                <ArrowRight className="size-4" />
              </Button>
            ) : (
              <Button
                variant="default"
                onClick={onComplete}
              >
                <CornerDownLeft className="size-4" />
                进入 Searchis 主程序
              </Button>
            )}
          </HStack>
        </Box>
      </Box>
    </Box>
  );
};
