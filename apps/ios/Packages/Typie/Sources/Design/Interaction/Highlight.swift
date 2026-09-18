import SwiftUI

extension View {
  public func highlight<S: Shape>(
    hovered: Bool, isPressed: Bool, enabled: Bool = true, hoverColor: Color, pressedColor: Color,
    in shape: S
  ) -> some View {
    overlay(enabled && hovered ? (isPressed ? pressedColor : hoverColor) : .clear, in: shape)
  }
}
