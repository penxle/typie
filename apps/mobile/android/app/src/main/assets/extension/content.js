(() => {
  const port = browser.runtime.connect({ name: "content" });

  const script = document.createElement('script');
  script.innerHTML = `
    (() => {
      const handlers = new WeakMap();
      window.__webview__ = {
        emitEvent: (name, data) => window.postMessage({
          direction: 'user-to-content',
          type: 'emitEvent',
          name,
          data: JSON.stringify(data ?? null),
        }, '*'),
        addEventListener: (name, fn) => {
          const handler = (event) => { if (event.data.direction === 'content-to-user' && event.data.type === 'emitEvent' && event.data.name === name) fn(JSON.parse(event.data.data)) };
          handlers.set(fn, handler);
          window.addEventListener('message', handler);
        },
        removeEventListener: (name, fn) => {
          const handler = handlers.get(fn);
          if (handler) {
            window.removeEventListener('message', handler);
          }
        },
      };
    })();
  `;
  document.head.append(script);

  window.addEventListener('message', (event) => {
    if (event.data.direction !== 'user-to-content') {
      return;
    }

    if (event.data.type == "emitEvent") {
      port.postMessage({
        type: 'emitEvent',
        name: event.data.name,
        data: event.data.data,
      });
    }
  });

  port.onMessage.addListener((message) => {
    if (message.type == "emitEvent") {
      window.postMessage({
        direction: 'content-to-user',
        type: 'emitEvent',
        name: message.name,
        data: message.data,
      }, '*');
    }
  });
})();