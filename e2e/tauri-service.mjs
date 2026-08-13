import TauriWorkerService, { launcher } from '@wdio/tauri-service';

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
