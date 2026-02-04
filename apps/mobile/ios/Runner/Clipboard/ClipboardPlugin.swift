import Flutter
import UIKit
import UniformTypeIdentifiers

class ClipboardPlugin: NSObject, FlutterPlugin {
  static func register(with registrar: FlutterPluginRegistrar) {
    let channel = FlutterMethodChannel(
      name: "co.typie.clipboard",
      binaryMessenger: registrar.messenger()
    )
    let instance = ClipboardPlugin()
    registrar.addMethodCallDelegate(instance, channel: channel)
  }

  func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    switch call.method {
    case "setData":
      handleSetData(call, result: result)
    case "getData":
      handleGetData(result: result)
    default:
      result(FlutterMethodNotImplemented)
    }
  }

  private func handleSetData(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let args = call.arguments as? [String: Any],
          let text = args["text"] as? String,
          let html = args["html"] as? String else {
      result(FlutterError(code: "INVALID_ARGS", message: "Missing text or html", details: nil))
      return
    }

    let pasteboard = UIPasteboard.general
    pasteboard.items = [[
      UTType.utf8PlainText.identifier: text,
      UTType.html.identifier: html,
    ]]

    result(nil)
  }

  private func handleGetData(result: @escaping FlutterResult) {
    let pasteboard = UIPasteboard.general
    var data: [String: String?] = [:]

    if let html = pasteboard.value(forPasteboardType: UTType.html.identifier) as? String {
      data["html"] = html
    }

    if let text = pasteboard.string {
      data["text"] = text
    }

    result(data)
  }
}
