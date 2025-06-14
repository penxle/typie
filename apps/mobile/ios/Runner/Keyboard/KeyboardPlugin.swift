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
    
    #if targetEnvironment(simulator)
      events(["type": "hardware", "hardware": false])
    #else
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
    
      events(["type": "hardware", "hardware": GCKeyboard.coalesced != nil])
    #endif

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
      events!(["type": "height", "height": Double(keyboardHeight)])
    }
  }
  
  @objc private func keyboardWillHide(_ notification: Notification) {
    events!(["type": "height", "height": 0.0])
  }
  
  @objc private func hardwareKeyboardDidConnect(_ notification: Notification) {
    events!(["type": "hardware", "hardware": true])
  }

  @objc private func hardwareKeyboardDidDisconnect(_ notification: Notification) {
    events!(["type": "hardware", "hardware": false])
  }
}
  
