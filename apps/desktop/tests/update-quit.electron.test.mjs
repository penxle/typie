/* eslint-disable unicorn/prefer-top-level-await -- Electron ready waits for module evaluation */
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { init } from '@sentry/electron/main';
import { app, autoUpdater as nativeUpdater, BaseWindow, dialog, ipcMain, Menu, session, webContents } from 'electron';

const root = process.env.TYPIE_DURABILITY_TEST_DIR;
if (!root) throw new Error('Set an isolated TYPIE_DURABILITY_TEST_DIR');
if (process.platform !== 'darwin') throw new Error('This test exercises the macOS two-stage updater');
const profile = mkdtempSync(path.join(root, 'profile-'));
app.setPath('appData', profile);
app.setPath('userData', path.join(profile, 'Typie'));
app.userAgentFallback = 'TypieTest/1.0';
process.env.ENVIRONMENT = 'local';
init({ dsn: undefined });

const waitFor = async (predicate, message) => {
  for (let attempt = 0; attempt < 200; attempt++) {
    if (await predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, 25));
  }
  assert.fail(message);
};

app.whenReady().then(async () => {
  try {
    // Run the actual entry point and native tab lifecycle. Only the website and
    // installer are controlled: no user profile, server, or OS update is touched.
    session.defaultSession.protocol.handle(
      'http',
      () =>
        new Response(
          `<!doctype html>
      <title>Update recovery document</title>
      <script>
        window.saved = true;
        window.stops = new Set();
        window.typieDesktop.onDocumentSave(async (request) => {
          if (request.phase === 'prepare') window.stops.add(request.operationId);
          if (request.phase === 'release') window.stops.delete(request.operationId);
          return {
            status: window.saved || request.discard ? 'protected' : 'failed',
            generation: 'document',
            documents: [{ id: 'document', sessionId: 'session', title: document.title, status: window.saved ? 'synced' : 'failed' }],
          };
        });
      </script>`,
          { headers: { 'Content-Type': 'text/html; charset=utf-8' } },
        ),
    );
    await session.defaultSession.cookies.set({ url: 'http://localhost:4100', name: 'typie-at', value: 'test-session' });
    let installations = 0;
    let failImmediately = false;
    nativeUpdater.checkForUpdates = () => {
      installations++;
      if (failImmediately) throw new Error('Installer could not start');
    };
    nativeUpdater.quitAndInstall = () => {
      nativeUpdater.emit('before-quit-for-update');
      for (const window of BaseWindow.getAllWindows()) window.close();
    };
    const messages = [];
    dialog.showMessageBox = async (...args) => {
      messages.push(args.at(-1));
      return { response: 0 };
    };
    await import(pathToFileURL(path.join(root, 'main/index.js')));
    const documents = () => webContents.getAllWebContents().filter((contents) => contents.getURL().startsWith('http://localhost:4100/'));
    await waitFor(() => documents().length === 1, 'The real main entry did not create a document tab');
    const first = documents()[0];
    await waitFor(() => first.executeJavaScript('!!window.stops'), 'The document save handler did not initialize');
    ipcMain.emit('tabs:new');
    await waitFor(() => documents().length === 2, 'The second tab did not load');
    const pages = documents();
    for (const page of pages) await waitFor(() => page.executeJavaScript('!!window.stops'), 'Save handler missing');
    const developmentMenu = Menu.getApplicationMenu().items.find((item) => item.label === '보기');
    developmentMenu.submenu.items.find((item) => item.label === '업데이트 알약 표시(개발용)').click();
    ipcMain.emit('update:restart');
    await waitFor(() => installations === 1, 'The accepted restart did not request installation');
    assert.ok(
      pages.every((page) => !page.isDestroyed()),
      'Installer preparation must not destroy document tabs',
    );
    for (const page of pages) assert.equal(await page.executeJavaScript('window.stops.size'), 1);

    nativeUpdater.emit('error', new Error('Installer preparation failed'));
    await waitFor(async () => (await first.executeJavaScript('window.stops.size')) === 0, 'Failed installation did not release editing');
    assert.ok(
      pages.every((page) => !page.isDestroyed()),
      'Failed installation must retain every tab',
    );
    assert.ok(
      messages.some((message) => message.type === 'error'),
      'Installation failure must be explained to the user',
    );
    for (const page of pages) await page.executeJavaScript('window.saved = false');
    // A late installer callback after failure must not authorize an unprepared quit.
    nativeUpdater.emit('before-quit-for-update');
    let prevented = false;
    app.emit('before-quit', {
      preventDefault: () => {
        prevented = true;
      },
    });
    assert.equal(prevented, true, 'A later quit must still enter document-save protection');
    const modal = () => BaseWindow.getAllWindows().find((window) => window.isModal());
    await waitFor(() => modal()?.isVisible(), 'The later unsaved quit did not ask for confirmation');
    await modal().webContents.executeJavaScript(`{
      window.documentSaveDialog.subscribe(data => window.documentSaveDialog.respond(data.id, 'cancel'));
      void 0;
    }`);
    await waitFor(async () => (await first.executeJavaScript('window.stops.size')) === 0, 'Cancellation did not release editing');
    assert.equal(installations, 1, 'Cancelling save confirmation must not start another installer');
    assert.ok(pages.every((page) => !page.isDestroyed()));
    console.log('PASS: delayed and failed update installation preserves tabs and later quit protection');

    for (const page of pages) await page.executeJavaScript('window.saved = true');
    failImmediately = true;
    ipcMain.emit('update:restart');
    await waitFor(() => installations === 2, 'A failed update could not be retried');
    await waitFor(async () => (await first.executeJavaScript('window.stops.size')) === 0, 'Synchronous failure did not release editing');
    assert.ok(pages.every((page) => !page.isDestroyed()));
    console.log('PASS: a synchronous installer exception also retains tabs and releases preparation');

    failImmediately = false;
    ipcMain.emit('update:restart');
    await waitFor(() => installations === 3, 'The installer could not recover after a synchronous exception');
    assert.ok(pages.every((page) => !page.isDestroyed()));
    // Emulate the installer boundary only: native Electron closes windows after
    // before-quit-for-update. The application's own close handlers still run.
    nativeUpdater.emit('update-downloaded');
    await waitFor(() => pages.every((page) => page.isDestroyed()), 'Confirmed update quit did not dispose document renderers');
    const saved = JSON.parse(readFileSync(path.join(profile, 'Typie/state.json'), 'utf8'));
    assert.equal(saved.tabs.urls.length, 2, 'Restart must retain the original tab restore state');
    console.log('PASS: only actual update quit disposes tabs and retains their restore state');
    app.exit(0);
  } catch (err) {
    console.error(err);
    app.exit(1);
  }
});
