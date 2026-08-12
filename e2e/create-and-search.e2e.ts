import { $, browser, expect } from '@wdio/globals';

const suffix = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
const key = `e2e-${suffix}`;
const title = `E2E 验收 ${suffix}`;

describe('compiled Searchis application', () => {
  it('creates a snippet and finds it in the quick-search window', async () => {
    const createButton = await $('button[aria-label="新建片段"]');
    await createButton.waitForClickable({ timeout: 10_000 });
    await createButton.click();
    await $('input[placeholder="例如 email-work, addr-office"]').setValue(key);
    await $('input[placeholder="例如 工作邮箱, 公司地址"]').setValue(title);
    await $('textarea[placeholder="在此输入需要快速粘贴的任意文本片段..."]').setValue('E2E-only content');
    await $('//button[contains(., "保存片段数据")]').click();

    await expect($(`//*[contains(text(), "${title}")]`)).toBeDisplayed();

    await $('//button[contains(., "呼出快速窗口")]').click();
    await browser.tauri.switchWindow('search');
    const search = await $('input[aria-label="检索文本片段"]');
    await search.waitForDisplayed();
    await search.setValue(key);
    await expect($(`//*[contains(text(), "${title}")]`)).toBeDisplayed();

    await browser.keys('Escape');
    await browser.tauri.switchWindow('main');
    await expect($('button[aria-label="新建片段"]')).toBeDisplayed();
  });
});
