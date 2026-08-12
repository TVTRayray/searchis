import { mkdtemp, mkdir, rm } from 'node:fs/promises';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { tmpdir } from 'node:os';

const root = process.cwd();
const sandbox = await mkdtemp(path.join(tmpdir(), 'searchis-e2e-'));
const artifactDir = path.join(root, 'artifacts', 'e2e');
await mkdir(artifactDir, { recursive: true });

try {
  const child = spawn(process.execPath, ['node_modules/@wdio/cli/bin/wdio.js', 'run', 'wdio.conf.ts'], {
    cwd: root,
    env: {
      ...process.env,
      SEARCHIS_E2E: '1',
      XDG_DATA_HOME: path.join(sandbox, 'data'),
      XDG_CONFIG_HOME: path.join(sandbox, 'config'),
      E2E_ARTIFACT_DIR: artifactDir,
    },
    stdio: 'inherit',
  });
  const code = await new Promise((resolve, reject) => {
    child.once('error', reject);
    child.once('exit', (exitCode, signal) => resolve(exitCode ?? (signal ? 1 : 0)));
  });
  process.exitCode = code;
} finally {
  await rm(sandbox, { recursive: true, force: true });
}
