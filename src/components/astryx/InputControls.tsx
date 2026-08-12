import React, { forwardRef } from 'react';

export interface TextInputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: React.ReactNode;
  error?: React.ReactNode;
  helperText?: React.ReactNode;
  leftIcon?: React.ReactNode;
  rightElement?: React.ReactNode;
  mono?: boolean;
  className?: string;
}

export const TextInput = forwardRef<HTMLInputElement, TextInputProps>(({
  label,
  error,
  helperText,
  leftIcon,
  rightElement,
  mono = false,
  className = '',
  id,
  ...rest
}, ref) => {
  const inputId = id || (typeof label === 'string' ? label.toLowerCase().replace(/\s+/g, '-') : undefined);

  return (
    <div className="w-full space-y-1">
      {label && (
        <label htmlFor={inputId} className="block text-xs font-semibold text-theme-secondary">
          {label}
        </label>
      )}
      <div className="relative flex items-center w-full">
        {leftIcon && (
          <div className="absolute left-3 text-theme-muted pointer-events-none flex items-center">
            {leftIcon}
          </div>
        )}
        <input
          ref={ref}
          id={inputId}
          className={`input-theme w-full px-3 py-1.5 rounded-lg text-xs transition-all ${
            leftIcon ? 'pl-9' : ''
          } ${rightElement ? 'pr-9' : ''} ${
            mono ? 'font-mono' : ''
          } ${error ? 'status-danger' : ''} ${className}`.trim()}
          {...rest}
        />
        {rightElement && (
          <div className="absolute right-3 flex items-center">
            {rightElement}
          </div>
        )}
      </div>
      {error && <p className="text-[11px] text-danger font-medium mt-0.5">{error}</p>}
      {!error && helperText && <p className="text-[11px] text-theme-muted mt-0.5">{helperText}</p>}
    </div>
  );
});

TextInput.displayName = 'TextInput';

export interface TextAreaProps extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: React.ReactNode;
  error?: React.ReactNode;
  helperText?: React.ReactNode;
  mono?: boolean;
  className?: string;
}

export const TextArea = forwardRef<HTMLTextAreaElement, TextAreaProps>(({
  label,
  error,
  helperText,
  mono = true,
  className = '',
  id,
  rows = 4,
  ...rest
}, ref) => {
  const inputId = id || (typeof label === 'string' ? label.toLowerCase().replace(/\s+/g, '-') : undefined);

  return (
    <div className="w-full space-y-1">
      {label && (
        <label htmlFor={inputId} className="block text-xs font-semibold text-theme-secondary">
          {label}
        </label>
      )}
      <textarea
        ref={ref}
        id={inputId}
        rows={rows}
        className={`input-theme w-full px-3 py-2 rounded-lg text-xs resize-y transition-all ${
          mono ? 'font-mono' : ''
        } ${error ? 'status-danger' : ''} ${className}`.trim()}
        {...rest}
      />
      {error && <p className="text-[11px] text-danger font-medium mt-0.5">{error}</p>}
      {!error && helperText && <p className="text-[11px] text-theme-muted mt-0.5">{helperText}</p>}
    </div>
  );
});

TextArea.displayName = 'TextArea';

export interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: React.ReactNode;
  description?: React.ReactNode;
  disabled?: boolean;
  className?: string;
}

export const Switch: React.FC<SwitchProps> = ({
  checked,
  onChange,
  label,
  description,
  disabled = false,
  className = '',
}) => {
  return (
    <label className={`flex items-start justify-between gap-3 cursor-pointer select-none ${disabled ? 'opacity-50 cursor-not-allowed' : ''} ${className}`}>
      {(label || description) && (
        <div className="space-y-0.5">
          {label && <span className="block text-xs font-semibold text-theme">{label}</span>}
          {description && <span className="block text-[11px] text-theme-muted">{description}</span>}
        </div>
      )}
      <div className="relative inline-flex items-center shrink-0 mt-0.5">
        <input
          type="checkbox"
          checked={checked}
          onChange={(e) => !disabled && onChange(e.target.checked)}
          disabled={disabled}
          className="sr-only peer"
        />
        <div className="w-9 h-5 bg-[color:var(--color-surface-sunken)] border theme-divider peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-[color:var(--color-border)] after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-[color:var(--color-accent)] peer-checked:border-[color:var(--color-accent)]"></div>
      </div>
    </label>
  );
};
