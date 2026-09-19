/* eslint-disable unicorn/prefer-top-level-await -- Electron ready waits for module evaluation */
import assert from 'node:assert/strict';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { app, BaseWindow, dialog } from 'electron';

if (!process.env.TYPIE_DURABILITY_TEST_DIR) throw new Error('Set an isolated TYPIE_DURABILITY_TEST_DIR');
const root = process.env.TYPIE_DURABILITY_TEST_DIR;
app.setPath('userData', root);
const { showDocumentSaveDialog } = await import(pathToFileURL(path.join(root, 'main/document-save-dialog.js')));
const { buildMenu } = await import(pathToFileURL(path.join(root, 'main/menu.js')));
const waitFor = async (predicate) => {
  for (let attempt = 0; attempt < 100; attempt++) {
    if (await predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  assert.fail('The independent confirmation UI did not become ready');
};

app
  .whenReady()
  .then(async () => {
    const parent = new BaseWindow({ show: false, width: 1000, height: 700 });
    const data = {
      id: 'independent-dialog',
      title: '아직 저장을 완료하지 못했어요',
      description: '최근 변경사항을 저장하는 중이에요.\n지금 나가면 최근 변경사항을 잃을 수 있어요.',
      required: false,
      cancelLabel: '계속 편집',
      discardLabel: '저장하지 않고 나가기',
      documents: [{ id: 'document', title: '문서', location: '탭 1', status: 'pending' }],
    };
    const original = dialog.showMessageBox;
    dialog.showMessageBox = async () => {
      throw new Error('A healthy independent dialog must not need a native fallback');
    };
    let controller = new AbortController();
    try {
      const prompt = showDocumentSaveDialog(parent, data, controller.signal);
      await waitFor(() => parent.getChildWindows().some((window) => window.isVisible()));
      const child = parent.getChildWindows()[0];
      assert.equal(child.isModal(), true);
      assert.equal(child.getParentWindow(), parent);
      assert.equal(await child.webContents.executeJavaScript('!!window.typieDesktop'), false);
      let actions = 0;
      const callbacks = new Proxy(
        {},
        {
          get: () => () => {
            actions++;
          },
        },
      );
      const menu = buildMenu(callbacks, { devTools: false, updateReady: false, tabs: [{ title: '문서', active: true }], canReopen: false });
      const newTab = menu.items.find((item) => item.label === '파일').submenu.items.find((item) => item.label === '새 탭');
      newTab.click(newTab, child, {});
      assert.equal(actions, 0, 'An application-menu callback must not mutate tabs behind the modal');
      let result;
      void prompt.result.then((choice) => {
        result = choice;
      });
      await child.webContents.executeJavaScript(`window.documentSaveDialog.respond('${data.id}', 'continue')`);
      await new Promise((resolve) => setTimeout(resolve, 100));
      assert.equal(result, undefined, 'A completion action cannot authorize an incomplete dialog');
      app.focus({ steal: true });
      child.focus();
      child.webContents.focus();
      await waitFor(() => child.webContents.executeJavaScript('document.hasFocus()'));
      await child.webContents.executeJavaScript('document.querySelector("button:last-child").focus()');
      child.webContents.sendInputEvent({ type: 'keyDown', keyCode: 'Tab' });
      child.webContents.sendInputEvent({ type: 'keyUp', keyCode: 'Tab' });
      await waitFor(() => child.webContents.executeJavaScript('document.activeElement === document.querySelector("button")'));
      child.webContents.sendInputEvent({ type: 'keyDown', keyCode: 'Tab', modifiers: ['shift'] });
      child.webContents.sendInputEvent({ type: 'keyUp', keyCode: 'Tab', modifiers: ['shift'] });
      await waitFor(() => child.webContents.executeJavaScript('document.activeElement === document.querySelector("button:last-child")'));
      await child.webContents.executeJavaScript('document.querySelector("button").focus()');
      prompt.update({
        title: '이제 안전하게 나갈 수 있어요',
        description: '최근 변경사항을 안전하게 저장했어요.',
        completed: true,
        continueLabel: '나가기',
        documents: [{ ...data.documents[0], status: 'protected' }],
      });
      await waitFor(() => child.webContents.executeJavaScript('!!document.querySelector("[data-save-countdown]")'));
      assert.equal(
        await child.webContents.executeJavaScript('document.activeElement === document.querySelector("button")'),
        true,
        'A status snapshot must not reset keyboard focus',
      );
      void child.webContents
        .executeJavaScript(`[...document.querySelectorAll('button')].find((button) => button.textContent.includes('계속 편집')).click()`)
        .catch(() => null);
      assert.deepEqual(await prompt.result, { action: 'cancel', completionShown: true });
      assert.equal(child.isDestroyed(), true);
      assert.equal(parent.getChildWindows().length, 0);
      newTab.click(newTab, parent, {});
      assert.equal(actions, 1);
      console.log('PASS: standalone modal, real shared UI, menu guard, validated completion, and cancellation cleanup');

      controller = new AbortController();
      const required = showDocumentSaveDialog(
        parent,
        { ...data, id: 'login', required: true, cancelLabel: '다시 시도', discardLabel: '저장하지 않고 로그인' },
        controller.signal,
      );
      await waitFor(() => parent.getChildWindows().some((window) => window.isVisible()));
      const login = parent.getChildWindows()[0];
      let answered = false;
      void required.result.then(() => {
        answered = true;
      });
      login.close();
      login.webContents.sendInputEvent({ type: 'keyDown', keyCode: 'Escape' });
      login.webContents.sendInputEvent({ type: 'keyUp', keyCode: 'Escape' });
      await new Promise((resolve) => setTimeout(resolve, 100));
      assert.equal(answered, false);
      assert.equal(login.isDestroyed(), false);
      void login.webContents
        .executeJavaScript(`[...document.querySelectorAll('button')].find((button) => button.textContent.includes('다시 시도')).click()`)
        .catch(() => null);
      assert.deepEqual(await required.result, { action: 'retry', completionShown: false });
      console.log('PASS: required login ignores window close/Escape and offers an explicit retry');

      controller = new AbortController();
      const aborted = showDocumentSaveDialog(parent, { ...data, id: 'abort' }, controller.signal);
      await waitFor(() => parent.getChildWindows().some((window) => window.isVisible()));
      controller.abort();
      assert.deepEqual(await aborted.result, { action: 'aborted', completionShown: false });
      assert.equal(parent.getChildWindows().length, 0);
      console.log('PASS: withdrawing a pending operation destroys its UI without authorizing departure');
    } finally {
      controller.abort();
      dialog.showMessageBox = original;
      parent.destroy();
    }
    app.quit();
  })
  .catch((err) => {
    console.error(err);
    app.exit(1);
  });
