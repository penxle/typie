import Flutter
import GameController
import UIKit

class KeyboardPlugin: NSObject, FlutterStreamHandler {
  private let channel: FlutterEventChannel
  private var events: FlutterEventSink?
  
  required init(channel: FlutterEventChannel) {
    self.channel = channel
    
    super.init()
  }
  
  static func register(with registrar: any FlutterPluginRegistrar) {
    let channel = FlutterEventChannel(name: "co.typie.keyboard", binaryMessenger: registrar.messenger())
    let instance = self.init(channel: channel)
    channel.setStreamHandler(instance)
  }
  
  func onListen(withArguments arguments: Any?, eventSink events: @escaping FlutterEventSink) -> FlutterError? {
    self.events = events
    
    NotificationCenter.default.addObserver(
      self,
      selector: #selector(keyboardWillHide),
      name: UIResponder.keyboardWillHideNotification,
      object: nil
    )

    NotificationCenter.default.addObserver(
      self,
      selector: #selector(keyboardWillChangeFrame),
      name: UIResponder.keyboardWillChangeFrameNotification,
      object: nil
    )
    
    NotificationCenter.default.addObserver(
      self,
      selector: #selector(hardwareKeyboardDidConnect),
      name: .GCKeyboardDidConnect,
      object: nil
    )

    NotificationCenter.default.addObserver(
      self,
      selector: #selector(hardwareKeyboardDidDisconnect),
      name: .GCKeyboardDidDisconnect,
      object: nil
    )

    sendHardwareState()

    return nil
  }
  
  func onCancel(withArguments arguments: Any?) -> FlutterError? {
    NotificationCenter.default.removeObserver(self)
    
    events = nil

    return nil
  }
  
  @objc private func keyboardWillChangeFrame(_ notification: Notification) {
    guard let keyboardFrame = notification.userInfo?[UIResponder.keyboardFrameEndUserInfoKey] as? CGRect else {
      return
    }

    let height = keyboardVisibleHeight(from: keyboardFrame)
    guard height > 0 else {
      return
    }

    sendKeyboardHeight(height)
  }
  
  @objc private func keyboardWillHide(_ notification: Notification) {
    sendKeyboardHeight(0)
  }
  
  @objc private func hardwareKeyboardDidConnect(_ notification: Notification) {
    sendHardwareState()
  }

  @objc private func hardwareKeyboardDidDisconnect(_ notification: Notification) {
    sendHardwareState()
  }

  private func sendHardwareState() {
    let isHardwareKeyboard = resolveHardwareKeyboardState()
    sendEvent(["type": "hardware", "hardware": isHardwareKeyboard])
  }

  private func resolveHardwareKeyboardState() -> Bool {
    if let isInHardwareKeyboardMode = detectHardwareKeyboardModeFromUIKeyboardImpl() {
      return isInHardwareKeyboardMode
    }

    return GCKeyboard.coalesced != nil
  }

  private func detectHardwareKeyboardModeFromUIKeyboardImpl() -> Bool? {
    guard let cls = NSClassFromString("UIKeyboardImpl") as? NSObject.Type,
          let instance = cls.perform(NSSelectorFromString("activeInstance"))?.takeUnretainedValue() as? NSObject else {
      return nil
    }

    let selector = NSSelectorFromString("isInHardwareKeyboardMode")
    guard instance.responds(to: selector) else {
      return nil
    }

    typealias BoolMethod = @convention(c) (AnyObject, Selector) -> Bool
    let methodImplementation = instance.method(for: selector)
    let function = unsafeBitCast(methodImplementation, to: BoolMethod.self)
    return function(instance, selector)
  }

  private func sendKeyboardHeight(_ height: Double) {
    sendEvent(["type": "height", "height": height])
    sendHardwareState()
  }

  private func keyboardVisibleHeight(from keyboardFrame: CGRect) -> Double {
    if let keyWindow = currentKeyWindow() {
      let frameInWindow = keyWindow.convert(keyboardFrame, from: nil)
      let overlap = keyWindow.bounds.intersection(frameInWindow)
      return Double(max(0, overlap.height))
    }

    let overlap = UIScreen.main.bounds.intersection(keyboardFrame)
    return Double(max(0, overlap.height))
  }

  private func currentKeyWindow() -> UIWindow? {
    return UIApplication.shared.connectedScenes
      .compactMap { $0 as? UIWindowScene }
      .flatMap { $0.windows }
      .first { $0.isKeyWindow }
  }

  private func sendEvent(_ event: [String: Any]) {
    events?(event)
  }
}
  
