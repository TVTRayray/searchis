import React, { useEffect, useState } from 'react';
import {
  ArrowLeft,
  Download,
  HardDrive,
  Keyboard,
  Monitor,
  Moon,
  RotateCcw,
  Settings,
  Sun,
  Upload,
  Zap,
} from 'lucide-react';
import { SettingsConfig, settingsApi } from '../api/snippets';
import { useTheme } from './ThemeProvider';
import { SelectField } from './ui/SelectField';

interface SettingsModalProps {
  config: SettingsConfig;
  onUpdateConfig: (key: string, value: unknown) => void;
  onExportData: () => void;
  onImportData: (e: React.ChangeEvent<HTMLInputElement>) => void;
  onResetSampleData: () => void;
  onClose: () => void;
}

const themes = [
  { id: 'light', label: '浅色', hint: 'Bloom', icon: Sun },
  { id: 'dark', label: '深色', hint: 'Aurora', icon: Moon },
  { id: 'system', label: '跟随系统', hint: '自动', icon: Monitor },
] as const;

export const SettingsModal: React.FC<SettingsModalProps> = ({
  config,
  onUpdateConfig,
  onExportData,
  onImportData,
  onResetSampleData,
  onClose,
}) => {
  const { setTheme, effectiveTheme } = useTheme();
  const [autostart, setAutostart] = useState(false);

  useEffect(() => {
    settingsApi.get().then(res => setAutostart(res.autostartActual)).catch(() => {});
  }, []);

  // 支持按下 Escape 键快速返回片段管理
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

  const handleThemeChange = (theme: 'light' | 'dark' | 'system') => {
    setTheme(theme);
    onUpdateConfig('theme', theme);
  };

  const updateAutostart = (enabled: boolean) => {
    onUpdateConfig('launchAtLogin', enabled);
    settingsApi.autostartSet(enabled).then(setAutostart).catch(() => {});
  };

  return (
    <div className="settings-page" role="region" aria-labelledby="settings-title">
      <div className="settings-nav-header">
        <button
          type="button"
          className="manager-button manager-button-secondary settings-back-button"
          onClick={onClose}
          aria-label="返回片段管理"
        >
          <ArrowLeft className="size-4" aria-hidden />
          <span>返回片段管理 (Esc)</span>
        </button>
        <h1 id="settings-title" className="settings-nav-title">偏好设置</h1>
      </div>

      <div className="settings-page-body">
        <section className="settings-section">
          <div className="settings-section-heading">
            <div className="settings-section-icon"><Keyboard className="size-4" aria-hidden /></div>
            <div><h2>快捷键与启动</h2><p>用一个组合键从桌面任意位置呼出快速检索。</p></div>
          </div>
          <div className="settings-row">
            <div><strong>全局呼出快捷键</strong><span>注册失败时会保留上一次可用快捷键。</span></div>
            <input
              type="text"
              value={config.globalShortcut}
              onChange={event => onUpdateConfig('globalShortcut', event.target.value)}
              className="settings-shortcut-input manager-control manager-mono"
              placeholder="例如 Alt+O"
              aria-label="全局呼出快捷键"
            />
          </div>
          <div className="settings-rule" />
          <div className="settings-row">
            <div><strong>开机自动启动</strong><span>使用 XDG Autostart，当前状态以系统文件为准。</span></div>
            <label className="settings-switch-label">
              <span className="sr-only">开机自动启动</span>
              <input type="checkbox" className="manager-switch" checked={config.launchAtLogin} onChange={event => updateAutostart(event.target.checked)} />
            </label>
          </div>
          <p className="settings-state-note">实际状态：{autostart ? '已启用' : '未启用'}</p>
        </section>

        <section className="settings-section">
          <div className="settings-section-heading">
            <div className="settings-section-icon"><Zap className="size-4" aria-hidden /></div>
            <div><h2>粘贴行为</h2><p>决定选中片段后是否尝试向原窗口发送 Ctrl+V。</p></div>
          </div>
          <div className="settings-row">
            <div><strong>自动粘贴</strong><span>关闭后仍可复制到剪贴板，但不会注入键盘事件。</span></div>
            <input type="checkbox" className="manager-switch" checked={config.autoPaste} onChange={event => onUpdateConfig('autoPaste', event.target.checked)} aria-label="自动粘贴" />
          </div>
          <div className="settings-rule" />
          <div className="settings-constraint">
            <strong>剪贴板恢复已关闭</strong>
            <span>V1 不保存或恢复用户原有剪贴板内容。</span>
          </div>
        </section>

        <section className="settings-section">
          <div className="settings-section-heading">
            <div className="settings-section-icon"><Sun className="size-4" aria-hidden /></div>
            <div><h2>外观主题</h2><p>管理窗口与独立快速检索窗口共享后端主题设置。</p></div>
          </div>
          <div className="settings-theme-grid" role="group" aria-label="外观主题">
            {themes.map(({ id, label, hint, icon: Icon }) => (
              <button
                key={id}
                type="button"
                className={`settings-theme-option ${config.theme === id ? 'is-selected' : ''}`}
                onClick={() => handleThemeChange(id)}
                aria-pressed={config.theme === id}
              >
                <Icon className="size-4" aria-hidden />
                <span><strong>{label}</strong><small>{hint}</small></span>
                {config.theme === id && <span className="settings-theme-check" aria-hidden>✓</span>}
              </button>
            ))}
          </div>
          <p className="settings-state-note">当前渲染：{effectiveTheme === 'dark' ? '深色 Aurora' : '浅色 Bloom'}</p>
        </section>

        <section className="settings-section">
          <div className="settings-section-heading">
            <div className="settings-section-icon"><Settings className="size-4" aria-hidden /></div>
            <div><h2>检索与数据</h2><p>这些值会写入本地设置，并在重启后保持。</p></div>
          </div>
          <div className="settings-row">
            <div><strong>最大结果数</strong><span>快速检索一次最多展示多少条结果。</span></div>
            <SelectField
              value={String(config.maxResultsCount)}
              options={[5, 10, 20, 50, 100].map(value => ({ value: String(value), label: `${value} 条` }))}
              onChange={value => onUpdateConfig('maxResultsCount', Number(value))}
              ariaLabel="最大结果数"
              className="settings-select"
            />
          </div>
          <div className="settings-rule" />
          <div className="settings-row">
            <div><strong>回收站自动清理</strong><span>关闭，或在片段进入回收站后经过指定天数清理。</span></div>
            <SelectField
              value={config.trashAutoPurgeDays == null ? '' : String(config.trashAutoPurgeDays)}
              options={[
                { value: '', label: '关闭' },
                { value: '7', label: '7 天' },
                { value: '30', label: '30 天' },
                { value: '90', label: '90 天' },
              ]}
              onChange={value => onUpdateConfig('trashAutoPurgeDays', value ? Number(value) : null)}
              ariaLabel="回收站自动清理"
              className="settings-select"
            />
          </div>
        </section>

        <section className="settings-section">
          <div className="settings-section-heading">
            <div className="settings-section-icon"><HardDrive className="size-4" aria-hidden /></div>
            <div><h2>本地备份</h2><p>备份文件为 UTF-8 明文 JSON；含敏感片段时请确认存放位置。</p></div>
          </div>
          <div className="settings-action-row">
            <div><strong>导出 JSON 备份</strong><span>导出当前片段库与允许备份的设置。</span></div>
            <button type="button" className="manager-button manager-button-secondary" onClick={onExportData}><Download className="size-3.5" aria-hidden />导出</button>
          </div>
          <div className="settings-rule" />
          <div className="settings-action-row">
            <div><strong>导入 JSON 备份</strong><span>选择文件后由应用服务校验格式，再决定是否写入。</span></div>
            <label className="manager-button manager-button-secondary settings-file-button">
              <Upload className="size-3.5" aria-hidden />选择文件
              <input type="file" accept=".json" onChange={onImportData} className="sr-only" />
            </label>
          </div>
          <div className="settings-rule" />
          <div className="settings-action-row settings-danger-row">
            <div><strong>重置内置示例数据</strong><span>这是破坏性操作，确认前不会改变数据库。</span></div>
            <button type="button" className="manager-button manager-button-danger" onClick={onResetSampleData}><RotateCcw className="size-3.5" aria-hidden />重置</button>
          </div>
        </section>
      </div>
    </div>
  );
};
