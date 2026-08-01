import React from 'react';
import { SettingsConfig } from '../types/snippet';
import {
  Settings,
  Keyboard,
  Sun,
  Moon,
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
  return (
    <div className="max-w-2xl mx-auto p-4 sm:p-6">
      <div className="rounded-lg bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 shadow-lg overflow-hidden">
        
        {/* Title Bar */}
        <div className="h-11 px-4 bg-zinc-50 dark:bg-zinc-950 border-b border-zinc-200 dark:border-zinc-800 flex items-center justify-between">
          <span className="text-sm font-bold text-zinc-800 dark:text-zinc-200 flex items-center gap-1.5">
            <Settings className="w-4 h-4" />
            <span>Searchis 偏好设置</span>
          </span>
          <button
            onClick={onClose}
            className="text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Settings Body */}
        <div className="p-6 space-y-6 max-h-[540px] overflow-y-auto">
          
          {/* Section 1: Shortcuts & Launch */}
          <div className="space-y-3">
            <h4 className="text-xs font-bold text-zinc-500 uppercase tracking-wider flex items-center gap-1.5">
              <Keyboard className="w-4 h-4" />
              <span>全局快捷键与唤醒</span>
            </h4>

            <div className="p-4 rounded-md bg-zinc-50 dark:bg-zinc-950 border border-zinc-200 dark:border-zinc-800 space-y-3 text-sm">
              <div className="flex items-center justify-between">
                <div>
                  <div className="font-semibold text-zinc-800 dark:text-zinc-200">
                    全局呼出快捷键
                  </div>
                  <div className="text-xs text-zinc-500">
                    按下组合键显示或隐藏快速检索浮层
                  </div>
                </div>
                <div className="flex items-center gap-1">
                  <span className="font-mono font-bold">⌥</span>
                  <span className="font-mono font-bold">Space</span>
                </div>
              </div>

              <div className="pt-2 border-t border-zinc-200 dark:border-zinc-800 flex items-center justify-between">
                <div>
                  <div className="font-semibold text-zinc-800 dark:text-zinc-200">
                    开机自动启动
                  </div>
                  <div className="text-xs text-zinc-500">
                    系统登录后自动运行菜单栏服务
                  </div>
                </div>
                <input
                  type="checkbox"
                  checked={config.launchAtLogin}
                  onChange={e => onUpdateConfig(prev => ({ ...prev, launchAtLogin: e.target.checked }))}
                  className="rounded border-zinc-300 text-blue-600 focus:ring-blue-500"
                />
              </div>
            </div>
          </div>

          {/* Section 2: Paste Behavior & Audio */}
          <div className="space-y-3">
            <h4 className="text-xs font-bold text-zinc-500 uppercase tracking-wider flex items-center gap-1.5">
              <Zap className="w-4 h-4" />
              <span>粘贴行为与音效</span>
            </h4>

            <div className="p-4 rounded-md bg-zinc-50 dark:bg-zinc-950 border border-zinc-200 dark:border-zinc-800 space-y-3 text-sm">
              <div className="flex items-center justify-between">
                <div>
                  <div className="font-semibold text-zinc-800 dark:text-zinc-200">
                    自动粘贴
                  </div>
                  <div className="text-xs text-zinc-500">
                    选择片段后写入剪贴板并自动模拟 Cmd+V 粘贴
                  </div>
                </div>
                <input
                  type="checkbox"
                  checked={config.autoPaste}
                  onChange={e => onUpdateConfig(prev => ({ ...prev, autoPaste: e.target.checked }))}
                  className="rounded border-zinc-300 text-blue-600 focus:ring-blue-500"
                />
              </div>

              <div className="pt-2 border-t border-zinc-200 dark:border-zinc-800 flex items-center justify-between">
                <div>
                  <div className="font-semibold text-zinc-800 dark:text-zinc-200">
                    操作音效
                  </div>
                  <div className="text-xs text-zinc-500">
                    按 Enter 粘贴或 Cmd+Enter 复制时播放轻柔按键音
                  </div>
                </div>
                <button
                  onClick={() => {
                    const next = !config.playAudioFeedback;
                    onUpdateConfig(prev => ({ ...prev, playAudioFeedback: next }));
                  }}
                  className={`p-1.5 rounded-md border text-xs flex items-center gap-1 ${
                    config.playAudioFeedback
                      ? 'bg-blue-50 dark:bg-blue-900/30 border-blue-300 text-blue-600 dark:text-blue-400'
                      : 'bg-zinc-200 dark:bg-zinc-800 border-zinc-300 dark:border-zinc-700 text-zinc-600 dark:text-zinc-400'
                  }`}
                >
                  {config.playAudioFeedback ? <Volume2 className="w-4 h-4" /> : <VolumeX className="w-4 h-4" />}
                  <span>{config.playAudioFeedback ? '开启' : '关闭'}</span>
                </button>
              </div>
            </div>
          </div>

          {/* Section 3: Appearance */}
          <div className="space-y-3">
            <h4 className="text-xs font-bold text-zinc-500 uppercase tracking-wider flex items-center gap-1.5">
              <Sun className="w-4 h-4" />
              <span>外观风格</span>
            </h4>

            <div className="p-4 rounded-md bg-zinc-50 dark:bg-zinc-950 border border-zinc-200 dark:border-zinc-800 space-y-4 text-sm">
              <div className="flex items-center justify-between">
                <span className="font-semibold text-zinc-800 dark:text-zinc-200">主题模式</span>
                <div className="flex items-center p-1 rounded-md bg-zinc-200 dark:bg-zinc-800">
                  <button
                    onClick={() => onUpdateConfig(prev => ({ ...prev, theme: 'light' }))}
                    className={`flex items-center gap-1 px-3 py-1 rounded-md text-xs font-medium transition-all ${
                      config.theme === 'light' ? 'bg-white text-zinc-900 shadow-sm' : 'text-zinc-600 dark:text-zinc-400'
                    }`}
                  >
                    <Sun className="w-3.5 h-3.5" />
                    <span>浅色</span>
                  </button>
                  <button
                    onClick={() => onUpdateConfig(prev => ({ ...prev, theme: 'dark' }))}
                    className={`flex items-center gap-1 px-3 py-1 rounded-md text-xs font-medium transition-all ${
                      config.theme === 'dark' ? 'bg-zinc-700 text-white shadow-sm' : 'text-zinc-600 dark:text-zinc-400'
                    }`}
                  >
                    <Moon className="w-3.5 h-3.5" />
                    <span>深色</span>
                  </button>
                </div>
              </div>
            </div>
          </div>

          {/* Section 4: Data Backup & Reset */}
          <div className="space-y-3">
            <h4 className="text-xs font-bold text-zinc-500 uppercase tracking-wider flex items-center gap-1.5">
              <HardDrive className="w-4 h-4" />
              <span>数据备份与管理 (本地优先)</span>
            </h4>

            <div className="p-4 rounded-md bg-zinc-50 dark:bg-zinc-950 border border-zinc-200 dark:border-zinc-800 space-y-3 text-sm">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div>
                  <div className="font-semibold text-zinc-800 dark:text-zinc-200">
                    导出片段 JSON 数据
                  </div>
                  <div className="text-xs text-zinc-500">
                    备份全部 key、标题与文本片段到本地 JSON 文件
                  </div>
                </div>
                <button
                  onClick={onExportData}
                  className="flex items-center gap-1 px-3 py-1.5 rounded-md bg-zinc-200 dark:bg-zinc-800 hover:bg-zinc-300 dark:hover:bg-zinc-700 text-zinc-800 dark:text-zinc-200 text-xs font-medium transition-colors"
                >
                  <Download className="w-3.5 h-3.5" />
                  <span>导出 JSON</span>
                </button>
              </div>

              <div className="pt-2 border-t border-zinc-200 dark:border-zinc-800 flex flex-wrap items-center justify-between gap-2">
                <div>
                  <div className="font-semibold text-zinc-800 dark:text-zinc-200">
                    导入 JSON 备份数据
                  </div>
                  <div className="text-xs text-zinc-500">
                    从已导出的备份文件恢复文本片段
                  </div>
                </div>
                <label className="flex items-center gap-1 px-3 py-1.5 rounded-md bg-zinc-200 dark:bg-zinc-800 hover:bg-zinc-300 dark:hover:bg-zinc-700 text-zinc-800 dark:text-zinc-200 text-xs font-medium transition-colors cursor-pointer">
                  <Upload className="w-3.5 h-3.5" />
                  <span>选择文件导入</span>
                  <input type="file" accept=".json" onChange={onImportData} className="hidden" />
                </label>
              </div>

              <div className="pt-2 border-t border-zinc-200 dark:border-zinc-800 flex flex-wrap items-center justify-between gap-2">
                <div>
                  <div className="font-semibold text-red-600 dark:text-red-400">
                    重置为内置示例数据
                  </div>
                  <div className="text-xs text-zinc-500">
                    清空当前数据并恢复初始化演示 Key 片段
                  </div>
                </div>
                <button
                  onClick={onResetSampleData}
                  className="flex items-center gap-1 px-3 py-1.5 rounded-md bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400 hover:bg-red-200 dark:hover:bg-red-900/50 text-xs font-medium transition-colors"
                >
                  <RotateCcw className="w-3.5 h-3.5" />
                  <span>恢复初始数据</span>
                </button>
              </div>
            </div>
          </div>

        </div>

      </div>
    </div>
  );
};
