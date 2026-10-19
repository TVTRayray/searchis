import { mkdirSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import TauriWorkerService, { launcher } from '@wdio/tauri-service';

// SPEC-22：E2E 构建里 template_open / 文件选择对话框是替身。路径经环境变量传给被测应用，
// 必须在 launcher 启动应用之前（即本模块加载时）设置；worker 继承同一目录。
if (!process.env.SEARCHIS_E2E_FILES_DIR) {
  const dir = mkdtempSync(path.join(tmpdir(), 'searchis-e2e-files-'));
  process.env.SEARCHIS_E2E_FILES_DIR = dir;
  process.once('exit', () => rmSync(dir, { recursive: true, force: true }));
}
process.env.SEARCHIS_E2E_OPEN_LOG ??= path.join(process.env.SEARCHIS_E2E_FILES_DIR, 'open-calls.log');
process.env.SEARCHIS_E2E_IMPORT_GATE ??= path.join(process.env.SEARCHIS_E2E_FILES_DIR, 'import-gate');
// SPEC-23：带格式复制写入“记录替身”而不是真实剪贴板；导出目录指向临时目录（不写用户的下载目录）。
process.env.SEARCHIS_E2E_CLIPBOARD_LOG ??= path.join(process.env.SEARCHIS_E2E_FILES_DIR, 'clipboard-calls.log');
process.env.SEARCHIS_E2E_EXPORT_DIR ??= path.join(process.env.SEARCHIS_E2E_FILES_DIR, 'exports');
mkdirSync(process.env.SEARCHIS_E2E_EXPORT_DIR, { recursive: true });
process.env.SEARCHIS_E2E_PICK_LIST ??= path.join(process.env.SEARCHIS_E2E_FILES_DIR, 'pick-list.txt');
// SPEC-24：目录/保存/打开对话框的替身——文件第一行是要返回的路径，空或不存在表示取消。
process.env.SEARCHIS_E2E_PICK_DIR ??= path.join(process.env.SEARCHIS_E2E_FILES_DIR, 'pick-dir.txt');
process.env.SEARCHIS_E2E_BACKUP_SAVE ??= path.join(process.env.SEARCHIS_E2E_FILES_DIR, 'backup-save.txt');
process.env.SEARCHIS_E2E_BACKUP_OPEN ??= path.join(process.env.SEARCHIS_E2E_FILES_DIR, 'backup-open.txt');

export { launcher };

/**
 * The WDIO runner can delete the WebDriver session before service.afterSession.
 * The upstream cleanup then calls browser.execute() without a session id and
 * emits a misleading mock-store warning. There is nothing left to clean in
 * that state; preserve upstream cleanup whenever the session is still alive.
 */
export default class SearchisTauriService extends TauriWorkerService {
  async afterSession(...args) {
    if (!this.browser?.sessionId) return;
    return super.afterSession(...args);
  }
}
