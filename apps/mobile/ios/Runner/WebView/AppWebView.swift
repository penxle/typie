import Flutter
import UIKit
import WebKit

class AppWebView: NSObject, FlutterPlatformView {
  private let webView: WKWebView
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
    configuration.websiteDataStore = WKWebsiteDataStore.nonPersistent()

    if let params = args as? [String: Any] {
      if let cookies = params["initialCookies"] as? [[String: Any]] {
        for cookie in cookies {
          let httpCookie = HTTPCookie(properties: [
            .name: cookie["name"]!,
            .value: cookie["value"]!,
            .domain: cookie["domain"]!,
            .path: "/",
            .sameSitePolicy: HTTPCookieStringPolicy.sameSiteLax,
            .secure: true,
          ])!

          configuration.websiteDataStore.httpCookieStore.setCookie(httpCookie)
        }
      }
    }

    webView = WKWebView(frame: frame, configuration: configuration)

    super.init()

    webView.navigationDelegate = self
    webView.uiDelegate = self

    if #available(iOS 16.4, *) {
      webView.isInspectable = true
    }

    setupConsole()
    disableZoom()

    if let params = args as? [String: Any] {
      if let userAgent = params["userAgent"] as? String {
        webView.customUserAgent = userAgent
      }

      if let initialUrl = params["initialUrl"] as? String,
         let nsUrl = URL(string: initialUrl)
      {
        let request = URLRequest(
          url: nsUrl,
          cachePolicy: .reloadIgnoringLocalCacheData,
          timeoutInterval: 60.0
        )

        webView.load(request)
      }
    }

    channel.setMethodCallHandler { [weak self] call, result in
      guard let self = self else {
        return
      }

      guard let args = call.arguments as? [String: Any] else {
        return
      }

      switch call.method {
        case "loadUrl":
          if let url = args["url"] as? String, let nsUrl = URL(string: url) {
            let request = URLRequest(
              url: nsUrl,
              cachePolicy: .reloadIgnoringLocalCacheData,
              timeoutInterval: 60.0
            )

            webView.load(request)
          }

        case "dispose":
          dispose()

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

    webView.configuration.userContentController.removeScriptMessageHandler(forName: "onConsole")
    webView.configuration.userContentController.removeAllUserScripts()
  }

  private func setupConsole() {
    let script = """
      (() => {
        const log = (level) => (message) => window.webkit.messageHandlers.onConsole.postMessage({ level, message: String(message) });
        console.log = log('LOG'); console.debug = log('DEBUG'); console.info = log('INFO'); console.warn = log('WARN'); console.error = log('ERROR');
      })();
    """

    let userScript = WKUserScript(
      source: script,
      injectionTime: .atDocumentStart,
      forMainFrameOnly: false
    )

    webView.configuration.userContentController.add(self, name: "onConsole")
    webView.configuration.userContentController.addUserScript(userScript)
  }

  private func disableZoom() {
    let script = """
      (() => {
        const meta = document.createElement('meta'); meta.name = 'viewport'; meta.content = 'width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no';
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
}

extension AppWebView: WKNavigationDelegate {
  func webViewWebContentProcessDidTerminate(_ webView: WKWebView) {
    webView.reload()
  }
}

extension AppWebView: WKUIDelegate {}

extension AppWebView: WKScriptMessageHandler {
  func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
    if message.name == "onConsole", let body = message.body as? [String: Any] {
      guard let level = body["level"] as? String,
            let _message = body["message"] as? String else { return }

      channel.invokeMethod("onConsole", arguments: [
        "level": level,
        "message": _message,
      ])
    }
  }
}
