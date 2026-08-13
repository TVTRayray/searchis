import { mkdirSync } from 'node:fs';
import path from 'node:path';
import type { Options } from '@wdio/types';

const artifactDir = process.env.E2E_ARTIFACT_DIR ?? path.resolve('artifacts/e2e');

export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./e2e/create-and-search.e2e.ts'],
  maxInstances: 1,
  logLevel: 'warn',
  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: {
    ui: 'bdd',
    timeout: 30_000,
  },
  services: [[
    path.resolve('e2e/tauri-service.mjs'),
    {
      appBinaryPath: path.resolve('src-tauri/target/debug/searchis'),
      // External tauri-driver keeps session cleanup ordered after the native session closes.
      // E2E_DRIVER_PROVIDER can opt into embedded for local diagnosis only.
      driverProvider: process.env.E2E_DRIVER_PROVIDER === 'embedded' ? 'embedded' : 'external',
      windowLabel: 'main',
    },
  ]],
  capabilities: [{
    browserName: 'tauri',
    'tauri:options': { application: path.resolve('src-tauri/target/debug/searchis') },
  }],
  afterTest: async (_test, _context, result) => {
    if (result.passed) return;
    mkdirSync(artifactDir, { recursive: true });
    const name = _test.title.replaceAll(/[^a-z0-9]+/gi, '-').replace(/^-|-$/g, '');
    await browser.saveScreenshot(path.join(artifactDir, `${Date.now()}-${name || 'failure'}.png`));
  },
};
