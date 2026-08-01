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
  Terminal
} from 'lucide-react';

interface OnboardingWizardProps {
  onComplete: () => void;
  onCreateSnippet: (key: string, title: string, content: string) => void;
}

export const OnboardingWizard: React.FC<OnboardingWizardProps> = ({
  onComplete,
  onCreateSnippet
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
      // Save snippet
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
    <div className="max-w-3xl mx-auto p-4 sm:p-6">
      
      {/* Container */}
      <div className="rounded-2xl raycast-window overflow-hidden min-h-[520px] flex flex-col justify-between">
        
        {/* Title bar */}
        <div className="h-10 px-4 theme-titlebar border-b theme-divider flex items-center justify-center">
          <span className="text-sm font-bold text-theme-secondary">
            Searchis 首次使用向导 (步骤 {step} / 6)
          </span>
        </div>

        {/* Step Indicator Bar */}
        <div className="px-6 pt-5">
          <div className="flex items-center justify-between relative">
            <div className="absolute left-0 top-1/2 transform -translate-y-1/2 w-full h-0.5 bg-[color:var(--border-strong)] -z-10" />
            {[1, 2, 3, 4, 5, 6].map(i => (
              <div
                key={i}
                className={`w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold transition-all ${
                  i === step
                    ? 'btn-primary'
                    : i < step
                    ? 'btn-success'
                    : 'theme-surface-subtle text-theme-muted border theme-divider'
                }`}
              >
                {i < step ? <Check className="w-4 h-4" /> : i}
              </div>
            ))}
          </div>
        </div>

        {/* Step Body */}
        <div className="p-6 flex-1 flex flex-col justify-center">
          
          {/* STEP 1: Welcome */}
          {step === 1 && (
            <div className="text-center space-y-4 max-w-md mx-auto">
              <div className="brand-mark w-16 h-16 rounded-xl flex items-center justify-center mx-auto">
                <Zap className="w-8 h-8" />
              </div>
              <h2 className="text-xl font-bold text-theme">
                欢迎体验 Searchis
              </h2>
              <p className="text-sm text-theme-secondary leading-relaxed">
                一个本地优先、键盘优先的文本片段检索与快速粘贴工具。
                <br />
                无需鼠标翻找，只需通过全局快捷键呼出，输入 2～5 个 Key 字符并回车，即可瞬间粘贴到当前任意应用！
              </p>
              <div className="pt-2 flex justify-center gap-4 text-xs text-theme-muted">
                <div className="flex items-center gap-1">
                  <Keyboard className="w-4 h-4 icon-accent" />
                  <span>键盘驱动</span>
                </div>
                <div className="flex items-center gap-1">
                  <Zap className="w-4 h-4 icon-accent" />
                  <span>毫秒搜索</span>
                </div>
                <div className="flex items-center gap-1">
                  <ShieldCheck className="w-4 h-4 icon-success" />
                  <span>本地存储</span>
                </div>
              </div>
            </div>
          )}

          {/* STEP 2: Shortcut Setup */}
          {step === 2 && (
            <div className="space-y-4 max-w-md mx-auto text-center">
              <div className="w-12 h-12 rounded-full badge-accent flex items-center justify-center mx-auto">
                <Keyboard className="w-6 h-6" />
              </div>
              <h3 className="text-base font-bold text-theme">
                配置全局呼出快捷键
              </h3>
              <p className="text-sm text-theme-muted">
                无论你在使用 Chrome、VS Code 还是微信，按下此快捷键即可置顶弹窗。
              </p>

              <div className="theme-surface-subtle p-4 rounded-xl border theme-divider space-y-3">
                <div className="text-sm font-semibold text-theme-secondary">
                  当前唤醒快捷键:
                </div>
                <div className="flex items-center justify-center gap-2">
                  <kbd className="raycast-kbd px-3 py-1.5 text-sm h-auto rounded-md">
                    {shortcutKey}
                  </kbd>
                </div>
                <button
                  onClick={() => setIsRecordingShortcut(prev => !prev)}
                  className="btn-secondary px-3 py-1 rounded-md text-sm font-medium transition-colors"
                >
                  {isRecordingShortcut ? '请按下您想设置的组合按键...' : '更改快捷键'}
                </button>
              </div>
            </div>
          )}

          {/* STEP 3: Permissions Check */}
          {step === 3 && (
            <div className="space-y-4 max-w-md mx-auto">
              <div className="text-center space-y-1">
                <ShieldCheck className="w-8 h-8 icon-success mx-auto" />
                <h3 className="text-base font-bold text-theme">
                  macOS 系统权限检查
                </h3>
                <p className="text-sm text-theme-muted">
                  为了实现自动将文本粘贴至上一个前台应用，Searchis 需要以下权限：
                </p>
              </div>

              <div className="space-y-2">
                <div className="theme-surface-subtle p-3 rounded-xl border theme-divider flex items-center justify-between text-sm">
                  <div>
                    <div className="font-semibold text-theme">
                      辅助功能权限
                    </div>
                    <div className="text-xs text-theme-muted">
                      用于模拟 Command+V 粘贴按键
                    </div>
                  </div>
                  <span className="badge-success px-2 py-0.5 rounded-full text-xs font-medium">
                    已授权 ✓
                  </span>
                </div>

                <div className="theme-surface-subtle p-3 rounded-xl border theme-divider flex items-center justify-between text-sm">
                  <div>
                    <div className="font-semibold text-theme">
                      剪贴板读取与写入
                    </div>
                    <div className="text-xs text-theme-muted">
                      用于暂存文本片段
                    </div>
                  </div>
                  <span className="badge-success px-2 py-0.5 rounded-full text-xs font-medium">
                    已授权 ✓
                  </span>
                </div>
              </div>
            </div>
          )}

          {/* STEP 4: Build First Snippet */}
          {step === 4 && (
            <div className="space-y-3 max-w-md mx-auto">
              <div className="text-center space-y-1">
                <Plus className="w-7 h-7 icon-accent mx-auto" />
                <h3 className="text-base font-bold text-theme">
                  创建你的第一条片段
                </h3>
                <p className="text-sm text-theme-muted">
                  设置一个简短、好记的 Key，并在下面填入常用文本。
                </p>
              </div>

              <div className="theme-surface-subtle space-y-3 p-4 rounded-xl border theme-divider text-sm">
                <div>
                  <label className="block text-xs font-bold text-theme-secondary mb-1">
                    Key (检索标识):
                  </label>
                  <input
                    type="text"
                    value={demoKey}
                    onChange={e => setDemoKey(e.target.value)}
                    className="input-theme w-full px-3 py-1.5 rounded-md text-accent font-mono font-bold"
                  />
                </div>
                <div>
                  <label className="block text-xs font-bold text-theme-secondary mb-1">
                    标题:
                  </label>
                  <input
                    type="text"
                    value={demoTitle}
                    onChange={e => setDemoTitle(e.target.value)}
                    className="input-theme w-full px-3 py-1.5 rounded-md"
                  />
                </div>
                <div>
                  <label className="block text-xs font-bold text-theme-secondary mb-1">
                    文本内容:
                  </label>
                  <textarea
                    rows={3}
                    value={demoContent}
                    onChange={e => setDemoContent(e.target.value)}
                    className="input-theme w-full px-3 py-1.5 rounded-md font-mono"
                  />
                </div>
              </div>
            </div>
          )}

          {/* STEP 5: Interactive Paste Test */}
          {step === 5 && (
            <div className="space-y-4 max-w-md mx-auto text-center">
              <div className="w-10 h-10 rounded-full badge-accent flex items-center justify-center mx-auto">
                <Terminal className="w-5 h-5" />
              </div>
              <h3 className="text-base font-bold text-theme">
                测试快速粘贴
              </h3>
              <p className="text-sm text-theme-muted">
                在下方输入框中输入刚刚设置的 Key <code className="px-1 font-mono font-bold text-accent">{demoKey}</code> 并按 <kbd className="font-mono">↵ 回车</kbd>：
              </p>

              <div className="theme-surface-subtle p-4 rounded-xl border theme-divider space-y-3">
                <input
                  type="text"
                  value={testInput}
                  onChange={e => setTestInput(e.target.value)}
                  onKeyDown={handleTestKeyDown}
                  placeholder={`在此处输入 ${demoKey} 并按回车...`}
                  className="input-theme w-full px-3 py-2 rounded-md text-sm font-mono text-center"
                />

                {testSuccess && (
                  <div className="status-success p-3 rounded-md border text-sm space-y-1">
                    <div className="font-bold flex items-center justify-center gap-1">
                      <CheckCircle2 className="w-4 h-4" /> 粘贴成功！
                    </div>
                    <div className="theme-surface text-theme-secondary text-xs font-mono p-2 rounded-md border theme-divider">
                      {demoContent}
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* STEP 6: Complete */}
          {step === 6 && (
            <div className="text-center space-y-4 max-w-md mx-auto">
              <div className="w-16 h-16 rounded-full btn-success flex items-center justify-center mx-auto">
                <CheckCircle2 className="w-10 h-10" />
              </div>
              <h2 className="text-xl font-bold text-theme">
                恭喜！引导全部完成！
              </h2>
              <p className="text-sm text-theme-secondary">
                您现在已经掌握了 Searchis 的核心使用流程。立刻开启效率翻倍的粘贴之旅吧！
              </p>
            </div>
          )}

        </div>

        {/* Footer Navigation Controls */}
        <div className="px-6 py-4 theme-titlebar border-t theme-divider flex items-center justify-between">
          <button
            onClick={prevStep}
            disabled={step === 1}
            className="btn-secondary flex items-center gap-1 px-3 py-1.5 rounded-md disabled:opacity-30 text-sm font-medium transition-colors"
          >
            <ArrowLeft className="w-4 h-4" />
            <span>上一步</span>
          </button>

          {step < 6 ? (
            <button
              onClick={nextStep}
              className="btn-primary flex items-center gap-1 px-4 py-1.5 rounded-md text-sm font-semibold transition-all"
            >
              <span>下一步</span>
              <ArrowRight className="w-4 h-4" />
            </button>
          ) : (
            <button
              onClick={onComplete}
              className="btn-success flex items-center gap-1.5 px-5 py-2 rounded-md text-sm font-bold transition-all"
            >
              <span>进入 Searchis 应用</span>
              <CornerDownLeft className="w-4 h-4" />
            </button>
          )}
        </div>

      </div>

    </div>
  );
};
