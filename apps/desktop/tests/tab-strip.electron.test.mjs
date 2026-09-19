/* eslint-disable unicorn/prefer-top-level-await -- Electron ready waits for module evaluation */
import assert from 'node:assert/strict';
import path from 'node:path';
import { init } from '@sentry/electron/main';
import { app, BrowserWindow, ipcMain } from 'electron';

const root = process.env.TYPIE_DURABILITY_TEST_DIR;
if (!root) throw new Error('Set an isolated TYPIE_DURABILITY_TEST_DIR');
app.setPath('userData', root);
init({ dsn: undefined });

app
  .whenReady()
  .then(async () => {
    const window = new BrowserWindow({
      show: true,
      width: 900,
      height: 160,
      webPreferences: { preload: path.join(root, 'preload/chrome.cjs'), sandbox: true, backgroundThrottling: false },
    });
    const contents = window.webContents;
    try {
      await contents.loadFile(path.join(root, 'renderer/chrome/index.html'));
      const tabs = ['first', 'second'].map((id) => ({ id, title: id, url: '', icon: null, saving: false }));
      const read = (source) => contents.executeJavaScript(source);
      const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
      const waitForRemoval = async () => {
        for (let attempt = 0; attempt < 100; attempt++) {
          if (await read('document.querySelector("[data-id=first]") === null')) return;
          await delay(25);
        }
        assert.fail('The approved tab did not finish its outro');
      };
      contents.send('tabs:state', { tabs, activeId: 'first' });
      for (let attempt = 0; attempt < 100; attempt++) {
        if (await read('document.querySelectorAll("[role=tab]").length === 2')) break;
        await delay(20);
      }
      const requests = [];
      const close = (event, id) => {
        if (event.sender === contents) requests.push(id);
      };
      ipcMain.on('tabs:close', close);
      await read('document.querySelector("[data-id=first] button").click()');
      await delay(250);
      assert.deepEqual(requests, ['first']);
      assert.equal(await read('document.querySelectorAll("[role=tab]").length'), 2);
      assert.equal(await read('document.querySelector("[data-id=first]").getAttribute("aria-selected")'), 'true');

      contents.send('tabs:state', { tabs: tabs.slice(1), activeId: 'second' });
      await waitForRemoval();
      assert.equal(await read('document.querySelector("[data-id=first]") === null'), true);
      assert.equal(await read('document.querySelector("[data-id=second]").getAttribute("aria-selected")'), 'true');
      ipcMain.removeListener('tabs:close', close);
      console.log('PASS: a close request keeps the tab and selection until main approves removal');
    } finally {
      window.destroy();
    }
    app.quit();
  })
  .catch((err) => {
    console.error(err);
    app.exit(1);
  });
