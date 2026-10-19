import React, { useCallback, useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import {
  AppError,
  BackupProgress,
  ExportPreview,
  ImportValidation,
  SettingsConfig,
  SettingsPaths,
  backupApi,
  settingsApi,
  settingsPathsApi,
} from '../api/snippets';
import {
  ArrowLeft,
  Database,
  Download,
  FolderOpen,
  HardDrive,
  Info,
  Keyboard,
  Monitor,
  Moon,
  RotateCcw,
  Search,
  ShieldCheck,
  Sparkles,
  Sun,
  Upload,
  Zap,
} from 'lucide-react';
import { THEME_LABELS, useTheme } from './ThemeProvider';
import { SelectField } from './ui/SelectField';

export type SettingsSection = 'shortcuts' | 'paste' | 'appearance' | 'files' | 'data' | 'backup' | 'about';

interface SettingsViewProps {
  config: SettingsConfig;
  /** 返回错误文案（成功为 null），以便分节内联显示。 */
  onUpdateConfig: (key: string, value: unknown) => Promise<string | null>;
  /** 要滚动到的分节；`nonce` 变化即再次定位（用于全局菜单“关于”）。 */
  target?: { section: SettingsSection; nonce: number } | null;
  onClose: () => void;
  /** 导入/重置后刷新片段、分组并重新扫描。 */
  onDataChanged: () => Promise<void>;
  showToast: (message: string, type?: 'success' | 'error') => void;
}

const SECTIONS: Array<{ id: SettingsSection; label: string; icon: typeof Keyboard }> = [
  { id: 'shortcuts', label: '快捷键与启动', icon: Keyboard },
  { id: 'paste', label: '粘贴行为', icon: Zap },
  { id: 'appearance', label: '外观主题', icon: Sun },
  { id: 'files', label: '文件与导出', icon: FolderOpen },
  { id: 'data', label: '检索与数据', icon: Search },
  { id: 'backup', label: '备份与重置', icon: HardDrive },
  { id: 'about', label: '关于', icon: Info },
];

const themes = [
  { id: 'light', label: THEME_LABELS.light, hint: '米纸底 · 森林绿', icon: Sun },
  { id: 'dark', label: THEME_LABELS.dark, hint: '暖炭底 · 陶土色', icon: Moon },
  { id: 'system', label: THEME_LABELS.system, hint: '按系统明暗自动切换', icon: Monitor },
] as const;

const formatBytes = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
};

const errorText = (error: unknown): string => {
  const err = error as AppError;
  const message = err?.message ?? '操作失败';
  return err?.code ? `${err.code}：${message}` : message;
};

type Busy = { label: string; percent: number | null } | null;
type Dialog =
  | { kind: 'export-sensitive'; preview: ExportPreview }
  | { kind: 'export-limit'; preview: ExportPreview }
  | { kind: 'import-preview'; path: string; validation: ImportValidation }
  | { kind: 'import-invalid'; lines: string[] }
  | { kind: 'error'; title: string; text: string }
  | { kind: 'reset' }
  | null;

export const SettingsView: React.FC<SettingsViewProps> = ({
  config,
  onUpdateConfig,
  target,
  onClose,
  onDataChanged,
  showToast,
}) => {
  const { setTheme, effectiveTheme } = useTheme();
  const [autostart, setAutostart] = useState(false);
  const [active, setActive] = useState<SettingsSection>('shortcuts');
  const [paths, setPaths] = useState<SettingsPaths | null>(null);
  const [exportDirDraft, setExportDirDraft] = useState('');
  const [exportDirError, setExportDirError] = useState<string | null>(null);
  const [exportDirHint, setExportDirHint] = useState<string | null>(null);
  const [dialog, setDialog] = useState<Dialog>(null);
  const [busy, setBusy] = useState<Busy>(null);
  const [lastExport, setLastExport] = useState<string | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const exportInputRef = useRef<HTMLInputElement>(null);
  /** 点击分节后一小段时间内不让滚动事件改写高亮（末尾分节无法滚到顶部）。 */
  const pinnedUntilRef = useRef(0);
  const busyRef = useRef(false);
  busyRef.current = busy !== null;
  const dialogRef = useRef<Dialog>(null);
  dialogRef.current = dialog;

  useEffect(() => {
    settingsApi.get().then(res => setAutostart(res.autostartActual)).catch(() => {});
    settingsPathsApi.paths().then(setPaths).catch(() => {});
  }, []);

  // 草稿跟随已保存的值（含恢复默认后回到默认目录）。
  const effectiveExportDir = config.exportDir || paths?.defaultExportDir || '';
  useEffect(() => {
    setExportDirDraft(effectiveExportDir);
  }, [effectiveExportDir]);

  // Esc 回到片段；有对话框或后台操作时不抢占。
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      if (busyRef.current) return;
      if (dialogRef.current) {
        if (dialogRef.current.kind === 'import-preview') {
          void backupApi.importDiscard(dialogRef.current.validation.importToken).catch(() => {});
        }
        setDialog(null);
        return;
      }
      onClose();
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

  // 备份进度事件
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<BackupProgress>('backup-progress', event => {
      const { done, total } = event.payload;
      setBusy(prev => (prev ? { ...prev, percent: total > 0 ? Math.min(100, Math.round((done / total) * 100)) : null } : prev));
    }).then(fn => { unlisten = fn; }).catch(() => {});
    return () => unlisten?.();
  }, []);

  const scrollTo = useCallback((section: SettingsSection) => {
    setActive(section);
    pinnedUntilRef.current = Date.now() + 800;
    document.getElementById(`settings-${section}`)?.scrollIntoView({ block: 'start' });
  }, []);

  useEffect(() => {
    if (target) scrollTo(target.section);
  }, [target, scrollTo]);

  const handleScroll = () => {
    const container = scrollRef.current;
    if (!container || Date.now() < pinnedUntilRef.current) return;
    const top = container.getBoundingClientRect().top + 96;
    let current: SettingsSection = SECTIONS[0].id;
    for (const { id } of SECTIONS) {
      const el = document.getElementById(`settings-${id}`);
      if (el && el.getBoundingClientRect().top <= top) current = id;
    }
    setActive(current);
  };

  const handleThemeChange = (theme: 'light' | 'dark' | 'system') => {
    setTheme(theme);
    void onUpdateConfig('theme', theme);
  };

  const updateAutostart = (enabled: boolean) => {
    void onUpdateConfig('launchAtLogin', enabled);
    settingsApi.autostartSet(enabled).then(setAutostart).catch(() => {});
  };

  // ---------------------------------------------------------------- 文件与导出

  const saveExportDir = async (value: string | null) => {
    const error = await onUpdateConfig('exportDir', value);
    setExportDirError(error);
    if (error) {
      // 保存失败：输入框保持用户键入的值以便修改，已保存的目录不变。
      return;
    }
    setExportDirHint(null);
  };

  const pickExportDir = async () => {
    try {
      const picked = await settingsPathsApi.pickDirectory(config.exportDir || paths?.defaultExportDir);
      if (picked) {
        setExportDirDraft(picked);
        await saveExportDir(picked);
      }
    } catch {
      setExportDirHint('系统目录对话框不可用，请直接输入目录路径后点击“应用”。');
      exportInputRef.current?.focus();
    }
  };

  const openTemplatesDir = async () => {
    try {
      await settingsPathsApi.openTemplatesDir();
    } catch (e) {
      showToast(`打开模板目录失败：${errorText(e)}`, 'error');
    }
  };

  // ---------------------------------------------------------------- 备份与重置

  const withBusy = async <T,>(label: string, work: () => Promise<T>): Promise<T> => {
    setBusy({ label, percent: null });
    try {
      return await work();
    } finally {
      setBusy(null);
    }
  };

  const doExport = async () => {
    try {
      const chosen = await backupApi.pickExportPath();
      if (!chosen) return;
      const outcome = await withBusy('正在导出备份…', () => backupApi.exportConfirm(chosen));
      setLastExport(outcome.path);
      showToast(`已导出备份：${outcome.snippetCount} 个条目，${outcome.templateCount} 个模板文件（${formatBytes(outcome.sizeBytes)}）`);
    } catch (e) {
      setDialog({ kind: 'error', title: '导出失败', text: errorText(e) });
    }
  };

  const startExport = async () => {
    try {
      const preview = await backupApi.exportPreview();
      if (preview.exceedsLimit) {
        setDialog({ kind: 'export-limit', preview });
      } else if (preview.sensitiveCount > 0) {
        setDialog({ kind: 'export-sensitive', preview });
      } else {
        await doExport();
      }
    } catch (e) {
      setDialog({ kind: 'error', title: '导出失败', text: errorText(e) });
    }
  };

  const startImport = async () => {
    try {
      const chosen = await backupApi.pickImportFile();
      if (!chosen) return;
      const validation = await withBusy('正在校验并解压备份…', () => backupApi.importValidate(chosen));
      if (!validation.valid) {
        setDialog({ kind: 'import-invalid', lines: validation.errors });
        return;
      }
      setDialog({ kind: 'import-preview', path: chosen, validation });
    } catch (e) {
      setDialog({ kind: 'error', title: '导入失败', text: errorText(e) });
    }
  };

  const cancelImport = (validation: ImportValidation) => {
    void backupApi.importDiscard(validation.importToken).catch(() => {});
    setDialog(null);
  };

  const commitImport = async (path: string, validation: ImportValidation, overwrite: boolean) => {
    setDialog(null);
    try {
      const outcome = await withBusy('正在导入备份…', () => backupApi.importCommit(path, validation.importToken, overwrite));
      const parts = [`新增 ${outcome.inserted}`, `更新 ${outcome.updated}`];
      if (outcome.replaced > 0) parts.push(`覆盖冲突 ${outcome.replaced}`);
      if (outcome.skipped > 0) parts.push(`跳过 ${outcome.skipped}`);
      if (outcome.groupsAdded > 0) parts.push(`新增分组 ${outcome.groupsAdded}`);
      showToast(`导入完成：${parts.join(' · ')}`);
      await onDataChanged();
    } catch (e) {
      setDialog({ kind: 'error', title: '导入失败', text: errorText(e) });
    }
  };

  const doReset = async () => {
    setDialog(null);
    try {
      const count = await withBusy('正在重置示例数据…', () => backupApi.resetExamples());
      showToast(`已重置：写入 ${count} 条示例片段，文件模板已清空`);
      await onDataChanged();
    } catch (e) {
      setDialog({ kind: 'error', title: '重置失败', text: errorText(e) });
    }
  };

  const draftDirty = exportDirDraft.trim() !== effectiveExportDir;

  return (
    <div className="manager-workspace settings-view" data-testid="settings-view">
      <aside className="manager-sidebar" aria-label="设置分节">
        <div className="manager-sidebar-heading">设置</div>
        <div className="manager-side-scroll">
          <button
            type="button"
            className="manager-side-item"
            onClick={onClose}
            aria-label="返回片段"
          >
            <span className="manager-side-icon"><ArrowLeft strokeWidth={1.5} className="size-4" aria-hidden /></span>
            <span className="manager-side-label">返回片段</span>
            <span className="manager-side-count">Esc</span>
          </button>
          <div className="manager-side-divider" />
          {SECTIONS.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              type="button"
              className={`manager-side-item ${active === id ? 'is-active' : ''}`}
              onClick={() => scrollTo(id)}
              aria-current={active === id ? 'true' : undefined}
              data-section={id}
            >
              <span className="manager-side-icon"><Icon strokeWidth={1.5} className="size-4" aria-hidden /></span>
              <span className="manager-side-label">{label}</span>
            </button>
          ))}
        </div>
      </aside>

      <div className="settings-scroll" ref={scrollRef} onScroll={handleScroll}>
        <div className="settings-page" role="region" aria-labelledby="settings-title">
          <h1 id="settings-title" className="settings-nav-title">偏好设置</h1>

          <div className="settings-page-body">
            <section className="settings-section" id="settings-shortcuts" aria-label="快捷键与启动">
              <div className="settings-section-heading">
                <div className="settings-section-icon"><Keyboard strokeWidth={1.5} className="size-4" aria-hidden /></div>
                <div><h2>快捷键与启动</h2><p>用一个组合键从桌面任意位置呼出快速检索。</p></div>
              </div>
              <div className="settings-row">
                <div><strong>全局呼出快捷键</strong><span>注册失败时会保留上一次可用快捷键。</span></div>
                <input
                  type="text"
                  value={config.globalShortcut}
                  onChange={event => void onUpdateConfig('globalShortcut', event.target.value)}
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

            <section className="settings-section" id="settings-paste" aria-label="粘贴行为">
              <div className="settings-section-heading">
                <div className="settings-section-icon"><Zap strokeWidth={1.5} className="size-4" aria-hidden /></div>
                <div><h2>粘贴行为</h2><p>决定选中片段后是否尝试向原窗口发送 Ctrl+V。</p></div>
              </div>
              <div className="settings-row">
                <div><strong>自动粘贴</strong><span>关闭后仍可复制到剪贴板，但不会注入键盘事件。</span></div>
                <input type="checkbox" className="manager-switch" checked={config.autoPaste} onChange={event => void onUpdateConfig('autoPaste', event.target.checked)} aria-label="自动粘贴" />
              </div>
              <div className="settings-rule" />
              <div className="settings-constraint">
                <strong>剪贴板恢复已关闭</strong>
                <span>V1 不保存或恢复用户原有剪贴板内容。</span>
              </div>
            </section>

            <section className="settings-section" id="settings-appearance" aria-label="外观主题">
              <div className="settings-section-heading">
                <div className="settings-section-icon"><Sun strokeWidth={1.5} className="size-4" aria-hidden /></div>
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
                    <Icon strokeWidth={1.5} className="size-4" aria-hidden />
                    <span><strong>{label}</strong><small>{hint}</small></span>
                    {config.theme === id && <span className="settings-theme-check" aria-hidden>✓</span>}
                  </button>
                ))}
              </div>
              <p className="settings-state-note">当前渲染：{THEME_LABELS[effectiveTheme]}</p>
            </section>

            <section className="settings-section" id="settings-files" aria-label="文件与导出">
              <div className="settings-section-heading">
                <div className="settings-section-icon"><FolderOpen strokeWidth={1.5} className="size-4" aria-hidden /></div>
                <div><h2>文件与导出</h2><p>文件模板的导出位置与存放目录。</p></div>
              </div>
              <div className="settings-field">
                <label htmlFor="export-dir-input"><strong>导出目录</strong></label>
                <span className="settings-field-note">快搜窗口对文件模板按 Shift+Enter 时，文件导出到这里。未设置时使用系统下载目录。</span>
                <div className="settings-path-row">
                  <input
                    id="export-dir-input"
                    ref={exportInputRef}
                    type="text"
                    className="manager-control manager-mono settings-path-input"
                    value={exportDirDraft}
                    onChange={event => { setExportDirDraft(event.target.value); setExportDirError(null); }}
                    onKeyDown={event => { if (event.key === 'Enter' && draftDirty) void saveExportDir(exportDirDraft.trim() || null); }}
                    placeholder={paths?.defaultExportDir ?? '未找到系统下载目录，请选择一个目录'}
                    spellCheck={false}
                    aria-label="导出目录"
                    aria-invalid={exportDirError ? true : undefined}
                    aria-describedby={exportDirError ? 'export-dir-error' : undefined}
                  />
                  {draftDirty && (
                    <button type="button" className="manager-button manager-button-secondary" onClick={() => void saveExportDir(exportDirDraft.trim() || null)}>应用</button>
                  )}
                  <button type="button" className="manager-button manager-button-secondary" onClick={() => void pickExportDir()}>选择…</button>
                  <button
                    type="button"
                    className="manager-button manager-button-secondary"
                    onClick={() => { setExportDirError(null); void saveExportDir(null); }}
                    disabled={!config.exportDir}
                  >恢复默认</button>
                </div>
                {exportDirError && <p id="export-dir-error" className="settings-field-error" role="alert" data-testid="export-dir-error">{exportDirError}</p>}
                {exportDirHint && <p className="settings-state-note">{exportDirHint}</p>}
                <p className="settings-state-note" data-testid="export-dir-state">
                  {config.exportDir ? '使用自定义导出目录。' : '使用默认下载目录。'}
                </p>
              </div>
              <div className="settings-rule" />
              <div className="settings-note-block">
                <strong>导出文件名规则</strong>
                <span>标题_日期.扩展名，例如 周报_2026-10-19.md；重名自动加 (2)、(3)，不会覆盖已有文件。</span>
              </div>
              <div className="settings-rule" />
              <div className="settings-action-row">
                <div>
                  <strong>模板目录（只读）</strong>
                  <span className="manager-mono settings-path-text" data-testid="templates-dir">{paths?.templatesDir ?? '…'}</span>
                </div>
                <button type="button" className="manager-button manager-button-secondary" onClick={() => void openTemplatesDir()}>
                  <FolderOpen strokeWidth={1.5} className="size-3.5" aria-hidden />打开
                </button>
              </div>
              <div className="settings-constraint settings-plaintext-note">
                <strong>明文存放</strong>
                <span>文件模板以明文存放，不受数据库加密保护。</span>
              </div>
            </section>

            <section className="settings-section" id="settings-data" aria-label="检索与数据">
              <div className="settings-section-heading">
                <div className="settings-section-icon"><Database strokeWidth={1.5} className="size-4" aria-hidden /></div>
                <div><h2>检索与数据</h2><p>这些值会写入本地设置，并在重启后保持。</p></div>
              </div>
              <div className="settings-row">
                <div><strong>最大结果数</strong><span>快速检索一次最多展示多少条结果。</span></div>
                <SelectField
                  value={String(config.maxResultsCount)}
                  options={[5, 10, 20, 50, 100].map(value => ({ value: String(value), label: `${value} 条` }))}
                  onChange={value => void onUpdateConfig('maxResultsCount', Number(value))}
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
                  onChange={value => void onUpdateConfig('trashAutoPurgeDays', value ? Number(value) : null)}
                  ariaLabel="回收站自动清理"
                  className="settings-select"
                />
              </div>
            </section>

            <section className="settings-section" id="settings-backup" aria-label="备份与重置">
              <div className="settings-section-heading">
                <div className="settings-section-icon"><HardDrive strokeWidth={1.5} className="size-4" aria-hidden /></div>
                <div>
                  <h2>备份与重置</h2>
                  <p>备份为 zip 压缩包（backup.json + 模板文件），全部为明文；备份包含文件模板原文件，含敏感片段时请确认存放位置。</p>
                </div>
              </div>
              <div className="settings-action-row">
                <div>
                  <strong>导出备份</strong>
                  <span>分组、片段、回收站记录、允许备份的设置与全部模板文件。</span>
                  {lastExport && <span className="manager-mono settings-path-text" data-testid="last-export">上次导出：{lastExport}</span>}
                </div>
                <button type="button" className="manager-button manager-button-secondary" onClick={() => void startExport()} disabled={busy !== null} data-testid="export-backup">
                  <Download strokeWidth={1.5} className="size-3.5" aria-hidden />导出…
                </button>
              </div>
              <div className="settings-rule" />
              <div className="settings-action-row">
                <div>
                  <strong>导入备份</strong>
                  <span>接受 zip 备份包和旧版 JSON 备份；先校验并预览，确认后才写入。</span>
                </div>
                <button type="button" className="manager-button manager-button-secondary" onClick={() => void startImport()} disabled={busy !== null} data-testid="import-backup">
                  <Upload strokeWidth={1.5} className="size-3.5" aria-hidden />选择文件…
                </button>
              </div>
              <div className="settings-rule" />
              <div className="settings-action-row settings-danger-row">
                <div>
                  <strong>重置示例数据</strong>
                  <span>清除全部片段、分组和文件模板（含文件），再写入内置示例。建议先导出备份。</span>
                </div>
                <button type="button" className="manager-button manager-button-danger" onClick={() => setDialog({ kind: 'reset' })} disabled={busy !== null} data-testid="reset-examples">
                  <RotateCcw strokeWidth={1.5} className="size-3.5" aria-hidden />重置
                </button>
              </div>
            </section>

            <section className="settings-section" id="settings-about" aria-label="关于">
              <div className="settings-section-heading">
                <div className="settings-section-icon"><Info strokeWidth={1.5} className="size-4" aria-hidden /></div>
                <div><h2>关于 Searchis</h2><p>版本 0.1.0</p></div>
              </div>
              <div className="settings-about">
                <div className="settings-about-mark brand-mark" aria-hidden>S</div>
                <p className="settings-about-text">
                  面向 Arch Linux、KDE Plasma 6、X11 的本机加密文本片段检索、复制与快捷管理工具。
                </p>
              </div>
              <div className="settings-about-grid">
                <div className="info-card settings-about-card">
                  <div className="settings-about-card-title"><ShieldCheck strokeWidth={1.5} className="size-3.5" aria-hidden />本地加密</div>
                  <p>SQLCipher 数据库级加密存储</p>
                </div>
                <div className="info-card settings-about-card">
                  <div className="settings-about-card-title"><Sparkles strokeWidth={1.5} className="size-3.5" aria-hidden />系统集成</div>
                  <p>KDE Global Menu / KGlobalAccel</p>
                </div>
              </div>
            </section>
          </div>
        </div>
      </div>

      {dialog?.kind === 'export-sensitive' && (
        <div className="manager-confirm-backdrop" role="presentation">
          <section className="manager-confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby="export-confirm-title" data-testid="export-sensitive-dialog">
            <h2 id="export-confirm-title">备份将包含明文敏感内容</h2>
            <p>
              库中有 {dialog.preview.sensitiveCount} 条敏感片段，备份文件中的正文可被任何人直接读取；
              备份还包含 {dialog.preview.templateCount} 个文件模板的原文件。请确认存放位置安全。
            </p>
            <div className="manager-confirm-actions">
              <button type="button" className="manager-button manager-button-secondary" onClick={() => setDialog(null)}>取消</button>
              <button type="button" className="manager-button manager-button-primary" onClick={() => { setDialog(null); void doExport(); }} data-testid="export-sensitive-confirm">我已了解，继续导出</button>
            </div>
          </section>
        </div>
      )}

      {dialog?.kind === 'export-limit' && (
        <div className="manager-confirm-backdrop" role="presentation">
          <section className="manager-confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby="export-limit-title">
            <h2 id="export-limit-title">备份内容超过上限</h2>
            <p>
              估算大小 {formatBytes(dialog.preview.estimatedSizeBytes)}（其中模板文件 {formatBytes(dialog.preview.templateBytes)}），
              超过上限（backup.json ≤ 100 MB，压缩包 ≤ 1 GB）。请先清理部分模板文件后重试。
            </p>
            <div className="manager-confirm-actions">
              <button type="button" className="manager-button manager-button-secondary" onClick={() => setDialog(null)}>知道了</button>
            </div>
          </section>
        </div>
      )}

      {dialog?.kind === 'import-preview' && (
        <div className="manager-confirm-backdrop" role="presentation">
          <section className="manager-confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby="import-preview-title" data-testid="import-preview-dialog">
            <h2 id="import-preview-title">导入备份预览</h2>
            <p data-testid="import-preview-summary">
              {dialog.validation.backupVersion === 2 ? 'zip 备份包' : '旧版 JSON 备份'}：
              新增 {dialog.validation.newCount} 条，更新 {dialog.validation.updateCount} 条；
              {dialog.validation.groupCount} 个分组，{dialog.validation.templateCount} 个文件模板。
              {dialog.validation.conflictCount === 0 && '没有 Key 冲突。'}
            </p>
            {dialog.validation.conflictCount > 0 && (
              <div className="settings-constraint" data-testid="import-conflicts">
                <strong>{dialog.validation.conflictCount} 个 Key 与现有条目冲突</strong>
                <span className="manager-mono">{dialog.validation.conflictKeys.join('、')}{dialog.validation.conflictCount > dialog.validation.conflictKeys.length ? ' …' : ''}</span>
                <span>选择“用导入记录覆盖”会删除这些现有条目（含其文件）；选择取消则不导入任何内容。</span>
              </div>
            )}
            <div className="manager-confirm-actions">
              <button type="button" className="manager-button manager-button-secondary" onClick={() => cancelImport(dialog.validation)}>取消</button>
              {dialog.validation.conflictCount > 0 ? (
                <button type="button" className="manager-button manager-button-danger" onClick={() => void commitImport(dialog.path, dialog.validation, true)} data-testid="import-overwrite">用导入记录覆盖并导入</button>
              ) : (
                <button type="button" className="manager-button manager-button-primary" onClick={() => void commitImport(dialog.path, dialog.validation, false)} data-testid="import-confirm">确认导入</button>
              )}
            </div>
          </section>
        </div>
      )}

      {dialog?.kind === 'import-invalid' && (
        <div className="manager-confirm-backdrop" role="presentation">
          <section className="manager-confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby="import-invalid-title" data-testid="backup-error">
            <h2 id="import-invalid-title">备份文件无效</h2>
            <p>IMPORT_SCHEMA_INVALID：备份内容未通过校验，未导入任何内容。</p>
            <ul className="settings-error-list">
              {dialog.lines.map((line, index) => <li key={index}>{line}</li>)}
            </ul>
            <div className="manager-confirm-actions">
              <button type="button" className="manager-button manager-button-secondary" onClick={() => setDialog(null)}>知道了</button>
            </div>
          </section>
        </div>
      )}

      {dialog?.kind === 'error' && (
        <div className="manager-confirm-backdrop" role="presentation">
          <section className="manager-confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby="backup-error-title" data-testid="backup-error">
            <h2 id="backup-error-title">{dialog.title}</h2>
            <p data-testid="backup-error-text">{dialog.text}</p>
            <div className="manager-confirm-actions">
              <button type="button" className="manager-button manager-button-secondary" onClick={() => setDialog(null)}>知道了</button>
            </div>
          </section>
        </div>
      )}

      {dialog?.kind === 'reset' && (
        <div className="manager-confirm-backdrop" role="presentation">
          <section className="manager-confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby="reset-title" data-testid="reset-dialog">
            <h2 id="reset-title">重置示例数据？</h2>
            <p>
              将清除当前所有片段、分组和文件模板（模板文件也会被删除），并写入内置示例数据。此操作不可撤销，建议先导出备份。
            </p>
            <div className="manager-confirm-actions">
              <button type="button" className="manager-button manager-button-secondary" onClick={() => setDialog(null)}>取消</button>
              <button type="button" className="manager-button manager-button-danger" onClick={() => void doReset()} data-testid="reset-confirm">确认重置</button>
            </div>
          </section>
        </div>
      )}

      {busy && (
        <div className="manager-confirm-backdrop settings-busy" role="presentation" data-testid="backup-busy">
          <section className="manager-confirm-dialog" role="alertdialog" aria-modal="true" aria-busy="true" aria-labelledby="busy-title">
            <h2 id="busy-title">{busy.label}</h2>
            <div
              className="settings-progress"
              role="progressbar"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={busy.percent ?? undefined}
              data-indeterminate={busy.percent === null ? 'true' : undefined}
            >
              <span style={busy.percent === null ? undefined : { width: `${busy.percent}%` }} />
            </div>
            <p>{busy.percent === null ? '请稍候，期间管理窗口暂停写入。' : `${busy.percent}%`}</p>
          </section>
        </div>
      )}
    </div>
  );
};
