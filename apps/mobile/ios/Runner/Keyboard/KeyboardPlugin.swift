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
  
  @objc private func keyboardWillShow(_ notification: Notification) {
    if let keyboardFrame = notification.userInfo?[UIResponder.keyboardFrameEndUserInfoKey] as? CGRect {
      let keyboardHeight = keyboardFrame.height
      sendEvent(["type": "height", "height": Double(keyboardHeight)])
      sendHardwareState()
    }
  }
  
  @objc private func keyboardWillHide(_ notification: Notification) {
    sendEvent(["type": "height", "height": 0.0])
    sendHardwareState()
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

  private func sendEvent(_ event: [String: Any]) {
    events?(event)
  }
}
  
