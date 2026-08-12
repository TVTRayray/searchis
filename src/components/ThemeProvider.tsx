import React, { createContext, useContext, useEffect, useState } from 'react';
import { AccentMode } from '../types/snippet';

export type ThemeMode = 'light' | 'dark' | 'system';

interface ThemeContextType {
  theme: ThemeMode;
  effectiveTheme: 'light' | 'dark';
  isDark: boolean;
  accent?: AccentMode;
  setTheme: (theme: ThemeMode) => void;
  setAccent: (accent?: AccentMode) => void;
  toggleTheme: () => void;
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

export const ThemeProvider: React.FC<{
  children: React.ReactNode;
  initialTheme?: ThemeMode;
}> = ({ children, initialTheme = 'system' }) => {
  const [theme, setThemeState] = useState<ThemeMode>(() => {
    try {
      const saved = JSON.parse(localStorage.getItem('searchis_config_v1') ?? '{}') as { theme?: ThemeMode };
      return saved.theme || initialTheme;
    } catch {
      return initialTheme;
    }
  });

  const [accent, setAccentState] = useState<AccentMode | undefined>(() => {
    try {
      const saved = JSON.parse(localStorage.getItem('searchis_config_v1') ?? '{}') as { accent?: AccentMode };
      return saved.accent;
    } catch {
      return undefined;
    }
  });

  const [effectiveTheme, setEffectiveTheme] = useState<'light' | 'dark'>('dark');

  useEffect(() => {
    let transitionTimeout: ReturnType<typeof setTimeout>;

    const updateTheme = () => {
      const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      const isDark = theme === 'dark' || (theme === 'system' && prefersDark);
      const active = isDark ? 'dark' : 'light';

      setEffectiveTheme(active);

      document.documentElement.classList.add('theme-transition');
      // Apply data-theme and dark class to documentElement
      document.documentElement.setAttribute('data-theme', active);
      document.documentElement.classList.toggle('dark', isDark);

      clearTimeout(transitionTimeout);
      transitionTimeout = setTimeout(() => {
        document.documentElement.classList.remove('theme-transition');
      }, 300);
    };

    updateTheme();

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleChange = () => {
      if (theme === 'system') updateTheme();
    };

    mediaQuery.addEventListener('change', handleChange);
    return () => {
      mediaQuery.removeEventListener('change', handleChange);
      clearTimeout(transitionTimeout);
    };
  }, [theme]);

  // Apply accent attribute whenever it changes
  useEffect(() => {
    const root = document.documentElement;
    if (accent) root.setAttribute('data-accent', accent);
    else root.removeAttribute('data-accent');
  }, [accent]);

  const setTheme = (newTheme: ThemeMode) => {
    setThemeState(newTheme);
    try {
      const saved = JSON.parse(localStorage.getItem('searchis_config_v1') ?? '{}');
      localStorage.setItem('searchis_config_v1', JSON.stringify({ ...saved, theme: newTheme }));
    } catch {
      // ignore
    }
  };

  const setAccent = (newAccent?: AccentMode) => {
    setAccentState(newAccent);
    try {
      const saved = JSON.parse(localStorage.getItem('searchis_config_v1') ?? '{}');
      if (newAccent) localStorage.setItem('searchis_config_v1', JSON.stringify({ ...saved, accent: newAccent }));
      else {
        const { accent: _drop, ...rest } = saved;
        localStorage.setItem('searchis_config_v1', JSON.stringify(rest));
      }
    } catch {
      // ignore
    }
  };

  const toggleTheme = () => {
    const next = effectiveTheme === 'dark' ? 'light' : 'dark';
    setTheme(next);
  };

  return (
    <ThemeContext.Provider value={{ theme, effectiveTheme, isDark: effectiveTheme === 'dark', accent, setTheme, setAccent, toggleTheme }}>
      {children}
    </ThemeContext.Provider>
  );
};

export const useTheme = () => {
  const context = useContext(ThemeContext);
  if (!context) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return context;
};
