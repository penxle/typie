import Foundation
import GameController
import UIKit

@MainActor @objcMembers public final class EditorKeyboardBridge: NSObject {
  public static func isInHardwareKeyboardMode() -> Bool {
    if let isInHardwareKeyboardMode = detectHardwareKeyboardModeFromUIKeyboardImpl() {
      return isInHardwareKeyboardMode
    }

    return GCKeyboard.coalesced != nil
  }

  public static func isImeFrameVisible(notification: Notification) -> Bool {
    guard
      let keyboardFrame =
        notification.userInfo?[UIResponder.keyboardFrameEndUserInfoKey] as? CGRect
    else {
      return false
    }

    return keyboardVisibleHeight(from: keyboardFrame) > 0
  }

  private static func detectHardwareKeyboardModeFromUIKeyboardImpl() -> Bool? {
    guard
      let cls = NSClassFromString("UIKeyboardImpl") as? NSObject.Type,
      let instance =
        cls.perform(NSSelectorFromString("activeInstance"))?
        .takeUnretainedValue() as? NSObject
    else {
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

  private static func keyboardVisibleHeight(from keyboardFrame: CGRect) -> Double {
    if let keyWindow = currentKeyWindow() {
      let frameInWindow = keyWindow.convert(keyboardFrame, from: nil)
      let overlap = keyWindow.bounds.intersection(frameInWindow)
      return Double(max(0, overlap.height))
    }

    let overlap = UIScreen.main.bounds.intersection(keyboardFrame)
    return Double(max(0, overlap.height))
  }

  private static func currentKeyWindow() -> UIWindow? {
    return UIApplication.shared.connectedScenes
      .compactMap { $0 as? UIWindowScene }
      .flatMap { $0.windows }
      .first { $0.isKeyWindow }
  }
}
