import Foundation
import GameController
import ObjectiveC.runtime
import UIKit

@MainActor @objcMembers public final class EditorKeyboardBridge: NSObject {
  public static func isInHardwareKeyboardMode() -> Bool {
    if let isInHardwareKeyboardMode = detectHardwareKeyboardModeFromUIKeyboardImpl() {
      return isInHardwareKeyboardMode
    }

    return GCKeyboard.coalesced != nil
  }

  public static func endInputMethodComposition() -> Bool {
    guard let keyboard = activeKeyboard() else {
      return false
    }

    let selector = NSSelectorFromString("acceptAutocorrectionAndEndComposition")
    guard
      keyboard.responds(to: selector),
      let method = class_getInstanceMethod(type(of: keyboard), selector),
      method_getNumberOfArguments(method) == 2
    else {
      return false
    }
    let returnType = method_copyReturnType(method)
    defer { free(returnType) }
    guard String(cString: returnType) == "v" else {
      return false
    }

    typealias VoidMethod = @convention(c) (AnyObject, Selector) -> Void
    let function = unsafeBitCast(method_getImplementation(method), to: VoidMethod.self)
    function(keyboard, selector)
    return true
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

  public static func imeVisibleHeight(notification: Notification) -> Double {
    guard
      let keyboardFrame =
        notification.userInfo?[UIResponder.keyboardFrameEndUserInfoKey] as? CGRect
    else {
      return 0
    }

    return keyboardVisibleHeight(from: keyboardFrame)
  }

  private static func detectHardwareKeyboardModeFromUIKeyboardImpl() -> Bool? {
    guard let instance = activeKeyboard() else {
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

  private static func activeKeyboard() -> NSObject? {
    guard let cls = NSClassFromString("UIKeyboardImpl") as? NSObject.Type else {
      return nil
    }

    let selector = NSSelectorFromString("activeInstance")
    guard cls.responds(to: selector) else {
      return nil
    }
    return cls.perform(selector)?.takeUnretainedValue() as? NSObject
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
