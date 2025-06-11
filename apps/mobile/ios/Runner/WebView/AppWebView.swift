import Flutter
import UIKit
import WebKit

class AppWebView: NSObject, FlutterPlatformView {
  private let webView: AppWKWebView
  private let channel: FlutterMethodChannel

  init(
    frame: CGRect,
    messenger: FlutterBinaryMessenger,
    id: Int64,
    args: Any?,
  ) {
    channel = FlutterMethodChannel(name: "co.typie.webview.\(id)", binaryMessenger: messenger)

    let configuration = WKWebViewConfiguration()

    configuration.suppressesIncrementalRendering = true
    configuration.selectionGranularity = .character
    configuration.websiteDataStore = WKWebsiteDataStore.default()

    configuration.setURLSchemeHandler(AppWebViewSchemeHandler(), forURLScheme: "picker")

    if let params = args as? [String: Any] {
      if let cookies = params["initialCookies"] as? [[String: Any]] {
        for cookie in cookies {
          let httpCookie = HTTPCookie(properties: [
            .name: cookie["name"]!,
            .value: cookie["value"]!,
            .domain: cookie["domain"]!,
            .path: "/",
            .sameSitePolicy: HTTPCookieStringPolicy.sameSiteLax,
          ])!

          configuration.websiteDataStore.httpCookieStore.setCookie(httpCookie)
        }
      }
    }

    webView = AppWKWebView(frame: frame, configuration: configuration)

    NotificationCenter.default.removeObserver(webView, name: UIResponder.keyboardWillShowNotification, object: nil)
    NotificationCenter.default.removeObserver(webView, name: UIResponder.keyboardWillHideNotification, object: nil)
    NotificationCenter.default.removeObserver(webView, name: UIResponder.keyboardWillChangeFrameNotification, object: nil)

    super.init()

    webView.navigationDelegate = self
    webView.uiDelegate = self

    if #available(iOS 16.4, *) {
      webView.isInspectable = true
    }

    webView.setKeyboardRequiresUserInteraction(false)
    webView.scrollView.contentInsetAdjustmentBehavior = .never
    webView.configuration.userContentController.add(self, name: "handler")

    setupConsole()
    disableZoom()
    setupEventChannel()

    if let params = args as? [String: Any] {
      if let userAgent = params["userAgent"] as? String {
        webView.customUserAgent = userAgent
      }

      if let initialUrl = params["initialUrl"] as? String,
         let nsUrl = URL(string: initialUrl)
      {
        let request = URLRequest(
          url: nsUrl,
          cachePolicy: .useProtocolCachePolicy,
          timeoutInterval: 60.0
        )

        webView.load(request)
      }
    }

    channel.setMethodCallHandler { [weak self] call, result in
      guard let self = self,
            let args = call.arguments as? [String: Any]
      else {
        result(nil)
        return
      }

      switch call.method {
      case "requestFocus":
        webView.becomeFirstResponder()
        result(nil)

      case "clearFocus":
        webView.resignFirstResponder()
        result(nil)

      case "emitEvent":
        if let name = args["name"] as? String, let data = args["data"] as? String {
          webView.evaluateJavaScript("""
            window.dispatchEvent(new CustomEvent('__webview__', { 
              detail: { 
                name: "\(name)", 
                data: JSON.parse("\(data)") 
              } 
            }));
          """)
        }
        result(nil)

      case "dispose":
        dispose()
        result(nil)

      default:
        result(FlutterMethodNotImplemented)
      }
    }
  }

  deinit {
    dispose()
  }

  func view() -> UIView {
    return webView
  }

  private func dispose() {
    channel.setMethodCallHandler(nil)

    webView.navigationDelegate = nil
    webView.uiDelegate = nil

    webView.configuration.userContentController.removeScriptMessageHandler(forName: "handler")
    webView.configuration.userContentController.removeAllUserScripts()

    webView.loadHTMLString("", baseURL: nil)
  }

  private func setupConsole() {
    let script = """
      (() => {
        const log = (level) => (...args) => {
          const formatArg = (arg) => {
            if (arg === null) return 'null';
            if (arg === undefined) return 'undefined';
            if (typeof arg === 'string') return arg;
            if (typeof arg === 'number' || typeof arg === 'boolean') return String(arg);
            if (typeof arg === 'function') return arg.toString();
            try {
              return JSON.stringify(arg, null, 2);
            } catch (e) {
              return String(arg);
            }
          };
          const message = args.map(formatArg).join(' ');
          window.webkit.messageHandlers.handler.postMessage({ name: 'console', attrs: { level, message }});
        };
        console.log = log('LOG'); console.debug = log('DEBUG'); console.info = log('INFO'); console.warn = log('WARN'); console.error = log('ERROR');
      })();
    """

    let userScript = WKUserScript(
      source: script,
      injectionTime: .atDocumentStart,
      forMainFrameOnly: false
    )

    webView.configuration.userContentController.addUserScript(userScript)
  }

  private func disableZoom() {
    let script = """
      (() => {
        const meta = document.createElement('meta'); meta.name = 'viewport'; meta.content = 'width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover';
        const head = document.querySelector('head'); head.appendChild(meta);
      })();
    """

    let userScript = WKUserScript(
      source: script,
      injectionTime: .atDocumentEnd,
      forMainFrameOnly: true
    )

    webView.configuration.userContentController.addUserScript(userScript)
  }

  private func setupEventChannel() {
    let script = """
      (() => {
        const handlers = new WeakMap();
        window.__webview__ = {
          emitEvent: (name, data) => window.webkit.messageHandlers.handler.postMessage({
            name: 'emitEvent',
            attrs: { name, data: JSON.stringify(data ?? null) },
          }),
          addEventListener: (name, fn) => {
            const handler = (event) => { if (event.detail.name === name) fn(event.detail.data) };
            handlers.set(fn, handler);
            window.addEventListener('__webview__', handler);
          },
          removeEventListener: (name, fn) => {
            const handler = handlers.get(fn);
            if (handler) {
              window.removeEventListener('__webview__', handler);
            }
          },
        };
      })();
    """

    let userScript = WKUserScript(
      source: script,
      injectionTime: .atDocumentStart,
      forMainFrameOnly: true
    )

    webView.configuration.userContentController.addUserScript(userScript)
  }
}

extension AppWebView: WKNavigationDelegate {
  func webViewWebContentProcessDidTerminate(_ webView: WKWebView) {
    webView.reload()
  }
}

extension AppWebView: WKUIDelegate {}

extension AppWebView: WKScriptMessageHandler {
  func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
    guard let body = message.body as? [String: Any],
          let name = body["name"] as? String,
          let attrs = body["attrs"] as? [String: Any]
    else {
      return
    }

    switch name {
    case "console":
      if let level = attrs["level"] as? String,
         let message = attrs["message"] as? String
      {
        channel.invokeMethod("console", arguments: [
          "level": level,
          "message": message,
        ])
      }
    case "emitEvent":
      if let name = attrs["name"] as? String,
         let data = attrs["data"] as? String
      {
        channel.invokeMethod("emitEvent", arguments: [
          "name": name,
          "data": data,
        ])
      }
    default:
      break
    }
  }
}

class AppWebViewSchemeHandler: NSObject, WKURLSchemeHandler {
  func webView(_ webView: WKWebView, start urlSchemeTask: any WKURLSchemeTask) {
    guard let url = urlSchemeTask.request.url else {
      urlSchemeTask.didFailWithError(NSError(domain: "co.typie.webview", code: -1))
      return
    }

    switch url.scheme {
    case "picker":
      do {
        let data = try Data(contentsOf: URL(fileURLWithPath: url.path))

        let mimetype = URLComponents(url: url, resolvingAgainstBaseURL: false)?
          .queryItems?
          .first(where: { $0.name == "type" })?
          .value ?? "application/octet-stream"

        let response = HTTPURLResponse(
          url: url,
          mimeType: mimetype,
          expectedContentLength: data.count,
          textEncodingName: nil
        )

        urlSchemeTask.didReceive(response)
        urlSchemeTask.didReceive(data)
        urlSchemeTask.didFinish()
      } catch {
        urlSchemeTask.didFailWithError(error)
      }
    default:
      urlSchemeTask.didFailWithError(
        NSError(
          domain: "co.typie.webview",
          code: 0,
          userInfo: [NSLocalizedDescriptionKey: "Invalid URL scheme"]
        )
      )
    }
  }

  func webView(_ webView: WKWebView, stop urlSchemeTask: any WKURLSchemeTask) {}
}
