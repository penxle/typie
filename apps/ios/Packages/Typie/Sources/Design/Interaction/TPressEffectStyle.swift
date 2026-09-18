import SwiftUI

public struct TPressEffectStyle<Look: ButtonStyle>: PrimitiveButtonStyle {
  private let look: Look

  public init(_ look: Look) {
    self.look = look
  }

  public func makeBody(configuration: Configuration) -> some View {
    PressEffectBody(configuration: configuration, look: look)
  }

  private struct PressEffectBody: View {
    let configuration: Configuration
    let look: Look
    @State private var taps = 0

    var body: some View {
      Button {
        taps += 1
        configuration.trigger()
      } label: {
        configuration.label
      }
      .buttonStyle(look)
      .environment(\.pressTrigger, taps)
    }
  }
}
