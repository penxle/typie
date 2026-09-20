import SwiftUI

private struct PressTriggerKey: EnvironmentKey {
  static let defaultValue = 0
}

extension EnvironmentValues {
  var pressTrigger: Int {
    get { self[PressTriggerKey.self] }
    set { self[PressTriggerKey.self] = newValue }
  }
}

private struct PressEffect: ViewModifier {
  let isPressed: Bool
  let scale: CGFloat

  @Environment(\.pressTrigger) private var trigger
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  @State private var heldAt: ContinuousClock.Instant?

  private let minimumHold: Duration = .milliseconds(120)
  private var shown: Bool { isPressed || heldAt != nil }

  func body(content: Content) -> some View {
    content
      .scaleEffect(shown ? scale : 1)
      .animation(
        reduceMotion ? nil : (shown ? .easeOut(duration: 0.05) : .smooth(duration: 0.3)),
        value: shown
      )
      .onChange(of: isPressed) { _, pressed in
        if pressed { heldAt = .now }
      }
      .onChange(of: trigger) {
        if !shown { heldAt = .now }
      }
      .task(id: heldAt) {
        guard heldAt != nil else { return }
        guard (try? await Task.sleep(for: minimumHold)) != nil else { return }
        heldAt = nil
      }
  }
}

extension View {
  public func pressEffect(_ isPressed: Bool, scale: CGFloat = 0.98) -> some View {
    modifier(PressEffect(isPressed: isPressed, scale: scale))
  }
}
