import React, { createContext, useContext, useEffect, useState } from 'react';
import { settingsApi } from '../api/snippets';

export type ThemeMode = 'light' | 'dark' | 'system';

interface ThemeContextType {
  theme: ThemeMode;
  effectiveTheme: 'light' | 'dark';
  isDark: boolean;
  setTheme: (theme: ThemeMode) => void;
  toggleTheme: () => void;
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

// Backend settings are the source of truth. localStorage is only a render/write
// cache: it seeds the first paint (flash prevention) and mirrors writes so the
// quick-search window can hydrate instantly before its backend sync lands.
const readCache = (): ThemeMode | null => {
  try {
    const saved = JSON.parse(localStorage.getItem('searchis_config_v1') ?? '{}') as { theme?: ThemeMode };
    if (saved.theme === 'light' || saved.theme === 'dark' || saved.theme === 'system') return saved.theme;
  } catch {
    // Cache unreadable: fall through to backend / default.
  }
  return null;
};

const writeCache = (theme: ThemeMode) => {
  try {
    const saved = JSON.parse(localStorage.getItem('searchis_config_v1') ?? '{}');
    localStorage.setItem('searchis_config_v1', JSON.stringify({ ...saved, theme }));
  } catch {
    // Cache write failures are non-fatal; the backend remains authoritative.
  }
};

export const ThemeProvider: React.FC<{
  children: React.ReactNode;
  initialTheme?: ThemeMode;
}> = ({ children, initialTheme = 'system' }) => {
  const [theme, setThemeState] = useState<ThemeMode>(() => readCache() ?? initialTheme);
  const [effectiveTheme, setEffectiveTheme] = useState<'light' | 'dark'>('dark');

  // Hydrate from the backend on mount; a stale/absent cache is corrected here.
  useEffect(() => {
    let cancelled = false;
    settingsApi.get().then(({ settings }) => {
      if (!cancelled && (settings.theme === 'light' || settings.theme === 'dark' || settings.theme === 'system')) {
        setThemeState(settings.theme);
      }
    }).catch(() => {}); // Backend unavailable: keep the cache-seeded value.
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    let transitionTimeout: ReturnType<typeof setTimeout>;

    const updateTheme = () => {
      const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      const isDark = theme === 'dark' || (theme === 'system' && prefersDark);
      const active = isDark ? 'dark' : 'light';

      setEffectiveTheme(active);
      document.documentElement.classList.add('theme-transition');
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

  const setTheme = (newTheme: ThemeMode) => {
    setThemeState(newTheme);
    writeCache(newTheme);
  };

  const toggleTheme = () => setTheme(effectiveTheme === 'dark' ? 'light' : 'dark');

  return (
    <ThemeContext.Provider value={{ theme, effectiveTheme, isDark: effectiveTheme === 'dark', setTheme, toggleTheme }}>
      {children}
    </ThemeContext.Provider>
  );
};

export const useTheme = () => {
  const context = useContext(ThemeContext);
  if (!context) throw new Error('useTheme must be used within a ThemeProvider');
  return context;
};
