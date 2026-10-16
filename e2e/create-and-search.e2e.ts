import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { browser } from '@wdio/globals';
import { withExecuteOptions } from '@wdio/tauri-service';

const exec = promisify(execFile);

const suffix = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
const snippets = [
  { key: `e2e-first-${suffix}`, title: `E2E 第一条 ${suffix}`, content: 'E2E first content' },
  { key: `e2e-second-${suffix}`, title: `E2E 第二条 ${suffix}`, content: 'E2E second content' },
  { key: `e2e-sensitive-${suffix}`, title: `E2E 敏感 ${suffix}`, content: `SECRET_E2E_${suffix}`, sensitive: true },
];

const inputSelectors = {
  key: 'input[placeholder="例如 email-work, addr-office"]',
  title: 'input[placeholder="例如 工作邮箱, 公司地址"]',
  // SPEC-21：正文改为 CodeMirror 6，可编辑区域是 .cm-content。
  content: '.cm-content',
  search: 'input[aria-label="检索文本片段"]',
};

const windowOptions = (windowLabel: string) => withExecuteOptions({ windowLabel });
type TauriScript = (tauri: unknown, ...args: unknown[]) => unknown;
type SearchItem = {
  id: string;
  key: string;
  title: string;
  aliases: string[];
  tags: string[];
  pinned: boolean;
  sensitive: boolean;
  usageCount: number;
  lastUsedAt: string | null;
  language?: string;
  groupName?: string | null;
  kind?: string;
};

const inWindow = (windowLabel: string, script: TauriScript, ...args: unknown[]) =>
  browser.tauri.execute(script, windowOptions(windowLabel), ...args);

const waitFor = async (
  windowLabel: string,
  predicate: TauriScript,
  args: unknown[] = [],
  message = `condition was not met in ${windowLabel}`,
) => {
  await browser.waitUntil(
    async () => Boolean(await inWindow(windowLabel, predicate, ...args)),
    { timeout: 10_000, interval: 100, timeoutMsg: message },
  );
};

const waitForWindow = async (windowLabel: string, visible: boolean) => {
  await browser.waitUntil(async () => {
    const states = await inWindow('main', (tauri: any) => tauri.core.invoke('plugin:wdio|get_window_states')) as Array<{
      label: string;
      is_visible: boolean;
    }>;
    return states.some(state => state.label === windowLabel && state.is_visible === visible);
  }, {
    timeout: 10_000,
    interval: 100,
    timeoutMsg: `${windowLabel} did not become ${visible ? 'visible' : 'hidden'}`,
  });
};

const click = async (windowLabel: string, selector: string) => {
  await waitFor(windowLabel, (_tauri, target) => Boolean(document.querySelector(target as string)), [selector], `missing ${selector}`);
  await inWindow(windowLabel, (_tauri, target) => {
    const element = document.querySelector(target as string);
    if (!(element instanceof HTMLElement)) return false;
    element.click();
    return true;
  }, selector);
};

const clickButtonContaining = async (windowLabel: string, text: string) => {
  await waitFor(windowLabel, (_tauri, target) => [...document.querySelectorAll('button')].some(button => button.textContent?.includes(target as string)), [text], `missing button containing ${text}`);
  await inWindow(windowLabel, (_tauri, target) => {
    const button = [...document.querySelectorAll('button')].find(element => element.textContent?.includes(target as string));
    if (!(button instanceof HTMLElement)) return false;
    button.click();
    return true;
  }, text);
};

const setValue = async (windowLabel: string, selector: string, value: string) => {
  await waitFor(windowLabel, (_tauri, target) => {
    const element = document.querySelector(target as string);
    return element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement;
  }, [selector], `missing editable ${selector}`);
  await inWindow(windowLabel, (_tauri, target, nextValue) => {
    const element = document.querySelector(target as string);
    if (!(element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement)) return false;
    const prototype = element instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
    const setter = Object.getOwnPropertyDescriptor(prototype, 'value')?.set;
    if (!setter) return false;
    element.focus();
    setter.call(element, nextValue as string);
    element.dispatchEvent(new Event('input', { bubbles: true }));
    element.dispatchEvent(new Event('change', { bubbles: true }));
    return element.value === nextValue;
  }, selector, value);
  await waitFor(windowLabel, (_tauri, target, expected) => {
    const element = document.querySelector(target as string);
    return (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) && element.value === expected;
  }, [selector, value], `${selector} did not receive its value`);
};

// CodeMirror 的可编辑区是 contenteditable：用 execCommand 走真实输入通道，由 CM 自己的 DOM 观察器更新文档。
const setCodeContent = async (windowLabel: string, text: string) => {
  await waitFor(windowLabel, () => Boolean(document.querySelector('.cm-content')), [], 'missing code editor');
  await inWindow(windowLabel, (_tauri, nextText) => {
    const element = document.querySelector('.cm-content');
    if (!(element instanceof HTMLElement)) return false;
    element.focus();
    document.execCommand('selectAll');
    return document.execCommand('insertText', false, nextText as string);
  }, text);
  await waitFor(windowLabel, (_tauri, expected) => document.querySelector('.cm-content')?.textContent === expected, [text], 'code editor did not receive its text');
};

const selectFieldOption = async (windowLabel: string, ariaLabel: string, optionText: string) => {
  await click(windowLabel, `button[role="combobox"][aria-label="${ariaLabel}"]`);
  await waitFor(windowLabel, (_tauri, label, text) => [...document.querySelectorAll(`[role="listbox"][aria-label="${label}"] [role="option"]`)].some(option => option.textContent?.trim() === text), [ariaLabel, optionText], `missing option ${optionText}`);
  await inWindow(windowLabel, (_tauri, label, text) => {
    const option = [...document.querySelectorAll(`[role="listbox"][aria-label="${label}"] [role="option"]`)].find(item => item.textContent?.trim() === text);
    if (!(option instanceof HTMLElement)) return false;
    option.click();
    return true;
  }, ariaLabel, optionText);
};

const invokeMain = (command: string, args?: Record<string, unknown>) =>
  inWindow('main', (tauri: any, name, payload) => tauri.core.invoke(name as string, payload as Record<string, unknown>), command, args ?? {});

const hasText = async (windowLabel: string, text: string) => Boolean(await inWindow(
  windowLabel,
  (_tauri, expected) => document.body.textContent?.includes(expected as string) ?? false,
  text,
));

const searchItems = async (windowLabel: string, query: string): Promise<SearchItem[]> => {
  const response = await inWindow(windowLabel, (tauri: any, value) => tauri.core.invoke('search_snippets', {
    input: { query: value, limit: 20 },
  }), query) as { items: SearchItem[] };
  return response.items;
};

type SnippetRecord = { id: string; key: string; title: string; deletedAt: string | null };
const listSnippetRecords = async (): Promise<SnippetRecord[]> =>
  await inWindow('main', (tauri: any) => tauri.core.invoke('snippet_list')) as SnippetRecord[];

const selectedResultId = async () => String(await inWindow(
  'search',
  () => document.querySelector('[cmdk-item][data-selected="true"]')?.getAttribute('data-snippet-id') ?? '',
));

const sendSearchKey = async (
  key: string,
  modifiers: { ctrl?: boolean; meta?: boolean; composing?: boolean } = {},
) => {
  await inWindow('search', (_tauri, nextKey, ctrl, meta, composing) => {
    const input = document.querySelector('input[aria-label="检索文本片段"]');
    if (!(input instanceof HTMLInputElement)) return false;
    input.focus();
    return input.dispatchEvent(new KeyboardEvent('keydown', {
      key: nextKey as string,
      code: (nextKey as string).length === 1 ? `Key${(nextKey as string).toUpperCase()}` : nextKey as string,
      ctrlKey: Boolean(ctrl),
      metaKey: Boolean(meta),
      isComposing: Boolean(composing),
      bubbles: true,
      cancelable: true,
    }));
  }, key, modifiers.ctrl ?? false, modifiers.meta ?? false, modifiers.composing ?? false);
};

const openSearch = async () => {
  await inWindow('main', () => {
    window.dispatchEvent(new KeyboardEvent('keydown', {
      key: 'o',
      code: 'KeyO',
      altKey: true,
      bubbles: true,
      cancelable: true,
    }));
    return true;
  });
  await waitForWindow('search', true);
  await waitFor('search', (_tauri, selector) => Boolean(document.querySelector(selector as string)), [inputSelectors.search]);
};

const closeSearch = async () => {
  await sendSearchKey('Escape');
  await waitForWindow('search', false);
};

const shellComputedProbe = async () => inWindow('search', () => {
  const shell = document.querySelector('.quick-search-window') ?? document.querySelector('.quick-search-shell');
  if (!shell) return null;
  const style = getComputedStyle(shell);
  const backgroundColor = style.backgroundColor;
  const values = backgroundColor.match(/\d+(\.\d+)?/g)?.map(Number) ?? [];
  const lightness = backgroundColor.startsWith('oklab(') || backgroundColor.startsWith('oklch(')
    ? values[0] ?? -1
    : values.length >= 3
      ? (0.2126 * values[0] + 0.7152 * values[1] + 0.0722 * values[2]) / 255
      : -1;
  const grain = getComputedStyle(shell, '::after');
  return {
    theme: document.documentElement.dataset.theme,
    rootClass: document.documentElement.className,
    backgroundColor,
    backdropFilter: style.backdropFilter || 'none',
    borderRadius: style.borderTopLeftRadius,
    grainPointerEvents: grain.pointerEvents,
    grainImage: grain.backgroundImage.startsWith('url("data:image/svg+xml'),
    colorScheme: style.colorScheme,
    lightness,
  };
}) as Promise<{ theme?: string; rootClass: string; backgroundColor: string; backdropFilter: string; borderRadius: string; grainPointerEvents: string; grainImage: boolean; colorScheme: string; lightness: number } | null>;

// SPEC-20 token values (docs/design/design-tokens.md); computed colors are rgb().
const themeSurfaces = {
  light: { canvas: 'rgb(242, 238, 226)', surface: 'rgb(250, 248, 241)', accent: '#5E7056', primaryBg: 'rgb(45, 58, 49)', primaryFg: 'rgb(249, 248, 244)' },
  dark: { canvas: 'rgb(26, 25, 23)', surface: 'rgb(38, 38, 36)', accent: '#E5916F', primaryBg: 'rgb(179, 88, 56)', primaryFg: 'rgb(255, 248, 242)' },
} as const;

const managerComputedProbe = async () => inWindow('main', () => {
  const canvas = getComputedStyle(document.documentElement).getPropertyValue('--color-canvas').trim();
  const shell = document.querySelector('.main-window-shell');
  const shellStyle = shell ? getComputedStyle(shell) : null;
  const grain = shell ? getComputedStyle(shell, '::after') : null;
  const glass = [...document.querySelectorAll('*')].some(element => {
    const value = getComputedStyle(element).backdropFilter;
    return Boolean(value) && value !== 'none';
  });
  return {
    theme: document.documentElement.dataset.theme,
    canvas,
    shellBackground: shellStyle?.backgroundColor ?? '',
    grainPointerEvents: grain?.pointerEvents ?? '',
    grainOpacity: grain?.opacity ?? '',
    glass,
    hasHeaderBar: Boolean(document.querySelector('.manager-header')),
    hasSidebarFooter: Boolean(document.querySelector('.manager-sidebar-footer')),
    hasWorkspace: Boolean(document.querySelector('.manager-workspace')),
    hasSettings: Boolean(document.querySelector('.settings-page')),
    themeChoices: document.querySelectorAll('.settings-theme-option').length,
    rootClass: document.documentElement.className,
    bodyText: document.body.textContent ?? '',
  };
}) as Promise<{ theme?: string; canvas: string; shellBackground: string; grainPointerEvents: string; grainOpacity: string; glass: boolean; hasHeaderBar: boolean; hasSidebarFooter: boolean; hasWorkspace: boolean; hasSettings: boolean; themeChoices: number; rootClass: string; bodyText: string }>;

const accentProbe = async () => inWindow('main', () => {
  const root = getComputedStyle(document.documentElement);
  const add = document.querySelector('.manager-add-button');
  const primary = document.querySelector('.manager-button-primary');
  const checkedSwitch = document.querySelector('.manager-switch:checked');
  return {
    accent: root.getPropertyValue('--color-accent').trim().toUpperCase(),
    addColor: add ? getComputedStyle(add).color : '',
    primaryBackground: primary ? getComputedStyle(primary).backgroundColor : '',
    primaryColor: primary ? getComputedStyle(primary).color : '',
    primaryCount: document.querySelectorAll('.manager-button-primary, .btn-primary').length,
    switchBackground: checkedSwitch ? getComputedStyle(checkedSwitch).backgroundColor : '',
  };
}) as Promise<{ accent: string; addColor: string; primaryBackground: string; primaryColor: string; primaryCount: number; switchBackground: string }>;

const fontProbe = async (windowLabel: string) => inWindow(windowLabel, async () => {
  await Promise.all([
    document.fonts.load('500 22px "Playfair Display"'),
    document.fonts.load('400 14px "Source Sans 3"'),
  ]);
  return {
    body: getComputedStyle(document.body).fontFamily,
    heading: document.fonts.check('500 22px "Playfair Display"'),
    text: document.fonts.check('400 14px "Source Sans 3"'),
    figtree: [...document.fonts].some(face => face.family.replace(/["']/g, '') === 'Figtree'),
  };
}) as Promise<{ body: string; heading: boolean; text: boolean; figtree: boolean }>;

const setBackendTheme = async (theme: 'light' | 'dark') => {
  const current = await inWindow('main', (tauri: any) => tauri.core.invoke('settings_get')) as { revision: number };
  await inWindow('main', (tauri: any, nextTheme, revision) => tauri.core.invoke('settings_update', {
    input: { key: 'theme', value: nextTheme, revision },
  }), theme, current.revision);
  await browser.waitUntil(async () => {
    const response = await inWindow('main', (tauri: any) => tauri.core.invoke('settings_get')) as { settings: { theme: string } };
    return response.settings.theme === theme;
  }, { timeout: 5_000, interval: 100, timeoutMsg: `backend theme did not become ${theme}` });
};

const createSnippet = async (snippet: typeof snippets[number]) => {
  await click('main', 'button[aria-label="新建片段"]');
  await setValue('main', inputSelectors.key, snippet.key);
  await setValue('main', inputSelectors.title, snippet.title);
  await setCodeContent('main', snippet.content);
  if (snippet.sensitive) {
    await inWindow('main', (_tauri, labelText) => {
      const label = [...document.querySelectorAll('label')].find(element => element.textContent?.includes(labelText as string));
      if (!(label instanceof HTMLElement)) return false;
      label.click();
      return true;
    }, '敏感内容防护');
  }
  await clickButtonContaining('main', '保存片段数据');
  await waitFor('main', (_tauri, title) => document.body.textContent?.includes(title as string) ?? false, [snippet.title], `${snippet.title} was not saved`);
};

describe('compiled Searchis application', () => {
  it('keeps the quick-search visual and keyboard contract on real data', async () => {
    await waitFor('main', () => Boolean(document.querySelector('.manager-sidebar-footer')), [], 'manager sidebar footer did not render');
    const initialManager = await managerComputedProbe();
    if (initialManager.hasHeaderBar || initialManager.glass) {
      throw new Error(`SPEC-20 manager still renders HeaderBar or glass: ${JSON.stringify({ hasHeaderBar: initialManager.hasHeaderBar, glass: initialManager.glass })}`);
    }
    const fonts = await fontProbe('main');
    if (!fonts.body.includes('Source Sans 3') || !fonts.heading || !fonts.text || fonts.figtree) {
      throw new Error(`bundled fonts did not load: ${JSON.stringify(fonts)}`);
    }

    // Sidebar footer keeps the former HeaderBar behavior: shortcuts, theme toggle, settings.
    await click('main', '.manager-sidebar-footer button[aria-label="键盘快捷键速查"]');
    await waitFor('main', () => Boolean(document.querySelector('[role="dialog"][aria-labelledby="keyboard-shortcuts-title"]')), [], 'keyboard help did not open from the sidebar footer');
    await clickButtonContaining('main', '知道了');
    await waitFor('main', () => !document.querySelector('[aria-labelledby="keyboard-shortcuts-title"]'), [], 'keyboard help did not close');
    const themeBeforeToggle = await inWindow('main', () => document.documentElement.dataset.theme) as string;
    const toggledTheme = themeBeforeToggle === 'dark' ? 'light' : 'dark';
    await click('main', '.manager-sidebar-footer button[aria-label^="切换主题"]');
    await waitFor('main', (_tauri, expected) => document.documentElement.dataset.theme === expected, [toggledTheme], 'sidebar theme toggle did not switch the theme');
    await waitFor('main', (tauri: any, expected) => tauri.core.invoke('settings_get').then((response: any) => response.settings.theme === expected), [toggledTheme], 'sidebar theme toggle was not persisted');

    await click('main', '.manager-sidebar-footer button[aria-label="打开设置"]');
    await waitFor('main', () => Boolean(document.querySelector('.settings-page')), [], 'settings page did not render');
    const settingsSurface = await managerComputedProbe();
    if (!settingsSurface.hasSettings || settingsSurface.themeChoices !== 3 || !['浅色 植物', '深色 暖炭', '跟随系统'].every(label => settingsSurface.bodyText.includes(label))) {
      throw new Error(`SPEC-20 settings surface is incomplete: ${JSON.stringify({ ...settingsSurface, bodyText: undefined })}`);
    }

    await clickButtonContaining('main', '浅色 植物');
    await waitFor('main', () => document.documentElement.dataset.theme === 'light' && !document.documentElement.classList.contains('theme-transition'), [], 'manager light theme did not apply');
    await waitFor('main', (tauri: any) => tauri.core.invoke('settings_get').then((response: any) => response.settings.theme === 'light'), [], 'manager light theme was not persisted');
    const lightManager = await managerComputedProbe();
    const lightAccent = await accentProbe();
    if (lightManager.theme !== 'light' || lightManager.shellBackground !== themeSurfaces.light.canvas || lightManager.grainPointerEvents !== 'none' || lightManager.glass || lightAccent.accent !== themeSurfaces.light.accent || !lightAccent.switchBackground) {
      throw new Error(`manager light theme/layout/accent mismatch: ${JSON.stringify({ ...lightManager, bodyText: undefined, lightAccent })}`);
    }

    await clickButtonContaining('main', '深色 暖炭');
    await waitFor('main', () => document.documentElement.dataset.theme === 'dark' && !document.documentElement.classList.contains('theme-transition'), [], 'manager dark theme did not apply');
    await waitFor('main', (tauri: any) => tauri.core.invoke('settings_get').then((response: any) => response.settings.theme === 'dark'), [], 'manager dark theme was not persisted');
    const darkManager = await managerComputedProbe();
    const darkAccent = await accentProbe();
    if (darkManager.theme !== 'dark' || darkManager.shellBackground !== themeSurfaces.dark.canvas || darkManager.grainPointerEvents !== 'none' || darkManager.glass || darkAccent.accent !== themeSurfaces.dark.accent) {
      throw new Error(`manager dark theme/layout mismatch: ${JSON.stringify({ ...darkManager, bodyText: undefined, darkAccent })}`);
    }
    if (darkManager.grainOpacity === lightManager.grainOpacity) {
      throw new Error(`grain opacity must differ per theme: ${JSON.stringify([lightManager.grainOpacity, darkManager.grainOpacity])}`);
    }

    await clickButtonContaining('main', '跟随系统');
    const systemTheme = await inWindow('main', () => window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light') as string;
    await waitFor('main', (_tauri, expectedTheme) => document.documentElement.dataset.theme === expectedTheme && !document.documentElement.classList.contains('theme-transition'), [systemTheme], 'manager system theme did not follow the OS preference');
    await waitFor('main', (tauri: any) => tauri.core.invoke('settings_get').then((response: any) => response.settings.theme === 'system'), [], 'manager system theme was not persisted');

    await clickButtonContaining('main', '浅色 植物');
    await waitFor('main', () => document.documentElement.dataset.theme === 'light' && !document.documentElement.classList.contains('theme-transition'), [], 'manager did not return to light theme');
    await click('main', 'button[aria-label="返回片段管理"]');
    await waitFor('main', () => Boolean(document.querySelector('.manager-workspace')), [], 'manager workspace did not render');
    const managerAccent = await accentProbe();
    if (managerAccent.addColor !== 'rgb(94, 112, 86)' || managerAccent.primaryCount > 1 || (managerAccent.primaryCount === 1 && (managerAccent.primaryBackground !== themeSurfaces.light.primaryBg || managerAccent.primaryColor !== themeSurfaces.light.primaryFg))) {
      throw new Error(`manager light actions did not render with the shared tokens: ${JSON.stringify(managerAccent)}`);
    }

    // F2 regression: the reduced-motion rules must beat .theme-transition while it is applied.
    // When the desktop already prefers reduced motion the real cascade is measured; otherwise
    // (WebKitGTK cannot emulate the media query) the @media body is applied unconditionally.
    const reducedMotion = await inWindow('main', () => {
      const reduceRules: string[] = [];
      for (const sheet of [...document.styleSheets]) {
        for (const rule of [...sheet.cssRules]) {
          if (rule instanceof CSSMediaRule && rule.media.mediaText.includes('prefers-reduced-motion: reduce')) {
            reduceRules.push(...[...rule.cssRules].map(inner => inner.cssText));
          }
        }
      }
      const targets = ['.manager-snippet-row', '.manager-add-button', '.manager-search-field', '.manager-sidebar-footer button']
        .map(selector => document.querySelector(selector))
        .filter((element): element is Element => Boolean(element));
      const root = document.documentElement;
      const hadClass = root.classList.contains('theme-transition');
      root.classList.add('theme-transition');
      const style = document.createElement('style');
      style.textContent = reduceRules.join('\n');
      const durations: number[] = [];
      for (const applyReduce of [false, true]) {
        if (applyReduce) document.head.append(style);
        durations.push(Math.max(...targets.flatMap(element => getComputedStyle(element).transitionDuration.split(',')
          .map(value => parseFloat(value) * (value.trim().endsWith('ms') ? 1 : 1000)))));
      }
      style.remove();
      if (!hadClass) root.classList.remove('theme-transition');
      const nativeReduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
      return { nativeReduce, ruleCount: reduceRules.length, targets: targets.length, withoutReduce: durations[0], withReduce: durations[1] };
    }) as { nativeReduce: boolean; ruleCount: number; targets: number; withoutReduce: number; withReduce: number };
    const reduceControlOk = reducedMotion.nativeReduce ? reducedMotion.withoutReduce <= 0.01 : reducedMotion.withoutReduce >= 200;
    if (reducedMotion.ruleCount === 0 || reducedMotion.targets < 3 || !reduceControlOk || reducedMotion.withReduce > 0.01) {
      throw new Error(`reduced-motion does not override the theme transition: ${JSON.stringify(reducedMotion)}`);
    }

    // F3 regression: placeholders use a token pair that meets WCAG AA on the filled field in both themes.
    const placeholderContrast = await inWindow('main', () => {
      const placeholderColors: string[] = [];
      for (const sheet of [...document.styleSheets]) {
        for (const rule of [...sheet.cssRules]) {
          if (rule instanceof CSSStyleRule && rule.selectorText.includes('::placeholder')) placeholderColors.push(rule.style.color);
        }
      }
      // WCAG 2.x relative luminance contrast, computed inline (page scripts lack esbuild's __name).
      const root = document.documentElement;
      const original = root.dataset.theme;
      const ratios: Record<string, number> = {};
      for (const theme of ['light', 'dark']) {
        root.dataset.theme = theme;
        const tokens = getComputedStyle(root);
        for (const background of ['--color-field', '--color-surface']) {
          const [high, low] = [tokens.getPropertyValue('--color-fg-secondary'), tokens.getPropertyValue(background)]
            .map(hex => hex.trim())
            .map(hex => [1, 3, 5].map(index => parseInt(hex.slice(index, index + 2), 16) / 255)
              .map(channel => channel <= 0.03928 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4))
            .map(([r, g, b]) => 0.2126 * r + 0.7152 * g + 0.0722 * b)
            .sort((x, y) => y - x);
          ratios[`${theme}/${background}`] = (high + 0.05) / (low + 0.05);
        }
      }
      if (original) root.dataset.theme = original;
      return { placeholderColors, ratios };
    }) as { placeholderColors: string[]; ratios: Record<string, number> };
    if (placeholderContrast.placeholderColors.length < 3
      || placeholderContrast.placeholderColors.some(color => color !== 'var(--color-fg-secondary)')
      || Object.values(placeholderContrast.ratios).some(ratio => !(ratio >= 4.5))) {
      throw new Error(`placeholder contrast is below WCAG AA: ${JSON.stringify(placeholderContrast)}`);
    }

    // RF8 regression: a mouse option selection must close the themed listbox.
    const sortTrigger = 'button[role="combobox"][aria-label="排序方式"]';
    const sortListbox = '[role="listbox"][aria-label="排序方式"]';
    await click('main', sortTrigger);
    await waitFor('main', (_tauri, selector) => Boolean(document.querySelector(selector as string)), [sortListbox], 'manager sort listbox did not open');
    await inWindow('main', (_tauri, selector) => {
      const option = [...document.querySelectorAll(`${selector as string} [role="option"]`)].find(item => item.getAttribute('aria-selected') !== 'true');
      if (!(option instanceof HTMLElement)) return false;
      option.click();
      return true;
    }, sortListbox);
    await waitFor('main', (_tauri, selector) => !document.querySelector(selector as string), [sortListbox], 'manager sort listbox stayed open after mouse selection');
    const sortSelection = await inWindow('main', (_tauri, selector) => {
      const trigger = document.querySelector(selector as string);
      return trigger instanceof HTMLButtonElement && trigger.getAttribute('aria-expanded') === 'false' && document.activeElement === trigger;
    }, sortTrigger);
    if (!sortSelection) throw new Error('manager sort trigger did not retain focus after mouse selection');

    await setBackendTheme('light');
    await openSearch();
    const searchWindow = (await exec('xdotool', ['search', '--name', 'Searchis 快速检索'])).stdout.trim().split('\n').at(-1);
    if (!searchWindow) throw new Error('X11 search window was not found');
    await exec('xdotool', ['windowactivate', '--sync', searchWindow]);
    const lightDiagnostic = await inWindow('search', async (tauri: any) => ({
      backend: (await tauri.core.invoke('settings_get')).settings.theme,
      dom: document.documentElement.dataset.theme,
    })) as { backend: string; dom?: string };
    if (lightDiagnostic.backend !== 'light') throw new Error(`search window backend theme mismatch: ${JSON.stringify(lightDiagnostic)}`);
    await waitFor('search', () => document.documentElement.dataset.theme === 'light' && !document.documentElement.classList.contains('theme-transition'), [], `search window did not load the backend theme: ${JSON.stringify(lightDiagnostic)}`);
    const dimensions = await inWindow('search', () => {
      const shell = document.querySelector('.quick-search-shell');
      const panel = document.querySelector('.quick-search-window');
      return {
        viewport: [window.innerWidth, window.innerHeight],
        shell: shell ? [shell.clientWidth, shell.clientHeight] : [],
        panel: panel ? [panel.clientWidth, panel.clientHeight] : [],
      };
    }) as { viewport: number[]; shell: number[]; panel: number[] };
    if (dimensions.shell[0] !== dimensions.viewport[0] || dimensions.shell[1] !== dimensions.viewport[1] || dimensions.panel[0] < dimensions.viewport[0] - 2 || dimensions.panel[1] < dimensions.viewport[1] - 2) {
      throw new Error(`search UI does not fill its viewport: ${JSON.stringify(dimensions)}`);
    }
    const focusStyle = await inWindow('search', () => {
      const input = document.querySelector('input[cmdk-input]');
      const field = input?.closest('.quick-search-field');
      if (!(input instanceof HTMLInputElement) || !field) return null;
      input.focus();
      // Resolve tokens to rgb() via a probe element (no named helpers: page scripts lack esbuild's __name).
      const resolved: Record<string, string> = {};
      for (const token of ['--color-field', '--color-focus-ring']) {
        const probe = document.createElement('span');
        probe.style.color = `var(${token})`;
        document.body.append(probe);
        resolved[token] = getComputedStyle(probe).color;
        probe.remove();
      }
      const inputStyle = getComputedStyle(input);
      const fieldStyle = getComputedStyle(field);
      return {
        focused: document.activeElement === input && field.matches(':focus-within'),
        inputBorderWidth: inputStyle.borderBottomWidth,
        inputOutline: inputStyle.outlineStyle,
        fieldBackground: fieldStyle.backgroundColor,
        fieldToken: resolved['--color-field'],
        ringStyle: fieldStyle.outlineStyle,
        ringWidth: fieldStyle.outlineWidth,
        ringOffset: fieldStyle.outlineOffset,
        ringColor: fieldStyle.outlineColor,
        ringToken: resolved['--color-focus-ring'],
        devicePixelRatio: window.devicePixelRatio,
      };
    }) as { devicePixelRatio: number; focused: boolean; inputBorderWidth: string; inputOutline: string; fieldBackground: string; fieldToken: string; ringStyle: string; ringWidth: string; ringOffset: string; ringColor: string; ringToken: string } | null;
    // SPEC-20: filled borderless input, 2px focus-ring with offset (drawn on the field so it is not doubled).
    // WebKit floors outline widths to whole device pixels, e.g. 2px reads back as 1.6px at DPR 1.25.
    const snappedRingWidth = focusStyle ? Math.max(1, Math.floor(2 * focusStyle.devicePixelRatio)) / focusStyle.devicePixelRatio : 0;
    if (!focusStyle || !focusStyle.focused || focusStyle.inputBorderWidth !== '0px' || focusStyle.inputOutline !== 'none'
      || focusStyle.fieldBackground !== focusStyle.fieldToken || focusStyle.ringStyle !== 'solid'
      || Math.abs(parseFloat(focusStyle.ringWidth) - snappedRingWidth) > 0.01
      || focusStyle.ringOffset !== '2px' || focusStyle.ringColor !== focusStyle.ringToken) {
      throw new Error(`search input focus ring does not match SPEC-20: ${JSON.stringify(focusStyle)}`);
    }
    const mainWindow = (await exec('xdotool', ['search', '--name', '^Searchis$'])).stdout.trim().split('\n').at(-1);
    if (!mainWindow) throw new Error('X11 main window was not found');
    await exec('xdotool', ['windowactivate', '--sync', mainWindow]);
    await waitForWindow('search', false);

    // Regression: input written immediately after the picker becomes visible must
    // survive delayed focus/visibility notifications.
    await openSearch();
    await setValue('search', inputSelectors.search, 'h');
    await new Promise(resolve => setTimeout(resolve, 350));

    const firstCharacter = await inWindow(
      'search',
      (_tauri, selector) => {
        const input = document.querySelector(selector as string);
        return input instanceof HTMLInputElement ? input.value : null;
      },
      inputSelectors.search,
    );

    if (firstCharacter !== 'h') {
      throw new Error(
        `search input lost the first character after activation: ${JSON.stringify(firstCharacter)}`,
      );
    }

    await closeSearch();

    await setBackendTheme('dark');
    await openSearch();
    await exec('xdotool', ['windowactivate', '--sync', searchWindow]);
    await waitFor('search', () => document.documentElement.dataset.theme === 'dark' && !document.documentElement.classList.contains('theme-transition'), [], 'search window did not refresh the backend theme');
    const darkProbe = await shellComputedProbe();
    if (!darkProbe || darkProbe.backgroundColor !== themeSurfaces.dark.surface || darkProbe.backdropFilter !== 'none' || darkProbe.borderRadius !== '26px' || darkProbe.grainPointerEvents !== 'none' || !darkProbe.grainImage || darkProbe.lightness < 0 || darkProbe.lightness > 0.35) {
      throw new Error(`search shell is not visually dark: ${JSON.stringify(darkProbe)}`);
    }
    await closeSearch();

    await setBackendTheme('light');
    await openSearch();
    await exec('xdotool', ['windowactivate', '--sync', searchWindow]);
    await waitFor('search', () => document.documentElement.dataset.theme === 'light' && !document.documentElement.classList.contains('theme-transition'), [], 'search window did not switch back to light');
    const lightProbe = await shellComputedProbe();
    if (!lightProbe || lightProbe.backgroundColor !== themeSurfaces.light.surface || lightProbe.backdropFilter !== 'none' || lightProbe.borderRadius !== '26px' || lightProbe.grainPointerEvents !== 'none' || !lightProbe.grainImage || lightProbe.lightness < 0.8) {
      throw new Error(`search shell is not visually light: ${JSON.stringify(lightProbe)}`);
    }
    await closeSearch();

    for (const snippet of snippets) await createSnippet(snippet);

    const initialItems = await searchItems('main', snippets[0].key);
    if (initialItems[0]?.usageCount !== 0) throw new Error('new snippet must start with usageCount=0');

    await openSearch();
    await setValue('search', inputSelectors.search, snippets[0].key);
    await waitFor('search', (_tauri, title) => document.body.textContent?.includes(title as string) ?? false, [snippets[0].title], 'search result did not appear');
    await closeSearch();

    // Real trash lifecycle: delete from the manager, restore from the trash view.
    const recycleTarget = snippets[1];
    await clickButtonContaining('main', recycleTarget.title);
    await click('main', 'button[aria-label="放入回收站"]');
    await browser.waitUntil(async () => (await listSnippetRecords()).find(item => item.key === recycleTarget.key)?.deletedAt !== null, { timeout: 5_000, interval: 100, timeoutMsg: 'delete IPC did not mark the snippet as trashed' });
    await clickButtonContaining('main', '回收站');
    await waitFor('main', (_tauri, title) => [...document.querySelectorAll('.manager-snippet-row')].some(row => row.textContent?.includes(title as string)), [recycleTarget.title], 'deleted snippet did not appear in trash');
    await clickButtonContaining('main', recycleTarget.title);
    await clickButtonContaining('main', '恢复');
    await browser.waitUntil(async () => (await listSnippetRecords()).find(item => item.key === recycleTarget.key)?.deletedAt === null, { timeout: 5_000, interval: 100, timeoutMsg: 'restore IPC did not clear deletedAt' });
    await clickButtonContaining('main', '全部片段');
    await waitFor('main', (_tauri, title) => [...document.querySelectorAll('.manager-snippet-row')].some(row => row.textContent?.includes(title as string)), [recycleTarget.title], 'restored snippet did not return to all snippets');

    // Navigation is asserted by item identity, including both J/K directions.
    await openSearch();
    await setValue('search', inputSelectors.search, 'e2e-');
    await waitFor('search', () => document.querySelectorAll('[cmdk-item]').length >= 3, [], 'grouped search results did not appear');
    const navigationItems = await searchItems('main', 'e2e-');
    const firstSelected = await selectedResultId();
    await sendSearchKey('ArrowDown');
    await waitFor('search', (_tauri, previous) => document.querySelector('[cmdk-item][data-selected="true"]')?.getAttribute('data-snippet-id') !== previous, [firstSelected], 'ArrowDown did not move selection');
    const afterArrowDown = await selectedResultId();
    await sendSearchKey('k', { ctrl: true });
    await waitFor('search', (_tauri, previous) => document.querySelector('[cmdk-item][data-selected="true"]')?.getAttribute('data-snippet-id') !== previous, [afterArrowDown], 'Ctrl+K did not move selection');
    const afterK = await selectedResultId();
    await sendSearchKey('j', { ctrl: true });
    await waitFor('search', (_tauri, previous) => document.querySelector('[cmdk-item][data-selected="true"]')?.getAttribute('data-snippet-id') !== previous, [afterK], 'Ctrl+J did not move selection');

    // IME composition must not trigger Enter or change usage.
    const beforeIme = await selectedResultId();
    const imeItem = navigationItems.find(item => item.id === beforeIme);
    if (!imeItem) throw new Error('selected navigation item was not returned by the backend');
    await sendSearchKey('Enter', { composing: true });
    await waitForWindow('search', true);
    if (await selectedResultId() !== beforeIme) throw new Error('IME Enter changed selection');
    if (((await searchItems('main', imeItem.key)).find(item => item.id === imeItem.id)?.usageCount) !== 0) throw new Error('IME Enter changed usageCount');

    // Enter is copy-only: the picker closes and usage metadata updates.
    await sendSearchKey('Enter');
    await waitForWindow('search', false);
    const copied = (await searchItems('main', imeItem.key)).find(item => item.id === imeItem.id);
    if (copied?.usageCount !== 1) throw new Error(`Enter usageCount expected 1, got ${copied?.usageCount}`);

    // Ctrl+1 requests auto-paste and closes the picker. The backend's Rust tests
    // cover clipboard contents; this UI check covers the real command result/count.
    await openSearch();
    await setValue('search', inputSelectors.search, snippets[0].key);
    await waitFor('search', (_tauri, title) => document.body.textContent?.includes(title as string) ?? false, [snippets[0].title]);
    const beforePaste = (await searchItems('main', snippets[0].key)).find(item => item.key === snippets[0].key)?.usageCount ?? 0;
    await sendSearchKey('1', { ctrl: true });
    await waitForWindow('search', false);
    const pasted = (await searchItems('main', snippets[0].key)).find(item => item.key === snippets[0].key);
    if (pasted?.usageCount !== beforePaste + 1) throw new Error(`Ctrl+1 usageCount expected ${beforePaste + 1}, got ${pasted?.usageCount}`);

    // Ctrl+E returns to the manager with the selected real snippet.
    await openSearch();
    await setValue('search', inputSelectors.search, snippets[0].key);
    await waitFor('search', (_tauri, title) => document.body.textContent?.includes(title as string) ?? false, [snippets[0].title]);
    await sendSearchKey('e', { ctrl: true });
    await waitForWindow('search', false);
    await waitFor('main', (_tauri, selector, expected) => {
      const element = document.querySelector(selector as string);
      return element instanceof HTMLInputElement && element.value === expected;
    }, [inputSelectors.key, snippets[0].key], 'Ctrl+E did not open the selected snippet');

    // Ctrl+N normalizes the query and opens a new manager form.
    await openSearch();
    await setValue('search', inputSelectors.search, 'new snippet');
    await sendSearchKey('n', { ctrl: true });
    await waitForWindow('search', false);
    await waitFor('main', (_tauri, selector, expected) => {
      const element = document.querySelector(selector as string);
      return element instanceof HTMLInputElement && element.value === expected;
    }, [inputSelectors.key, 'new-snippet'], 'Ctrl+N did not prefill the normalized key');

    // A real IPC command validation failure stays in the picker and is visible.
    await openSearch();
    await setValue('search', inputSelectors.search, 'bad/key!');
    await sendSearchKey('n', { ctrl: true });
    await waitFor('search', (_tauri) => Boolean(document.querySelector('[role="alert"]')?.textContent?.includes('Key')), [], 'IPC validation failure was not shown');
    await waitForWindow('search', true);
    await closeSearch();

    // Sensitive content is never rendered by the search DTO.
    await openSearch();
    await setValue('search', inputSelectors.search, snippets[2].key);
    await waitFor('search', (_tauri, title) => document.body.textContent?.includes(title as string) ?? false, [snippets[2].title]);
    if (await hasText('search', snippets[2].content)) throw new Error('sensitive content leaked into quick-search DOM');
    await closeSearch();

    // Permanent deletion requires an explicit native confirmation and removes the real row.
    await clickButtonContaining('main', snippets[1].title);
    await click('main', 'button[aria-label="放入回收站"]');
    await clickButtonContaining('main', '回收站');
    await clickButtonContaining('main', snippets[1].title);
    await clickButtonContaining('main', '彻底删除');
    await waitFor('main', () => Boolean(document.querySelector('[role="alertdialog"]')), [], 'permanent deletion confirmation did not open');
    await clickButtonContaining('main', '确认永久删除');
    await browser.waitUntil(async () => !(await listSnippetRecords()).some(item => item.key === snippets[1].key), { timeout: 5_000, interval: 100, timeoutMsg: 'permanently deleted snippet remained in the real list' });

    // RF3 lifecycle check is last: after the native window hides, no search-context calls follow.
    await openSearch();
    await exec('xdotool', ['windowactivate', '--sync', mainWindow]);
    await waitForWindow('search', false);
  });

  it('manages the group tree, language badges and non-empty group deletion (SPEC-21)', async () => {
    const groupSuffix = suffix.slice(-6);
    const root = `运维${groupSuffix}`;
    const db = `数据库${groupSuffix}`;
    const slow = `慢查询${groupSuffix}`;
    const bashKey = `ngxr-${suffix}`;
    const bashContent = `sudo nginx -s reload # ${suffix}`;

    await clickButtonContaining('main', '全部片段');
    const createGroup = async (name: string, parentAriaLabel?: string) => {
      await click('main', parentAriaLabel ? `button[aria-label="${parentAriaLabel}"]` : 'button[aria-label="新建分组"]');
      await setValue('main', 'input[aria-label="分组名称"]', name);
      await clickButtonContaining('main', '创建');
      await waitFor('main', (_tauri, text) => [...document.querySelectorAll('.manager-group-main')].some(node => node.textContent?.includes(text as string)), [name], `${name} did not appear in the tree`);
    };

    // AC-21 数据部分：三层允许，第四层被拒绝；UI 在第三层不再提供“新建子分组”。
    await createGroup(root);
    await createGroup(db, `在 ${root} 下新建子分组`);
    await createGroup(slow, `在 ${db} 下新建子分组`);
    if (await inWindow('main', (_tauri, label) => Boolean(document.querySelector(`button[aria-label="${label}"]`)), `在 ${slow} 下新建子分组`)) {
      throw new Error('third-level group must not offer a child-group button');
    }
    const groups = await invokeMain('group_list') as Array<{ id: string; name: string; depth: number; parentId: string | null; revision: number }>;
    const slowGroup = groups.find(group => group.name === slow);
    const rootGroup = groups.find(group => group.name === root);
    const dbGroup = groups.find(group => group.name === db);
    if (!slowGroup || !rootGroup || !dbGroup || slowGroup.depth !== 3) throw new Error(`unexpected group tree: ${JSON.stringify(groups)}`);
    const tooDeep = await inWindow('main', async (tauri: any, parentId, requestId) => {
      try {
        await tauri.core.invoke('group_create', { input: { name: 'too-deep', parentId, requestId } });
        return 'created';
      } catch (error) {
        return (error as { code?: string }).code ?? 'unknown';
      }
    }, slowGroup.id, crypto.randomUUID());
    if (tooDeep !== 'GROUP_DEPTH_EXCEEDED') throw new Error(`expected GROUP_DEPTH_EXCEEDED, got ${tooDeep}`);

    // 新建 bash 片段并放进分组。
    await click('main', 'button[aria-label="新建片段"]');
    await setValue('main', inputSelectors.key, bashKey);
    await setValue('main', inputSelectors.title, `Nginx 重载 ${groupSuffix}`);
    await setCodeContent('main', bashContent);
    await selectFieldOption('main', '分组', root);
    await selectFieldOption('main', '语言', 'Bash');
    await clickButtonContaining('main', '保存片段数据');
    await browser.waitUntil(async () => {
      const list = await listSnippetRecords() as Array<SnippetRecord & { groupId?: string | null; language?: string; content?: string; kind?: string }>;
      const record = list.find(item => item.key === bashKey);
      return record?.groupId === rootGroup.id && record.language === 'bash' && record.content === bashContent && record.kind === 'text';
    }, { timeout: 5_000, interval: 100, timeoutMsg: 'snippet did not persist its group, language and exact content' });

    // 管理列表：BASH 徽标。
    await waitFor('main', (_tauri, key) => [...document.querySelectorAll('.manager-snippet-row')].some(row => row.textContent?.includes(key as string) && row.querySelector('[data-testid="language-badge"]')?.textContent === 'BASH'), [bashKey], 'manager row did not show the BASH badge');

    // 快搜：徽标 + 副标题 Key · 分组 · N 次；分组名不参与检索。
    await openSearch();
    await setValue('search', inputSelectors.search, bashKey);
    await waitFor('search', (_tauri, key, group) => {
      const row = [...document.querySelectorAll('[cmdk-item]')].find(item => item.textContent?.includes(key as string));
      return Boolean(row?.textContent?.includes(group as string) && row.querySelector('[data-testid="language-badge"]')?.textContent === 'BASH' && /0 次/.test(row.textContent ?? ''));
    }, [bashKey, root], 'quick-search row did not show badge, group name and usage');
    await closeSearch();
    if ((await searchItems('main', root)).length !== 0) throw new Error('group name must not be searchable');
    const hit = (await searchItems('main', bashKey))[0];
    if (hit?.language !== 'bash' || hit.groupName !== root || hit.kind !== 'text') throw new Error(`unexpected search DTO: ${JSON.stringify(hit)}`);

    // AC-22 数据部分：取消不变，确认后内容上移。
    await click('main', `button[aria-label="删除分组 ${root}"]`);
    await waitFor('main', () => Boolean(document.querySelector('[role="alertdialog"]')), [], 'group delete confirmation did not open');
    await clickButtonContaining('main', '取消');
    if (!((await invokeMain('group_list')) as Array<{ name: string }>).some(group => group.name === root)) throw new Error('cancel must keep the group');

    await click('main', `button[aria-label="删除分组 ${root}"]`);
    await waitFor('main', () => Boolean(document.querySelector('[role="alertdialog"]')), [], 'group delete confirmation did not reopen');
    await clickButtonContaining('main', '确认删除分组');
    await browser.waitUntil(async () => {
      const after = await invokeMain('group_list') as Array<{ id: string; name: string; depth: number; parentId: string | null }>;
      const moved = after.find(group => group.id === dbGroup.id);
      const deeper = after.find(group => group.id === slowGroup.id);
      return !after.some(group => group.name === root) && moved?.parentId === null && moved.depth === 1 && deeper?.depth === 2;
    }, { timeout: 5_000, interval: 100, timeoutMsg: 'children were not promoted after deleting the group' });
    const afterDelete = (await listSnippetRecords() as Array<SnippetRecord & { groupId?: string | null }>).find(item => item.key === bashKey);
    if (afterDelete?.groupId !== null) throw new Error('snippet must be ungrouped after its group was deleted');
  });
  it('keeps editor history per snippet and preserves original line endings (SPEC-21 repair R1/R2)', async () => {
    const keyA = `undo-a-${suffix}`;
    const keyB = `undo-b-${suffix}`;
    const keyCrlf = `crlf-${suffix}`;
    const crlfBody = 'line1\r\nline2\r\n';
    for (const [key, content] of [[keyA, 'AAA'], [keyB, 'BBB'], [keyCrlf, crlfBody]]) {
      await invokeMain('snippet_create', { input: { key, title: key, content, aliases: [], tags: [], pinned: false, sensitive: false, groupId: null, language: 'plain', requestId: crypto.randomUUID() } });
    }
    await inWindow('main', () => { location.reload(); });
    const selectRow = async (key: string, text: string) => {
      await waitFor('main', (_t, k) => [...document.querySelectorAll('.manager-snippet-row')].some(e => e.textContent?.includes(k as string)), [key], `row ${key} missing`);
      await inWindow('main', (_t, k) => ([...document.querySelectorAll('.manager-snippet-row')].find(e => e.textContent?.includes(k as string)) as HTMLElement).click(), key);
      await waitFor('main', (_t, expected) => document.querySelector('.cm-content')?.textContent === expected, [text], `editor did not show ${key}`);
    };

    // R2：A → B 后在 B 中 Ctrl+Z 不得带回 A 的正文。
    await selectRow(keyA, 'AAA');
    await selectRow(keyB, 'BBB');
    await inWindow('main', () => {
      const editor = document.querySelector('.cm-content') as HTMLElement;
      editor.focus();
      editor.dispatchEvent(new KeyboardEvent('keydown', { key: 'z', code: 'KeyZ', ctrlKey: true, bubbles: true, cancelable: true }));
    });
    await browser.pause(300);
    if (await inWindow('main', () => document.querySelector('.cm-content')?.textContent) !== 'BBB') {
      throw new Error('Ctrl+Z restored another snippet body');
    }

    // R1：打开 CRLF 正文后不编辑直接保存，正文字节不变。
    await selectRow(keyCrlf, 'line1line2');
    await clickButtonContaining('main', '保存片段数据');
    await browser.pause(500);
    const saved = (await listSnippetRecords()).find(item => item.key === keyCrlf);
    if ((saved as { content?: string } | undefined)?.content !== crlfBody) {
      throw new Error(`CRLF body changed on save: ${JSON.stringify((saved as { content?: string } | undefined)?.content)}`);
    }
  });
});
