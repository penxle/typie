/* eslint-disable unicorn/prefer-top-level-await -- Electron ready waits for the entry module to finish evaluating */
import assert from 'node:assert/strict';
import { once } from 'node:events';
import { readFileSync } from 'node:fs';
import { createServer } from 'node:http';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { app, BaseWindow, dialog, session, WebContentsView } from 'electron';

// Run in a dedicated Electron process, never against the user's running app/profile.
if (!process.env.TYPIE_DURABILITY_TEST_DIR) throw new Error('Set an isolated TYPIE_DURABILITY_TEST_DIR');
app.setPath('userData', process.env.TYPIE_DURABILITY_TEST_DIR);
const { prepareDocumentDeparture, requestDocumentSave } = await import(
  pathToFileURL(path.join(process.env.TYPIE_DURABILITY_TEST_DIR, 'main/document-save.js'))
);

app
  .whenReady()
  .then(async () => {
    const window = new BaseWindow({ show: false, width: 800, height: 600, title: '타이피 저장 확인 테스트' });
    const views = [];
    const create = async (title, protectedChanges = true) => {
      const view = new WebContentsView({
        webPreferences: {
          preload: path.join(path.dirname(fileURLToPath(import.meta.url)), 'document-save-preload.cjs'),
          sandbox: true,
          contextIsolation: true,
        },
      });
      views.push(view);
      view.webContents.on('preload-error', (_event, _path, error) => console.error(error));
      window.contentView.addChildView(view);
      await view.webContents.loadURL('data:text/html,<button>Test document</button>');
      await view.webContents.executeJavaScript(`window.saveFixture.configure(${JSON.stringify({ protected: protectedChanges })})`);
      return { id: String(view.webContents.id), title, webContents: view.webContents };
    };
    const configure = (target, config) => target.webContents.executeJavaScript(`window.saveFixture.configure(${JSON.stringify(config)})`);
    const stops = (target) => target.webContents.executeJavaScript('window.saveFixture.stops()');
    const originalDialog = dialog.showMessageBox;
    try {
      const first = await create('성공 문서');
      const second = await create('실패 문서', false);
      const dialogHost = () => first.webContents;
      await configure(first, { protected: true });
      await configure(second, { protected: false, prepareDelay: 900 });
      const waitFor = async (predicate, message) => {
        for (let attempt = 0; attempt < 240; attempt++) {
          if (await predicate()) return;
          await new Promise((resolve) => setTimeout(resolve, 25));
        }
        assert.fail(message);
      };
      const dialogPage = {
        async executeJavaScript(source) {
          const child = window.getChildWindows()[0];
          if (!child || child.isDestroyed() || !child.isVisible()) return null;
          const wc = child.webContents;
          await wc.executeJavaScript(`{
            if (!window.inspectSaveDialog) {
              window.inspectSaveDialog = true;
              window.documentSaveDialog.subscribe((data) => { window.saveDialog = data; });
            }
            void 0;
          }`);
          return wc.executeJavaScript(source);
        },
      };
      const ready = async () => {
        await waitFor(() => dialogPage.executeJavaScript('!!window.saveDialog'), 'The independent save dialog did not load');
        return dialogPage.executeJavaScript('window.saveDialog.id');
      };
      const firstReady = ready();
      dialog.showMessageBox = async () => {
        throw new Error('A responsive app dialog must not use the system dialog');
      };
      const appDeparture = prepareDocumentDeparture({
        targets: () => [first, second],
        reason: 'close',
        window,
        commit: () => {
          throw new Error('Cancellation must not commit departure');
        },
        getActiveContents: dialogHost,
      });
      for (let attempt = 0; attempt < 15; attempt++) {
        if (await first.webContents.executeJavaScript('window.saveFixture.progress()')) break;
        await new Promise((resolve) => setTimeout(resolve, 50));
      }
      assert.equal(await first.webContents.executeJavaScript('window.saveFixture.progress()'), true);
      const firstDialogId = await firstReady;
      assert.equal(await first.webContents.executeJavaScript('window.saveFixture.progress()'), false);
      const prompt = await dialogPage.executeJavaScript('window.saveDialog');
      assert.equal(prompt.documents[0].status, 'failed');
      assert.equal(prompt.documents.length, 1);
      assert.match(prompt.documents[0].title, /저장 실패 문서/u);
      assert.equal(prompt.documents[0].icon, 'book-open');
      await second.webContents.executeJavaScript(`window.saveFixture.respondToDialog(${JSON.stringify(firstDialogId)}, 1)`);
      assert.equal(await stops(second), 1);
      await dialogPage.executeJavaScript(`window.documentSaveDialog.respond(${JSON.stringify(firstDialogId)}, 'cancel')`);
      assert.equal(await appDeparture, false);
      assert.equal(await dialogPage.executeJavaScript('window.saveDialog'), null);
      assert.equal(await stops(first), 0);
      assert.equal(await stops(second), 0);
      assert.equal(await first.webContents.executeJavaScript('window.saveFixture.recoveryNotifications()'), 0);
      console.log('PASS: the independent modal receives failures and a document renderer cannot approve departure');

      await configure(second, { protected: false, pending: true, documents: [] });
      const pendingTab = prepareDocumentDeparture({
        targets: () => [second],
        reason: 'close',
        window,
        commit: () => assert.fail('An unidentified pending tab must not leave'),
      });
      await ready();
      await dialogPage.executeJavaScript(`{
        window.pendingUpdates = [];
        window.documentSaveDialog.subscribe((data) => window.pendingUpdates.push(data));
        void 0;
      }`);
      await waitFor(() => dialogPage.executeJavaScript('window.pendingUpdates.length >= 2'), 'The pending tab was not polled');
      const pendingRow = await dialogPage.executeJavaScript('window.pendingUpdates.at(-1).documents[0]');
      assert.equal(pendingRow.status, 'pending');
      assert.equal(pendingRow.location, '', 'A tab placeholder must not repeat its title as a location');
      await dialogPage.executeJavaScript("window.documentSaveDialog.respond(window.saveDialog.id, 'cancel')");
      assert.equal(await pendingTab, false);
      console.log('PASS: a pending tab without document details retains its status and a single title across polling');

      const failedDocument = {
        id: 'shared',
        sessionId: 'first-session',
        title: '여러 곳에서 편집 중인 문서',
        status: 'failed',
        protectedChanges: false,
      };
      const secondDocument = { ...failedDocument, sessionId: 'second-session' };
      for (const separateTabs of [false, true]) {
        await configure(first, { protected: false, documents: separateTabs ? [failedDocument] : [failedDocument, secondDocument] });
        await configure(second, { protected: false, documents: [secondDocument] });
        const groupedDeparture = prepareDocumentDeparture({
          targets: () => (separateTabs ? [first, second] : [first]),
          reason: 'close',
          window,
          commit: () => assert.fail('A failed sibling must not be treated as protected'),
        });
        await ready();
        const groupedPrompt = await dialogPage.executeJavaScript('window.saveDialog');
        assert.equal(groupedPrompt.documents.length, 2, 'Each editing session must keep its own status');
        assert.notEqual(groupedPrompt.documents[0].id, groupedPrompt.documents[1].id);
        assert.equal(groupedPrompt.documents[0].status, 'failed');
        assert.equal(groupedPrompt.documents[0].protectedChanges, false);
        assert.deepEqual(
          groupedPrompt.documents.map((document) => document.location),
          [first.title, separateTabs ? second.title : first.title],
        );
        const protectedDocument = { ...failedDocument, status: 'protected', protectedChanges: true };
        await configure(first, {
          protected: separateTabs,
          documents: separateTabs ? [protectedDocument] : [protectedDocument, secondDocument],
        });
        let recovered;
        for (let attempt = 0; attempt < 40; attempt++) {
          recovered = await dialogPage.executeJavaScript('window.saveDialog');
          if (recovered.documents[0].status === 'protected') break;
          await new Promise((resolve) => setTimeout(resolve, 50));
        }
        assert.deepEqual(
          recovered.documents.map((document) => document.status),
          ['protected', 'failed'],
        );
        assert.equal(recovered.completed, false);
        await dialogPage.executeJavaScript(`window.documentSaveDialog.respond('${groupedPrompt.id}', 'cancel')`);
        assert.equal(await groupedDeparture, false);
      }
      console.log('PASS: sessions editing the same document recover independently across panes and tabs');
      await configure(first, { protected: true });

      let reloads = 0;
      for (const state of ['pending', 'failed', 'protected']) {
        await configure(second, { protected: false, pending: state !== 'failed' });
        const reloadReady = ready();
        const reloading = prepareDocumentDeparture({
          targets: () => [first, second],
          reason: 'reload',
          window,
          commit: () => {
            reloads++;
          },
          getActiveContents: dialogHost,
        });
        await reloadReady;
        const reloadPrompt = await dialogPage.executeJavaScript('window.saveDialog');
        assert.equal(reloadPrompt.required, false, 'A user-requested reload must remain cancellable');
        if (state === 'protected') {
          await configure(second, { protected: true });
          await waitFor(
            () => dialogPage.executeJavaScript('window.saveDialog?.completed === true'),
            'Protected reload must offer cancellation during its completion countdown',
          );
        }
        let reloadResult;
        void reloading.then((result) => {
          reloadResult = result;
        });
        if (state === 'failed') {
          const child = window.getChildWindows()[0];
          child.webContents.sendInputEvent({ type: 'keyDown', keyCode: 'Escape' });
          child.webContents.sendInputEvent({ type: 'keyUp', keyCode: 'Escape' });
        } else {
          // Clicking can destroy this renderer before executeJavaScript replies.
          // Observe the main-process departure result below instead.
          void dialogPage
            .executeJavaScript(
              `[...document.querySelectorAll('button')].find((button) => button.textContent.trim() === '계속 편집').click()`,
            )
            .catch(() => null);
        }
        await waitFor(() => reloadResult !== undefined, `The ${state} user reload did not respond to cancellation`);
        assert.equal(reloadResult, false);
        await waitFor(async () => (await stops(first)) === 0 && (await stops(second)) === 0, 'Cancelled reload must release its stops');
        await configure(second, { protected: true });
        assert.equal(reloads, 0);
        assert.equal(window.getChildWindows().length, 0);
      }
      // Outwait the cancelled five-second completion timer to detect a delayed reload.
      await new Promise((resolve) => setTimeout(resolve, 5200));
      assert.equal(reloads, 0, 'Late protection or an old completion timer must not replay a cancelled reload');
      assert.equal(window.getChildWindows().length, 0);
      assert.equal(await first.webContents.executeJavaScript('window.saveFixture.recoveryNotifications()'), 0);
      console.log('PASS: Continue editing and Escape cancel user reloads, including their completion countdown, without later replay');

      await configure(second, { protected: false, pending: true });
      const recoveryReady = ready();
      let recoveredCommit = false;
      const recoveredDeparture = prepareDocumentDeparture({
        targets: () => [first, second],
        reason: 'close',
        window,
        commit: () => {
          recoveredCommit = true;
        },
        getActiveContents: dialogHost,
      });
      await recoveryReady;
      assert.equal(await dialogPage.executeJavaScript('window.saveDialog.documents[0].status'), 'pending');
      await configure(second, { protected: false, pending: false });
      await waitFor(
        () => dialogPage.executeJavaScript('window.saveDialog.documents[0].status === "failed"'),
        'The next main-process check must refresh the document save state',
      );
      await dialogPage.executeJavaScript(`window.documentSaveDialog.respond(${JSON.stringify(firstDialogId)}, 'discard')`);
      assert.equal(recoveredCommit, false);
      await configure(second, { protected: true });
      for (let attempt = 0; attempt < 40; attempt++) {
        const data = await dialogPage.executeJavaScript('window.saveDialog');
        if (data?.completed === true) break;
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
      const recoveredPrompt = await dialogPage.executeJavaScript('window.saveDialog');
      assert.equal(recoveredPrompt.completed, true);
      assert.equal(recoveredPrompt.documents[0].status, 'protected');
      assert.equal(recoveredCommit, false);
      await configure(second, {
        protected: true,
        documents: [{ id: 'document', sessionId: 'session', title: '서버 저장 완료', status: 'synced', protectedChanges: true }],
      });
      await waitFor(
        () => dialogPage.executeJavaScript('window.saveDialog?.documents[0].status === "synced"'),
        'Server acknowledgement must reach the dialog independently of departure protection',
      );
      await waitFor(() => recoveredCommit, 'Repeated protected snapshots must not restart the visible completion countdown');
      assert.equal(await recoveredDeparture, true);
      assert.equal(recoveredCommit, true);
      assert.equal(await dialogPage.executeJavaScript('window.saveDialog'), null);
      assert.equal(await first.webContents.executeJavaScript('window.saveFixture.recoveryNotifications()'), 0);
      console.log('PASS: recovery retains the document row for a five-second countdown without a duplicate toast');

      await configure(second, { protected: false, pending: true });
      const earlyReady = ready();
      let earlyCommitted = false;
      const earlyDeparture = prepareDocumentDeparture({
        targets: () => [first, second],
        reason: 'close',
        window,
        commit: () => {
          earlyCommitted = true;
        },
        getActiveContents: dialogHost,
      });
      await earlyReady;
      await configure(second, { protected: true });
      for (let attempt = 0; attempt < 40; attempt++) {
        const data = await dialogPage.executeJavaScript('window.saveDialog');
        if (data?.completed === true) break;
        await new Promise((resolve) => setTimeout(resolve, 50));
      }
      const earlyPrompt = await dialogPage.executeJavaScript('window.saveDialog');
      assert.equal(earlyPrompt.completed, true);
      await dialogPage.executeJavaScript(`window.documentSaveDialog.respond('${earlyPrompt.id}', 'continue')`);
      assert.equal(await earlyDeparture, true);
      assert.equal(earlyCommitted, true);
      assert.equal(await first.webContents.executeJavaScript('window.saveFixture.recoveryNotifications()'), 0);
      console.log('PASS: confirming the completed app modal before the countdown ends does not duplicate its success message');

      await prepareDocumentDeparture({
        targets: () => [first, second],
        reason: 'close',
        window,
        commit: () => {
          assert.equal(first.webContents.isDestroyed(), false);
        },
        getActiveContents: dialogHost,
      });
      assert.equal(await first.webContents.executeJavaScript('window.saveFixture.recoveryNotifications()'), 0);
      console.log('PASS: an ordinary protected departure does not show a recovery notification');

      process.env.ELECTRON_RENDERER_URL = 'http://127.0.0.1:1';
      await configure(second, { protected: false });
      dialog.showMessageBox = async () => ({ response: 1, checkboxChecked: false });
      await prepareDocumentDeparture({
        targets: () => [first, second],
        reason: 'close',
        window,
        commit: () => {
          assert.equal(first.webContents.isDestroyed(), false);
        },
        getActiveContents: dialogHost,
      });
      assert.equal(await first.webContents.executeJavaScript('window.saveFixture.recoveryNotifications()'), 0);
      console.log('PASS: explicit discard never reports a successful save');
      delete process.env.ELECTRON_RENDERER_URL;
      await configure(first, { protected: true });

      await configure(second, { protected: false });
      let fallbackShown = false;
      dialog.showMessageBox = async () => {
        fallbackShown = true;
        return { response: 0, checkboxChecked: false };
      };
      const stalledReady = ready();
      const stalledDeparture = prepareDocumentDeparture({
        targets: () => [first, second],
        reason: 'close',
        window,
        commit: () => {
          throw new Error('An unresponsive app UI must not authorize departure');
        },
        getActiveContents: dialogHost,
      });
      const stalledId = await stalledReady;
      void dialogPage
        .executeJavaScript(
          `{
        const deadline = Date.now() + 5000;
        while (Date.now() < deadline) { /* Isolated renderer hang, never the user's app. */ }
        window.saveFixture.respondToDialog(${JSON.stringify(stalledId)}, 1);
      }`,
        )
        .catch(() => null);
      assert.equal(await stalledDeparture, false);
      assert.equal(fallbackShown, true);
      assert.equal(window.getChildWindows().length, 0);
      assert.equal(await stops(second), 0);
      console.log('PASS: a hung confirmation renderer falls back without granting departure');
      process.env.ELECTRON_RENDERER_URL = 'http://127.0.0.1:1';
      await configure(first, { protected: true });
      fallbackShown = false;
      assert.equal(
        await prepareDocumentDeparture({
          targets: () => [first, second],
          reason: 'close',
          window,
          commit: () => {
            throw new Error('A missing modal acknowledgement must not authorize departure');
          },
          getActiveContents: dialogHost,
        }),
        false,
      );
      assert.equal(fallbackShown, true);
      assert.equal(await stops(first), 0);
      assert.equal(await stops(second), 0);
      console.log('PASS: a confirmation page that cannot load falls back without closing any tab');
      delete process.env.ELECTRON_RENDERER_URL;
      await configure(first, { protected: true });
      let activeHost = first.webContents;
      const switchedReady = ready();
      fallbackShown = false;
      const switchedDeparture = prepareDocumentDeparture({
        targets: () => [first, second],
        reason: 'close',
        window,
        commit: () => {
          throw new Error('Switching the modal host tab must not authorize departure');
        },
        getActiveContents: () => activeHost,
      });
      const switchedId = await switchedReady;
      activeHost = second.webContents;
      await new Promise((resolve) => setTimeout(resolve, 1100));
      assert.equal(fallbackShown, false);
      await dialogPage.executeJavaScript(`window.documentSaveDialog.respond('${switchedId}', 'cancel')`);
      assert.equal(await switchedDeparture, false);
      assert.equal(await stops(first), 0);
      console.log('PASS: changing the active document does not replace the independent modal');
      process.env.ELECTRON_RENDERER_URL = 'http://127.0.0.1:1';
      let committed = false;
      dialog.showMessageBox = async (_window, options) => {
        assert.match(options.detail, /저장 실패 문서/u);
        assert.deepEqual(options.buttons, ['계속 편집', '변경사항 버리고 불러오기']);
        assert.equal(await stops(first), 1);
        assert.equal(await stops(second), 1);
        assert.equal(first.webContents.isDestroyed(), false);
        return { response: 0, checkboxChecked: false };
      };
      assert.equal(
        await prepareDocumentDeparture({
          targets: () => [first, second],
          reason: 'reload',
          window,
          commit: () => {
            committed = true;
          },
        }),
        false,
      );
      assert.equal(committed, false);
      assert.equal(await stops(first), 0);
      assert.equal(await stops(second), 0);
      console.log('PASS: the system fallback cancels a user reload and preserves every renderer');

      let prompts = 0;
      dialog.showMessageBox = async (_window, options) => {
        prompts++;
        assert.equal(options.buttons.length, 2);
        await configure(second, { protected: true });
        return { response: 0, checkboxChecked: false };
      };
      await prepareDocumentDeparture({
        targets: () => [first, second],
        reason: 'login',
        window,
        commit: () => {
          committed = true;
        },
      });
      assert.equal(committed, true);
      assert.equal(prompts, 1);
      console.log('PASS: required login still retries without a continue-editing choice');

      await configure(second, { protected: false });
      let recoveryHost;
      dialog.showMessageBox = async (_window, options) => {
        recoveryHost = first.webContents;
        await configure(second, { protected: true });
        await new Promise((resolve) => options.signal.addEventListener('abort', resolve, { once: true }));
        return { response: 0, checkboxChecked: false };
      };
      committed = false;
      assert.equal(
        await prepareDocumentDeparture({
          targets: () => [first, second],
          reason: 'close',
          window,
          commit: () => {
            committed = true;
          },
          getActiveContents: () => recoveryHost,
        }),
        true,
      );
      assert.equal(committed, true);
      assert.equal(await first.webContents.executeJavaScript('window.saveFixture.recoveryNotifications()'), 1);
      console.log('PASS: background protection dismisses the native failure dialog and revalidates');

      await configure(second, { protected: false, silent: true });
      dialog.showMessageBox = async () => {
        return { response: 0, checkboxChecked: false };
      };
      committed = false;
      assert.equal(
        await prepareDocumentDeparture({
          targets: () => [first, second],
          reason: 'close',
          window,
          commit: () => {
            committed = true;
          },
        }),
        false,
      );
      assert.equal(committed, false);
      console.log('PASS: a missing renderer response is not permission to close');

      await configure(second, { protected: true, replaceOnCommit: true });
      dialog.showMessageBox = async () => ({ response: 0, checkboxChecked: false });
      assert.equal(
        await prepareDocumentDeparture({
          targets: () => [second],
          reason: 'close',
          window,
          commit: () => {
            committed = true;
          },
        }),
        false,
      );
      assert.equal(committed, false);
      console.log('PASS: page replacement invalidates an earlier protection result');

      await configure(second, { protected: false });
      dialog.showMessageBox = async () => ({ response: 1, checkboxChecked: false });
      await prepareDocumentDeparture({
        targets: () => [first, second],
        reason: 'close',
        window,
        commit: () =>
          Promise.all(
            [first, second].map(
              ({ webContents }) =>
                new Promise((resolve) => {
                  webContents.once('destroyed', resolve);
                  webContents.close();
                }),
            ),
          ).then(() => {
            /* Every renderer has now been destroyed. */
          }),
      });
      assert.equal(first.webContents.isDestroyed(), true);
      assert.equal(second.webContents.isDestroyed(), true);
      console.log('PASS: explicit discard closes all prepared renderers together');

      const third = await create('Late response');
      await configure(third, { protected: true, silent: true });
      const result = await requestDocumentSave(third, { operationId: 'late', phase: 'prepare' });
      assert.equal(result.status, 'unknown');
      console.log('PASS: request timeout is explicitly unknown');

      delete process.env.ELECTRON_RENDERER_URL;
      const { TabManager } = await import(pathToFileURL(path.join(process.env.TYPIE_DURABILITY_TEST_DIR, 'main/tab-manager.js')));
      let redirect = false;
      const server = createServer((_request, response) => {
        if (redirect) response.writeHead(302, { Location: '/login' });
        else response.writeHead(200, { 'Content-Type': 'text/html' });
        response.end('<p>Isolated reload fixture</p>');
      });
      server.listen(0, '127.0.0.1');
      await once(server, 'listening');
      let login;
      const manager = new TabManager(
        {
          window,
          background: '#fff',
          attach: (view) => window.contentView.addChildView(view),
          detach: (view) => window.contentView.removeChildView(view),
        },
        {
          // Coverage itself is exercised above. This case isolates the native
          // navigation queue from renderer registration and live user accounts.
          classify: () => 'blocked',
          attach: (contents) =>
            contents.on('will-redirect', (event) => {
              event.preventDefault();
              login = manager.prepareDeparture('login', () => true);
            }),
        },
        0,
      );
      try {
        manager.create(`http://127.0.0.1:${server.address().port}/editor`);
        await once(manager.activeTab.view.webContents, 'did-finish-load');
        redirect = true;
        const interrupted = once(manager.activeTab.view.webContents, 'will-redirect');
        manager.reloadActive();
        await interrupted;
        assert.equal(await Promise.race([login, new Promise((resolve) => setTimeout(() => resolve('blocked'), 2000))]), true);
        console.log('PASS: an intercepted auth redirect releases reload before queued login');
      } finally {
        await manager.closeAll();
        server.close();
      }

      let requested = Promise.withResolvers();
      const appRequested = Promise.withResolvers();
      const appHtml = readFileSync(new URL('../../website/src/app.html', import.meta.url), 'utf8');
      const loadingServer = createServer((request, response) => {
        if (request.url === '/loading') {
          requested.resolve();
          return;
        }
        if (request.url === '/app.js') {
          appRequested.resolve(response);
          return;
        }
        response.writeHead(200, { 'Content-Type': 'text/html' });
        response.end(
          request.url === '/initial' || request.url === '/ready'
            ? appHtml
                .replace('%sveltekit.body%', () =>
                  request.url === '/initial' ? '<script src="/app.js"></script>' : '<p>Application entry</p>',
                )
                .replaceAll(/%[\w.]+%/gu, '')
            : '<p>A committed page</p>',
        );
      });
      loadingServer.listen(0, '127.0.0.1');
      await once(loadingServer, 'listening');
      const loadingManager = new TabManager(
        {
          window,
          background: '#fff',
          attach: (view) => window.contentView.addChildView(view),
          detach: (view) => window.contentView.removeChildView(view),
          presize: (view) => view.setBounds({ x: 0, y: 0, width: 800, height: 600 }),
        },
        {
          classify: (url) => (url.startsWith('http:') ? 'website' : 'blocked'),
          attach: (contents) => contents.setWindowOpenHandler(() => ({ action: 'deny' })),
        },
        0,
      );
      const loadingUrl = `http://127.0.0.1:${loadingServer.address().port}`;
      // The production HTML is exercised without contacting analytics or assets.
      session.defaultSession.webRequest.onBeforeRequest((details, callback) =>
        callback({ cancel: !details.url.startsWith(loadingUrl) && !details.url.startsWith('file:') }),
      );
      try {
        const loadedId = loadingManager.create(`${loadingUrl}/editor`);
        await once(loadingManager.activeTab.view.webContents, 'did-finish-load');
        const initialId = loadingManager.create(`${loadingUrl}/loading`, { background: true });
        await requested.promise;
        dialog.showMessageBox = async () => {
          throw new Error('An initial uncommitted tab has no editor to preserve');
        };
        assert.equal(await loadingManager.close(initialId), true);
        assert.equal(loadingManager.tabs.length, 1);
        console.log('PASS: a new tab with no committed document closes without a save dialog');

        let unexpectedDialog = false;
        dialog.showMessageBox = async () => {
          unexpectedDialog = true;
          return { response: 0 };
        };
        const bootingId = loadingManager.create(`${loadingUrl}/initial`);
        const booting = loadingManager.activeTab.view.webContents;
        await appRequested.promise;
        assert.equal(booting.getURL(), `${loadingUrl}/initial`);
        assert.equal(await loadingManager.close(bootingId), true);
        assert.equal(unexpectedDialog, false);
        console.log('PASS: a committed initial HTML page closes while the application script is still loading');

        const readyId = loadingManager.create(`${loadingUrl}/ready`);
        const readyPage = loadingManager.activeTab.view.webContents;
        await once(readyPage, 'did-finish-load');
        const readyTarget = { id: readyId, title: 'Initializing editor', webContents: readyPage };
        const initialPreparation = await requestDocumentSave(readyTarget, { operationId: 'initialize', phase: 'prepare' });
        assert.equal(initialPreparation.status, 'protected');
        await readyPage.executeJavaScript(`
          window.releaseSave = window.typieDesktop.onDocumentSave(async (request) => {
            if (request.phase === 'prepare') return new Promise((resolve) => { window.resolvePrepared = resolve; });
            return { status: 'protected', documents: [] };
          });
          void 0;
        `);
        const stalePreparation = requestDocumentSave(readyTarget, { operationId: 'initialize', phase: 'prepare' });
        for (let attempt = 0; attempt < 40; attempt++) {
          if (await readyPage.executeJavaScript('!!window.resolvePrepared')) break;
          await new Promise((resolve) => setTimeout(resolve, 25));
        }
        await readyPage.executeJavaScript(`
          window.typieDesktop.onDocumentSave(async () => ({ status: 'pending', documents: [] }));
          window.resolvePrepared({ status: 'protected', documents: [] });
        `);
        const replacedPreparation = await stalePreparation;
        assert.equal(replacedPreparation.status, 'unknown');
        assert.notEqual(replacedPreparation.generation, initialPreparation.generation);
        console.log('PASS: replacing the initial save responder invalidates its in-flight protection result');

        await readyPage.executeJavaScript(`
          window.typieDesktop.onDocumentSave(async (request) => ({
            status: 'unknown', documents: [],
            generation: request.phase === 'commit' ? 'replacement-session' : 'original-session',
          }));
          void 0;
        `);
        const beforeReplacement = await requestDocumentSave(readyTarget, { operationId: 'session-replacement', phase: 'prepare' });
        const afterReplacement = await requestDocumentSave(readyTarget, {
          operationId: 'session-replacement',
          phase: 'commit',
          discard: true,
        });
        assert.notEqual(
          afterReplacement.generation,
          beforeReplacement.generation,
          'Pane replacement must invalidate discard within the same page',
        );
        const staleDiscard = prepareDocumentDeparture({
          targets: () => [readyTarget],
          reason: 'close',
          window,
          commit: () => assert.fail('A prior discard must not close replacement sessions'),
        });
        const priorPrompt = await ready();
        await dialogPage.executeJavaScript(`window.documentSaveDialog.respond('${priorPrompt}', 'discard')`);
        await waitFor(
          () => dialogPage.executeJavaScript(`!!window.saveDialog && window.saveDialog.id !== '${priorPrompt}'`),
          'Replacement sessions did not receive their own confirmation',
        );
        await dialogPage.executeJavaScript("window.documentSaveDialog.respond(window.saveDialog.id, 'cancel')");
        assert.equal(await staleDiscard, false);
        console.log('PASS: session replacement invalidates a prior discard without replacing the page or its responder');

        await readyPage.executeJavaScript(`
          window.releaseSave = window.typieDesktop.onDocumentSave(async () => ({ status: 'protected', documents: [] }));
          void 0;
        `);
        const committed = await requestDocumentSave(readyTarget, { operationId: 'initialize', phase: 'commit' });
        assert.equal(committed.status, 'protected');
        assert.equal(
          await readyPage.executeJavaScript(`(() => {
            try { window.typieDesktop.onDocumentSave(async () => ({ status: 'protected', documents: [] })); return false; }
            catch { return true; }
          })()`),
          true,
        );
        await requestDocumentSave(readyTarget, { operationId: 'initialize', phase: 'release' });
        await readyPage.executeJavaScript(`
          window.typieDesktop.onDocumentSave(async (request) => {
            if (request.phase === 'commit') return new Promise((resolve) => { window.finishCommit = resolve; });
            return { status: 'protected', documents: [] };
          });
          void 0;
        `);
        const lateCommit = requestDocumentSave(readyTarget, { operationId: 'cancelled-commit', phase: 'commit' });
        await waitFor(() => readyPage.executeJavaScript('!!window.finishCommit'), 'Commit did not reach the renderer');
        await requestDocumentSave(readyTarget, { operationId: 'cancelled-commit', phase: 'release' });
        await readyPage.executeJavaScript('window.finishCommit({ status: "protected", documents: [] });');
        const lateResult = await lateCommit;
        assert.equal(lateResult.status, 'unknown', 'A released operation must not approve a late commit');
        await readyPage.executeJavaScript(`
          window.releaseSave = window.typieDesktop.onDocumentSave(async () => ({ status: 'protected', documents: [] }));
          window.releaseSave();
        `);
        const unregistered = await requestDocumentSave(readyTarget, { operationId: 'unregistered', phase: 'prepare' });
        assert.equal(unregistered.status, 'unknown');
        console.log('PASS: commit blocks late editor initialization, release restores registration, and missing handlers remain unknown');

        const anotherId = loadingManager.create(`${loadingUrl}/ready`);
        const anotherPage = loadingManager.activeTab.view.webContents;
        await once(anotherPage, 'did-finish-load');
        for (const contents of [readyPage, anotherPage]) {
          await contents.executeJavaScript(`
            window.saved = false;
            window.started = false;
            window.waitForPreparation = true;
            window.typieDesktop.onDocumentSave(async (request) => {
              if (request.phase === 'prepare') {
                window.started = true;
                if (window.waitForPreparation) await new Promise((resolve) => { window.finishPreparation = resolve; });
              }
              return {
                status: window.saved || request.discard ? 'protected' : 'pending',
                documents: [{ id: '${contents.id}', sessionId: 'session', title: '문서 ${contents.id}', status: window.saved ? 'protected' : 'pending' }],
              };
            });
            void 0;
          `);
        }
        dialog.showMessageBox = async () => {
          throw new Error('The responsive combined close dialog must not use the system dialog');
        };
        const firstClose = loadingManager.close(readyId);
        await waitFor(() => readyPage.executeJavaScript('window.started'), 'First tab did not begin preparation');
        const secondClose = loadingManager.close(anotherId);
        await waitFor(() => anotherPage.executeJavaScript('window.started'), 'Second tab waited for the first close to finish');
        for (const contents of [readyPage, anotherPage]) {
          await contents.executeJavaScript('window.waitForPreparation = false; window.finishPreparation();');
        }
        await waitFor(() => dialogPage.executeJavaScript('window.saveDialog?.documents.length === 2'), 'Both tabs must share one modal');
        const combined = await dialogPage.executeJavaScript('window.saveDialog');
        assert.deepEqual(
          combined.documents.map((document) => document.location),
          ['탭 2', '탭 3'],
        );
        assert.equal(loadingManager.tabs.filter((tab) => tab.saving).length, 2);
        await readyPage.executeJavaScript('window.saved = true;');
        await waitFor(
          () => dialogPage.executeJavaScript('window.saveDialog?.documents[0].status === "protected"'),
          'The saved sibling must update in the combined modal',
        );
        assert.equal(loadingManager.tabs.length, 3);
        await dialogPage.executeJavaScript(`window.documentSaveDialog.respond('${combined.id}', 'cancel')`);
        assert.deepEqual(await Promise.all([firstClose, secondClose]), [false, false]);
        assert.equal(loadingManager.tabs.length, 3);
        assert.equal(
          loadingManager.tabs.some((tab) => tab.saving),
          false,
        );
        console.log('PASS: separate tab closes prepare in parallel, identify their real tab positions, and cancel together');

        await readyPage.executeJavaScript('window.saved = false;');
        const switchClose = loadingManager.close(readyId);
        await waitFor(() => dialogPage.executeJavaScript('!!window.saveDialog'), 'The app modal did not open before switching tabs');
        const switchPromptId = await dialogPage.executeJavaScript('window.saveDialog.id');
        loadingManager.activate(readyId);
        await new Promise((resolve) => setTimeout(resolve, 1100));
        assert.equal(await dialogPage.executeJavaScript('window.saveDialog.id'), switchPromptId);
        await dialogPage.executeJavaScript(`window.documentSaveDialog.respond('${switchPromptId}', 'cancel')`);
        assert.equal(await switchClose, false);
        assert.equal(loadingManager.tabs.length, 3);
        loadingManager.activate(anotherId);
        console.log('PASS: tab changes leave the same independent modal and cancellation preserves every tab');

        await readyPage.executeJavaScript('window.saved = false;');
        const retryFirst = loadingManager.close(readyId);
        await waitFor(() => dialogPage.executeJavaScript('window.saveDialog?.documents.length === 1'), 'First close modal missing');
        const originalPromptId = await dialogPage.executeJavaScript('window.saveDialog.id');
        await readyPage.executeJavaScript('window.saved = true;');
        await waitFor(() => dialogPage.executeJavaScript('window.saveDialog?.completed === true'), 'Recovery countdown did not begin');
        await anotherPage.executeJavaScript('window.started = false; window.waitForPreparation = true;');
        const retrySecond = loadingManager.close(anotherId);
        assert.equal(await dialogPage.executeJavaScript('window.saveDialog?.completed'), false);
        assert.equal(await dialogPage.executeJavaScript('window.saveDialog?.documents[1].status'), 'pending');
        assert.equal(await dialogPage.executeJavaScript('window.saveDialog?.documents[1].title'), '탭 3');
        await waitFor(() => anotherPage.executeJavaScript('window.started'), 'The additional tab did not begin saving');
        await anotherPage.executeJavaScript('window.finishPreparation();');
        await waitFor(
          () => dialogPage.executeJavaScript(`window.saveDialog?.documents.some((row) => row.title === '문서 ${anotherPage.id}')`),
          'Late close did not join the modal',
        );
        assert.equal(await dialogPage.executeJavaScript('window.saveDialog.id'), originalPromptId);
        await dialogPage.executeJavaScript(`window.documentSaveDialog.respond('${originalPromptId}', 'discard')`);
        assert.deepEqual(await Promise.all([retryFirst, retrySecond]), [true, true]);
        assert.equal(loadingManager.tabs.length, 1);
        console.log('PASS: an additional close resets the countdown in the same modal and explicit discard closes both tabs');
        loadingManager.activate(loadedId);

        requested = Promise.withResolvers();
        void loadingManager.activeTab.view.webContents.loadURL(`${loadingUrl}/loading`).catch(() => null);
        await requested.promise;
        let fallback = false;
        dialog.showMessageBox = async () => {
          fallback = true;
          return { response: 0 };
        };
        const protectedClose = loadingManager.prepareDeparture(
          'close',
          () => {
            throw new Error('Unknown previous page must not close');
          },
          loadedId,
        );
        for (let attempt = 0; attempt < 20 && !loadingManager.tabs[0].saving; attempt++) {
          await new Promise((resolve) => setTimeout(resolve, 50));
        }
        assert.equal(loadingManager.tabs[0].saving, true);
        await ready();
        assert.equal(await dialogPage.executeJavaScript('window.saveDialog.documents[0].status'), 'unknown');
        await dialogPage.executeJavaScript("window.documentSaveDialog.respond(window.saveDialog.id, 'cancel')");
        assert.equal(await protectedClose, false);
        assert.equal(fallback, false);
        assert.equal(loadingManager.tabs[0].saving, false);
        console.log('PASS: loading a replacement page does not waive protection, and cancellation clears the tab spinner');
      } finally {
        session.defaultSession.webRequest.onBeforeRequest(null);
        await loadingManager.closeAll();
        loadingServer.closeAllConnections();
        loadingServer.close();
      }
    } finally {
      dialog.showMessageBox = originalDialog;
      for (const view of views) if (view.webContents && !view.webContents.isDestroyed()) view.webContents.close();
      window.destroy();
      app.quit();
    }
  })
  .catch((err) => {
    console.error(err);
    app.exit(1);
  });
