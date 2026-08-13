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
  content: 'textarea[placeholder="在此输入需要快速粘贴的任意文本片段..."]',
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
  await clickButtonContaining('main', '呼出快速窗口');
  await waitForWindow('search', true);
  await waitFor('search', (_tauri, selector) => Boolean(document.querySelector(selector as string)), [inputSelectors.search]);
};

const closeSearch = async () => {
  await sendSearchKey('Escape');
  await waitForWindow('search', false);
};

const shellComputedProbe = async () => inWindow('search', () => {
  const shell = document.querySelector('.quick-search-shell');
  if (!shell) return null;
  const style = getComputedStyle(shell);
  const backgroundColor = style.backgroundColor;
  const values = backgroundColor.match(/\d+(\.\d+)?/g)?.map(Number) ?? [];
  const lightness = backgroundColor.startsWith('oklab(') || backgroundColor.startsWith('oklch(')
    ? values[0] ?? -1
    : values.length >= 3
      ? (0.2126 * values[0] + 0.7152 * values[1] + 0.0722 * values[2]) / 255
      : -1;
  return {
    theme: document.documentElement.dataset.theme,
    rootClass: document.documentElement.className,
    backgroundColor,
    quickCanvas: style.getPropertyValue('--quick-canvas').trim(),
    colorScheme: style.colorScheme,
    lightness,
  };
}) as Promise<{ theme?: string; rootClass: string; backgroundColor: string; quickCanvas: string; colorScheme: string; lightness: number } | null>;

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
  await setValue('main', inputSelectors.content, snippet.content);
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
      if (!(input instanceof HTMLInputElement)) return null;
      input.focus();
      const style = getComputedStyle(input);
      return {
        outlineStyle: style.outlineStyle,
        boxShadow: style.boxShadow,
        borderBottomStyle: style.borderBottomStyle,
        borderBottomWidth: style.borderBottomWidth,
      };
    }) as { outlineStyle: string; boxShadow: string; borderBottomStyle: string; borderBottomWidth: string } | null;
    if (!focusStyle || focusStyle.outlineStyle !== 'none' || focusStyle.boxShadow !== 'none' || focusStyle.borderBottomWidth !== '0px') {
      throw new Error(`search input has an unexpected focus border: ${JSON.stringify(focusStyle)}`);
    }
    const mainWindow = (await exec('xdotool', ['search', '--name', '^Searchis$'])).stdout.trim().split('\n').at(-1);
    if (!mainWindow) throw new Error('X11 main window was not found');
    await exec('xdotool', ['windowactivate', '--sync', mainWindow]);
    await waitForWindow('search', false);

    await setBackendTheme('dark');
    await openSearch();
    await exec('xdotool', ['windowactivate', '--sync', searchWindow]);
    await waitFor('search', () => document.documentElement.dataset.theme === 'dark' && !document.documentElement.classList.contains('theme-transition'), [], 'search window did not refresh the backend theme');
    const darkProbe = await shellComputedProbe();
    if (!darkProbe || darkProbe.lightness < 0 || darkProbe.lightness > 0.35) {
      throw new Error(`search shell is not visually dark: ${JSON.stringify(darkProbe)}`);
    }
    await closeSearch();

    await setBackendTheme('light');
    await openSearch();
    await exec('xdotool', ['windowactivate', '--sync', searchWindow]);
    await waitFor('search', () => document.documentElement.dataset.theme === 'light' && !document.documentElement.classList.contains('theme-transition'), [], 'search window did not switch back to light');
    const lightProbe = await shellComputedProbe();
    if (!lightProbe || lightProbe.lightness < 0.8) {
      throw new Error(`search shell is not visually light: ${JSON.stringify(lightProbe)}`);
    }
    await closeSearch();

    for (const snippet of snippets) await createSnippet(snippet);

    const initialItems = await searchItems('main', snippets[0].key);
    if (initialItems[0]?.usageCount !== 0) throw new Error('new snippet must start with usageCount=0');

    await openSearch();
    await setValue('search', inputSelectors.search, snippets[0].key);
    await waitFor('search', (_tauri, title) => document.body.textContent?.includes(title as string) ?? false, [snippets[0].title], 'search result did not appear');
    await sendSearchKey('Tab');
    await waitFor('search', () => Boolean(document.querySelector('.quick-preview')), [], 'preview did not open');
    await sendSearchKey('Tab');
    await waitFor('search', () => !document.querySelector('.quick-preview'), [], 'preview did not close');
    await closeSearch();

    // Navigation is asserted by item identity, including both J/K directions.
    await openSearch();
    await setValue('search', inputSelectors.search, 'e2e-');
    await waitFor('search', () => document.querySelectorAll('[cmdk-item]').length >= 3, [], 'grouped search results did not appear');
    const navigationItems = await searchItems('main', 'e2e-');
    const firstSelected = await selectedResultId();
    await sendSearchKey('ArrowDown');
    await waitFor('search', (_tauri, previous) => document.querySelector('[cmdk-item][data-selected="true"]')?.getAttribute('data-snippet-id') !== previous, [firstSelected], 'ArrowDown did not move selection');
    const afterArrowDown = await selectedResultId();
    await sendSearchKey('k');
    await waitFor('search', (_tauri, previous) => document.querySelector('[cmdk-item][data-selected="true"]')?.getAttribute('data-value') !== previous, [afterArrowDown], 'K did not move selection');
    const afterK = await selectedResultId();
    await sendSearchKey('j');
    await waitFor('search', (_tauri, previous) => document.querySelector('[cmdk-item][data-selected="true"]')?.getAttribute('data-value') !== previous, [afterK], 'J did not move selection');

    // IME composition must not trigger Enter or change usage.
    const beforeIme = await selectedResultId();
    const imeItem = navigationItems.find(item => item.id === beforeIme);
    if (!imeItem) throw new Error('selected navigation item was not returned by the backend');
    await sendSearchKey('Enter', { composing: true });
    await waitForWindow('search', true);
    if (await selectedResultId() !== beforeIme) throw new Error('IME Enter changed selection');
    if (((await searchItems('main', imeItem.key)).find(item => item.id === imeItem.id)?.usageCount) !== 0) throw new Error('IME Enter changed usageCount');

    // Ctrl+Enter is copy-only: the picker remains open and usage metadata updates.
    await sendSearchKey('Enter', { ctrl: true });
    await waitForWindow('search', true);
    await waitFor('search', () => document.body.textContent?.includes('使用 1 次') ?? false, [], 'Ctrl+Enter did not update usage metadata');
    const copied = (await searchItems('main', imeItem.key)).find(item => item.id === imeItem.id);
    if (copied?.usageCount !== 1) throw new Error(`Ctrl+Enter usageCount expected 1, got ${copied?.usageCount}`);
    await closeSearch();

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

    // Sensitive content is never rendered by the search DTO or preview.
    await openSearch();
    await setValue('search', inputSelectors.search, snippets[2].key);
    await waitFor('search', (_tauri, title) => document.body.textContent?.includes(title as string) ?? false, [snippets[2].title]);
    if (await hasText('search', snippets[2].content)) throw new Error('sensitive content leaked into quick-search DOM');
    await sendSearchKey('Tab');
    await waitFor('search', () => Boolean(document.querySelector('.quick-preview')), [], 'sensitive preview did not open');
    if (await hasText('search', snippets[2].content)) throw new Error('sensitive content leaked into preview DOM');
    await closeSearch();

    // RF3 lifecycle check is last: after the native window hides, no search-context calls follow.
    await openSearch();
    await exec('xdotool', ['windowactivate', '--sync', mainWindow]);
    await waitForWindow('search', false);
  });
});
