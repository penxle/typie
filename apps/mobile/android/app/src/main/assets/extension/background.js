const connectNative = (contentPort) => {
  const nativePort = browser.runtime.connectNative("webview");

  nativePort.onMessage.addListener(async (message) => {
    if (message.type === "setCookies") {
      await Promise.all(
        message.cookies.map(async (cookie) => {
          await browser.cookies.set({
            url: cookie.url,
            name: cookie.name,
            value: cookie.value,
            domain: cookie.domain,
            path: cookie.path ?? "/",
            secure: cookie.secure ?? true,
            httpOnly: cookie.httpOnly ?? true,
            sameSite: cookie.sameSite ?? "lax",
          });
        })
      );
      
      nativePort.postMessage({ type: "cookiesSet" });
    } else if (message.type === "emitEvent") {
      contentPort.postMessage({
        type: "emitEvent",
        name: message.name,
        data: message.data,
      });
    }
  });

  return nativePort;
}

browser.runtime.onConnect.addListener((contentPort) => {
  const nativePort = connectNative(contentPort);
  
  contentPort.onMessage.addListener((message) => {
    if (message.type === "emitEvent") {
      nativePort.postMessage({
        type: "emitEvent",
        name: message.name,
        data: message.data,
      });
    }
  });
});