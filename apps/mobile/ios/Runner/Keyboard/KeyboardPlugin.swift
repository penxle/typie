import Flutter
import UIKit

class KeyboardPlugin: NSObject, FlutterPlugin {
  private let channel: FlutterMethodChannel
  
  required init(channel: FlutterMethodChannel) {
    self.channel = channel
    
    super.init()
  }
  
  deinit {
    dispose()
  }
  
  static func register(with registrar: any FlutterPluginRegistrar) {
    let channel = FlutterMethodChannel(name: "co.typie.keyboard", binaryMessenger: registrar.messenger())
    let instance = self.init(channel: channel)
    registrar.addMethodCallDelegate(instance, channel: channel)
  }
  
  func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    switch call.method {
    case "listen":
      listen()
      result(nil)
    case "dispose":
      dispose()
      result(nil)
    default:
      result(FlutterMethodNotImplemented)
    }
  }
  
  private func listen() {
    NotificationCenter.default.addObserver(
      self,
      selector: #selector(keyboardWillShow),
      name: UIResponder.keyboardWillShowNotification,
      object: nil
    )
    
    NotificationCenter.default.addObserver(
      self,
      selector: #selector(keyboardWillHide),
      name: UIResponder.keyboardWillHideNotification,
      object: nil
    )
  }
  
  private func dispose() {
    NotificationCenter.default.removeObserver(self)
  }
  
  @objc private func keyboardWillShow(_ notification: Notification) {
    if let keyboardFrame = notification.userInfo?[UIResponder.keyboardFrameEndUserInfoKey] as? CGRect {
      let keyboardHeight = keyboardFrame.height
      channel.invokeMethod("heightChanged", arguments: ["height": Double(keyboardHeight)])
    }
  }
  
  @objc private func keyboardWillHide(_ notification: Notification) {
    channel.invokeMethod("heightChanged", arguments: ["height": 0.0])
  }
}
  
