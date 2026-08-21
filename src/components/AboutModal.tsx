import React from 'react';
import { Info, X, ShieldCheck, Sparkles } from 'lucide-react';

interface AboutModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const AboutModal: React.FC<AboutModalProps> = ({ isOpen, onClose }) => {
  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-3 theme-backdrop backdrop-blur-xs select-none"
      onClick={onClose}
    >
      <section
        className="raycast-window max-w-md w-full rounded-xl overflow-hidden animate-pop-in"
        role="dialog"
        aria-modal="true"
        aria-labelledby="about-dialog-title"
        onClick={event => event.stopPropagation()}
      >
        <header className="flex items-center justify-between px-5 py-3 theme-titlebar border-b">
          <div className="flex items-center gap-2">
            <Info className="w-4 h-4 icon-accent" aria-hidden />
            <h2 id="about-dialog-title" className="text-sm font-bold text-theme">关于 Searchis</h2>
          </div>
          <button
            type="button"
            className="manager-icon-button"
            aria-label="关闭关于窗口"
            title="关闭关于窗口"
            onClick={onClose}
          >
            <X className="w-4 h-4" aria-hidden />
          </button>
        </header>

        <div className="p-6 text-center space-y-4">
          <div className="mx-auto w-12 h-12 rounded-xl flex items-center justify-center brand-mark text-lg font-bold">
            S
          </div>
          <div>
            <h3 className="text-base font-bold text-theme">Searchis</h3>
            <p className="text-xs text-theme-secondary mt-1">版本 0.1.0</p>
          </div>

          <p className="text-xs text-theme-secondary leading-relaxed max-w-xs mx-auto">
            面向 Arch Linux、KDE Plasma 6、X11 的本机加密文本片段检索、复制与快捷管理工具。
          </p>

          <div className="grid grid-cols-2 gap-2 pt-2 text-left">
            <div className="p-2.5 rounded-lg bg-[var(--color-surface-subtle)] border border-[var(--color-border-subtle)] text-[11px]">
              <div className="flex items-center gap-1.5 font-semibold text-theme">
                <ShieldCheck className="w-3.5 h-3.5 text-[var(--color-accent-fg)]" />
                本地加密
              </div>
              <p className="text-[10px] text-theme-secondary mt-0.5">SQLCipher 数据库级加密存储</p>
            </div>
            <div className="p-2.5 rounded-lg bg-[var(--color-surface-subtle)] border border-[var(--color-border-subtle)] text-[11px]">
              <div className="flex items-center gap-1.5 font-semibold text-theme">
                <Sparkles className="w-3.5 h-3.5 text-[var(--color-accent-fg)]" />
                系统集成
              </div>
              <p className="text-[10px] text-theme-secondary mt-0.5">KDE Global Menu / KGlobalAccel</p>
            </div>
          </div>
        </div>

        <footer className="flex justify-end px-5 py-3 theme-titlebar border-t">
          <button
            type="button"
            className="btn-primary inline-flex items-center justify-center gap-1.5 cursor-pointer px-3 py-1.5 text-xs rounded-md font-semibold"
            onClick={onClose}
          >
            确定 (Esc)
          </button>
        </footer>
      </section>
    </div>
  );
};
