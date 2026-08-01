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
  const sectionClass = 'p-4 rounded-xl theme-surface-subtle border theme-divider-subtle space-y-3 text-sm';
  const sectionTitleClass = 'text-xs font-bold text-theme-muted uppercase tracking-wider flex items-center gap-1.5';
  const rowTitleClass = 'font-semibold text-theme';
  const rowDescriptionClass = 'text-xs text-theme-muted';
  const dividerRowClass = 'pt-3 border-t theme-divider flex items-center justify-between gap-3';

  return (
    <div className="max-w-2xl mx-auto p-4 sm:p-6">
      <div className="rounded-2xl raycast-window overflow-hidden animate-pop-in">
        <div className="h-11 px-4 theme-titlebar border-b theme-divider flex items-center justify-between">
          <span className="text-sm font-bold text-theme flex items-center gap-1.5">
            <Settings className="w-4 h-4 icon-accent" />
            <span>Searchis 偏好设置</span>
          </span>
          <button onClick={onClose} className="p-1 rounded-md interactive-muted" aria-label="关闭设置">
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="p-6 space-y-6 max-h-[540px] overflow-y-auto">
          <section className="space-y-3">
            <h4 className={sectionTitleClass}>
              <Keyboard className="w-4 h-4 icon-accent" />
              <span>全局快捷键与唤醒</span>
            </h4>
            <div className={sectionClass}>
              <div className="flex items-center justify-between gap-3">
                <div>
                  <div className={rowTitleClass}>全局呼出快捷键</div>
                  <div className={rowDescriptionClass}>按下组合键显示或隐藏快速检索浮层</div>
                </div>
                <div className="flex items-center gap-1"><kbd className="raycast-kbd">⌥</kbd><kbd className="raycast-kbd">Space</kbd></div>
              </div>
              <div className={dividerRowClass}>
                <div>
                  <div className={rowTitleClass}>开机自动启动</div>
                  <div className={rowDescriptionClass}>系统登录后自动运行菜单栏服务</div>
                </div>
                <input type="checkbox" checked={config.launchAtLogin} onChange={e => onUpdateConfig(prev => ({ ...prev, launchAtLogin: e.target.checked }))} />
              </div>
            </div>
          </section>

          <section className="space-y-3">
            <h4 className={sectionTitleClass}>
              <Zap className="w-4 h-4 icon-accent" />
              <span>粘贴行为与音效</span>
            </h4>
            <div className={sectionClass}>
              <div className="flex items-center justify-between gap-3">
                <div>
                  <div className={rowTitleClass}>自动粘贴</div>
                  <div className={rowDescriptionClass}>选择片段后写入剪贴板并自动模拟 Cmd+V 粘贴</div>
                </div>
                <input type="checkbox" checked={config.autoPaste} onChange={e => onUpdateConfig(prev => ({ ...prev, autoPaste: e.target.checked }))} />
              </div>
              <div className={dividerRowClass}>
                <div>
                  <div className={rowTitleClass}>操作音效</div>
                  <div className={rowDescriptionClass}>粘贴或复制时播放轻柔按键音</div>
                </div>
                <button
                  onClick={() => onUpdateConfig(prev => ({ ...prev, playAudioFeedback: !prev.playAudioFeedback }))}
                  className={`px-2.5 py-1.5 rounded-lg border text-xs flex items-center gap-1.5 transition-colors ${config.playAudioFeedback ? 'badge-accent' : 'btn-secondary'}`}
                >
                  {config.playAudioFeedback ? <Volume2 className="w-4 h-4" /> : <VolumeX className="w-4 h-4" />}
                  <span>{config.playAudioFeedback ? '开启' : '关闭'}</span>
                </button>
              </div>
            </div>
          </section>

          <section className="space-y-3">
            <h4 className={sectionTitleClass}>
              <Sun className="w-4 h-4 icon-accent" />
              <span>外观风格</span>
            </h4>
            <div className={sectionClass}>
              <div className="flex items-center justify-between gap-4">
                <div>
                  <div className={rowTitleClass}>主题模式</div>
                  <div className={rowDescriptionClass}>界面色彩与系统外观保持协调</div>
                </div>
                <div className="flex items-center p-1 rounded-lg control">
                  {([
                    ['light', Sun, '浅色'],
                    ['dark', Moon, '深色'],
                    ['system', Monitor, '跟随系统'],
                  ] as const).map(([theme, Icon, label]) => (
                    <button
                      key={theme}
                      onClick={() => onUpdateConfig(prev => ({ ...prev, theme }))}
                      className={`flex items-center gap-1 px-2.5 py-1 rounded-md text-xs font-medium transition-all ${config.theme === theme ? 'nav-active shadow-xs' : 'interactive-muted'}`}
                    >
                      <Icon className="w-3.5 h-3.5" />
                      <span>{label}</span>
                    </button>
                  ))}
                </div>
              </div>
            </div>
          </section>

          <section className="space-y-3">
            <h4 className={sectionTitleClass}>
              <HardDrive className="w-4 h-4 icon-accent" />
              <span>数据备份与管理（本地优先）</span>
            </h4>
            <div className={sectionClass}>
              <div className="flex flex-wrap items-center justify-between gap-3">
                <div><div className={rowTitleClass}>导出片段 JSON 数据</div><div className={rowDescriptionClass}>备份全部 Key、标题与文本片段到本地</div></div>
                <button onClick={onExportData} className="btn-secondary flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors"><Download className="w-3.5 h-3.5" /><span>导出 JSON</span></button>
              </div>
              <div className={`${dividerRowClass} flex-wrap`}>
                <div><div className={rowTitleClass}>导入 JSON 备份数据</div><div className={rowDescriptionClass}>从已导出的备份文件恢复文本片段</div></div>
                <label className="btn-secondary flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors cursor-pointer"><Upload className="w-3.5 h-3.5" /><span>选择文件导入</span><input type="file" accept=".json" onChange={onImportData} className="hidden" /></label>
              </div>
              <div className={`${dividerRowClass} flex-wrap`}>
                <div><div className="font-semibold text-danger">重置为内置示例数据</div><div className={rowDescriptionClass}>清空当前数据并恢复初始化演示 Key 片段</div></div>
                <button onClick={onResetSampleData} className="badge-danger flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors"><RotateCcw className="w-3.5 h-3.5" /><span>恢复初始数据</span></button>
              </div>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
};
