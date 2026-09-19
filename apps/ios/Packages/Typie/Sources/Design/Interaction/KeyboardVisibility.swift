#if canImport(UIKit)

  import SwiftUI
  import UIKit

  private struct KeyboardVisibilityModifier: ViewModifier {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    @Binding var visible: Bool

    func body(content: Content) -> some View {
      content
        .onReceive(
          NotificationCenter.default.publisher(for: UIResponder.keyboardWillShowNotification)
        ) {
          setVisible(true, notification: $0)
        }
        .onReceive(
          NotificationCenter.default.publisher(for: UIResponder.keyboardWillHideNotification)
        ) {
          setVisible(false, notification: $0)
        }
    }

    private func setVisible(_ value: Bool, notification: Notification) {
      let info = notification.userInfo
      guard info?[UIResponder.keyboardIsLocalUserInfoKey] as? Bool != false else { return }
      guard visible != value else { return }
      let duration = info?[UIResponder.keyboardAnimationDurationUserInfoKey] as? Double ?? 0.25
      withAnimation(reduceMotion ? nil : .easeOut(duration: duration)) {
        visible = value
      }
    }
  }

  extension View {
    public func observingKeyboard(_ visible: Binding<Bool>) -> some View {
      modifier(KeyboardVisibilityModifier(visible: visible))
    }
  }

#endif
