import { EventEmitter } from 'node:events';
import { captureException } from '@sentry/electron/main';
import { dialog } from 'electron';
import { autoUpdater } from 'electron-updater';
import type { BaseWindow } from 'electron';

const CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;
const INITIAL_DELAY_MS = 10 * 1000;

// eslint-disable-next-line unicorn/prefer-event-target
export class Updater extends EventEmitter<{ ready: [] }> {
  #enabled: boolean;
  #ready = false;

  constructor(enabled: boolean) {
    super();
    this.#enabled = enabled;
    autoUpdater.autoDownload = true;
    autoUpdater.autoInstallOnAppQuit = true;
    autoUpdater.on('update-downloaded', () => {
      this.#ready = true;
      this.emit('ready');
    });
    autoUpdater.on('error', (err) => {
      console.warn('[updater]', err.message);
      captureException(err);
    });
  }

  #check() {
    autoUpdater.checkForUpdates().catch(() => null);
  }

  get ready() {
    return this.#ready;
  }

  simulateReady() {
    this.#ready = true;
    this.emit('ready');
  }

  start() {
    if (!this.#enabled) return;
    setTimeout(() => this.#check(), INITIAL_DELAY_MS);
    setInterval(() => this.#check(), CHECK_INTERVAL_MS);
  }

  async checkManually(window?: BaseWindow) {
    if (!this.#enabled) {
      await dialog.showMessageBox({ type: 'info', message: '개발 빌드에서는 업데이트를 확인하지 않아요.', buttons: ['확인'] });
      return;
    }

    if (this.#ready) {
      await this.confirmRestart(window);
      return;
    }

    try {
      const result = await autoUpdater.checkForUpdates();
      const available = result?.isUpdateAvailable ?? false;
      await dialog.showMessageBox({
        type: 'info',
        message: available
          ? '새 버전을 내려받고 있어요.\n준비되면 오른쪽 위에 업데이트 버튼이 나타나요.'
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

  async confirmRestart(window?: BaseWindow) {
    if (!this.#ready) return;
    const options = {
      type: 'info' as const,
      message: '새 버전으로 업데이트할까요?',
      detail: '타이피가 다시 시작되고, 열려 있던 탭은 그대로 복원돼요.',
      buttons: ['다시 시작', '나중에'],
      defaultId: 0,
      cancelId: 1,
    };
    const { response } =
      window && !window.isDestroyed() ? await dialog.showMessageBox(window, options) : await dialog.showMessageBox(options);
    if (response === 0) autoUpdater.quitAndInstall();
  }
}
