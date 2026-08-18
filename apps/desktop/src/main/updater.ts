import { EventEmitter } from 'node:events';
import { captureException } from '@sentry/electron/main';
import { dialog } from 'electron';
import { autoUpdater } from 'electron-updater';

const CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;
const INITIAL_DELAY_MS = 10 * 1000;

// eslint-disable-next-line unicorn/prefer-event-target
export class Updater extends EventEmitter<{ ready: [string] }> {
  #enabled: boolean;
  #readyVersion: string | null = null;

  constructor(enabled: boolean) {
    super();
    this.#enabled = enabled;
    autoUpdater.autoDownload = true;
    autoUpdater.autoInstallOnAppQuit = true;
    autoUpdater.on('update-downloaded', (info) => {
      this.#readyVersion = info.version;
      this.emit('ready', info.version);
    });
    autoUpdater.on('error', (err) => {
      console.warn('[updater]', err.message);
      captureException(err);
    });
  }

  #check() {
    autoUpdater.checkForUpdates().catch(() => null);
  }

  start() {
    if (!this.#enabled) return;
    setTimeout(() => this.#check(), INITIAL_DELAY_MS);
    setInterval(() => this.#check(), CHECK_INTERVAL_MS);
  }

  async checkManually() {
    if (!this.#enabled) {
      await dialog.showMessageBox({ type: 'info', message: '개발 빌드에서는 업데이트를 확인하지 않아요.', buttons: ['확인'] });
      return;
    }

    if (this.#readyVersion) {
      const { response } = await dialog.showMessageBox({
        type: 'info',
        message: `새 버전 ${this.#readyVersion}이 준비됐어요.`,
        buttons: ['지금 재시작', '나중에'],
        cancelId: 1,
      });
      if (response === 0) this.restart();
      return;
    }

    try {
      const result = await autoUpdater.checkForUpdates();
      const available = result?.isUpdateAvailable ?? false;
      await dialog.showMessageBox({
        type: 'info',
        message:
          available && result
            ? `새 버전 ${result.updateInfo.version}을 내려받고 있어요. 준비되면 알려드릴게요.`
            : '현재 최신 버전을 이용하고 있어요.',
        buttons: ['확인'],
      });
    } catch (err) {
      captureException(err);
      await dialog.showMessageBox({
        type: 'error',
        message: '지금은 업데이트를 확인할 수 없어요.',
        detail: err instanceof Error ? err.message : String(err),
        buttons: ['확인'],
      });
    }
  }

  restart() {
    autoUpdater.quitAndInstall();
  }
}
