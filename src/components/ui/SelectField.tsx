import React, { useEffect, useId, useRef, useState } from 'react';

export interface SelectOption {
  value: string;
  label: string;
}

interface SelectFieldProps {
  value: string;
  options: SelectOption[];
  onChange: (value: string) => void;
  ariaLabel: string;
  className?: string;
}

export const SelectField: React.FC<SelectFieldProps> = ({
  value,
  options,
  onChange,
  ariaLabel,
  className = '',
}) => {
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listId = `select-${useId().replace(/:/g, '')}`;
  const selectedIndex = Math.max(0, options.findIndex(option => option.value === value));
  const [open, setOpen] = useState(false);
  const [highlightedIndex, setHighlightedIndex] = useState(selectedIndex);
  const selected = options[selectedIndex] ?? options[0];

  useEffect(() => {
    setHighlightedIndex(selectedIndex);
  }, [selectedIndex]);

  useEffect(() => {
    if (!open) return;
    const handlePointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener('pointerdown', handlePointerDown);
    return () => document.removeEventListener('pointerdown', handlePointerDown);
  }, [open]);

  const commit = (index: number) => {
    const option = options[index];
    if (!option) return;
    onChange(option.value);
    setOpen(false);
    triggerRef.current?.focus();
  };

  const openAt = (index: number) => {
    setHighlightedIndex(Math.max(0, Math.min(index, options.length - 1)));
    setOpen(true);
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLButtonElement>) => {
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      openAt(open ? highlightedIndex + 1 : selectedIndex);
      return;
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      openAt(open ? highlightedIndex - 1 : selectedIndex);
      return;
    }
    if (event.key === 'Home' && open) {
      event.preventDefault();
      setHighlightedIndex(0);
      return;
    }
    if (event.key === 'End' && open) {
      event.preventDefault();
      setHighlightedIndex(options.length - 1);
      return;
    }
    if (event.key === 'Escape' && open) {
      event.preventDefault();
      setOpen(false);
      return;
    }
    if ((event.key === 'Enter' || event.key === ' ') && open) {
      event.preventDefault();
      commit(highlightedIndex);
      return;
    }
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      openAt(selectedIndex);
    }
  };

  return (
    <div ref={rootRef} className={`theme-select ${className}`.trim()}>
      <button
        ref={triggerRef}
        type="button"
        role="combobox"
        className="theme-select-trigger"
        aria-label={ariaLabel}
        aria-expanded={open}
        aria-controls={listId}
        aria-haspopup="listbox"
        aria-activedescendant={open ? `${listId}-option-${highlightedIndex}` : undefined}
        onClick={() => open ? setOpen(false) : openAt(selectedIndex)}
        onBlur={() => window.setTimeout(() => {
          if (!rootRef.current?.contains(document.activeElement)) setOpen(false);
        }, 0)}
        onKeyDown={handleKeyDown}
      >
        <span>{selected?.label ?? ''}</span>
        <span className={`theme-select-chevron ${open ? 'is-open' : ''}`} aria-hidden />
      </button>
      {open && (
        <div id={listId} className="theme-select-menu" role="listbox" aria-label={ariaLabel}>
          {options.map((option, index) => (
            <div
              id={`${listId}-option-${index}`}
              key={option.value}
              role="option"
              aria-selected={option.value === value}
              className={`theme-select-option ${index === highlightedIndex ? 'is-highlighted' : ''}`}
              onMouseEnter={() => setHighlightedIndex(index)}
              onMouseDown={event => event.preventDefault()}
              onClick={() => commit(index)}
            >
              {option.label}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
