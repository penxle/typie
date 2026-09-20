import SwiftUI

public struct TPressLook: ButtonStyle {
  private let scale: CGFloat

  public init(scale: CGFloat = 0.98) {
    self.scale = scale
  }

  public func makeBody(configuration: Configuration) -> some View {
    configuration.label.pressEffect(configuration.isPressed, scale: scale)
  }
}
