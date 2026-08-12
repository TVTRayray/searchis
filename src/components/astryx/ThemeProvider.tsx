import React, { createContext, useContext, useEffect, useState } from 'react';

export type ThemeMode = 'light' | 'dark' | 'system';

interface ThemeContextType {
  theme: ThemeMode;
  effectiveTheme: 'light' | 'dark';
  isDark: boolean;
  setTheme: (theme: ThemeMode) => void;
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

  const setTheme = (newTheme: ThemeMode) => {
    setThemeState(newTheme);
    try {
      const saved = JSON.parse(localStorage.getItem('searchis_config_v1') ?? '{}');
      localStorage.setItem('searchis_config_v1', JSON.stringify({ ...saved, theme: newTheme }));
    } catch {
      // ignore
    }
  };

  const toggleTheme = () => {
    const next = effectiveTheme === 'dark' ? 'light' : 'dark';
    setTheme(next);
  };

  return (
    <ThemeContext.Provider value={{ theme, effectiveTheme, isDark: effectiveTheme === 'dark', setTheme, toggleTheme }}>
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
