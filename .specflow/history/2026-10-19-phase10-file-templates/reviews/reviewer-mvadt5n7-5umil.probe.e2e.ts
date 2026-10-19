import { browser } from '@wdio/globals';
import { withExecuteOptions } from '@wdio/tauri-service';

// Reviewer-only defect reproduction, not acceptance tests. Uses the isolated E2E database.
const run = (fn: any, ...args: any[]) => browser.tauri.execute(fn, withExecuteOptions({ windowLabel: 'main' }), ...args);
const wait = (fn: any, ...args: any[]) => browser.waitUntil(async () => Boolean(await run(fn, ...args)), { timeout: 10000, interval: 100 });
const select = async (key: string, content: string) => {
  await wait((_t: any, key: string) => [...document.querySelectorAll('.manager-snippet-row')].some(e => e.textContent?.includes(key)), key);
  await run((_t: any, key: string) => ([...document.querySelectorAll('.manager-snippet-row')].find(e => e.textContent?.includes(key)) as HTMLElement).click(), key);
  await wait((_t: any, text: string) => document.querySelector('.cm-content')?.textContent === text, content);
};

describe('Reviewer CodeMirror defect reproduction', () => {
  it('observes cross-document undo and CRLF mutation on metadata-only save', async () => {
    await wait(() => Boolean(document.querySelector('.manager-sidebar-footer')));
    await run(async (tauri: any) => {
      for (const [key, content] of [['review-probe-a', 'AAA'], ['review-probe-b', 'BBB'], ['review-probe-crlf', 'line1\r\nline2\r\n']]) {
        await tauri.core.invoke('snippet_create', { input: { key, title: key, content, aliases: [], tags: [], pinned: false, sensitive: false, groupId: null, language: 'plain', requestId: crypto.randomUUID() } });
      }
    });
    await run(() => { location.reload(); });
    await select('review-probe-a', 'AAA');
    await select('review-probe-b', 'BBB');
    await run(() => {
      const editor = document.querySelector('.cm-content') as HTMLElement;
      editor.focus();
      editor.dispatchEvent(new KeyboardEvent('keydown', { key: 'z', code: 'KeyZ', ctrlKey: true, bubbles: true, cancelable: true }));
    });
    await wait(() => document.querySelector('.cm-content')?.textContent === 'AAA');
    const undoResult = await run(() => ({ selected: document.querySelector('.manager-snippet-row.is-selected')?.textContent, body: document.querySelector('.cm-content')?.textContent }));
    console.log('REVIEW_UNDO_OBSERVED', JSON.stringify(undoResult));

    await select('review-probe-crlf', 'line1line2');
    const before = await run(async (tauri: any) => (await tauri.core.invoke('snippet_list')).find((s: any) => s.key === 'review-probe-crlf').content);
    await run(() => ([...document.querySelectorAll('button')].find(e => e.textContent?.includes('保存片段数据')) as HTMLElement).click());
    await wait(async (tauri: any) => (await tauri.core.invoke('snippet_list')).find((s: any) => s.key === 'review-probe-crlf').content === 'line1\nline2\n');
    const after = await run(async (tauri: any) => (await tauri.core.invoke('snippet_list')).find((s: any) => s.key === 'review-probe-crlf').content);
    console.log('REVIEW_CRLF_OBSERVED', JSON.stringify({ before, after }));
  });
});
