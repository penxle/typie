import SwiftUI

public struct TProgressRing: View {
  public enum RingState: Sendable {
    case under
    case achieved
  }

  @Environment(\.theme) private var theme
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  @Environment(\.redactionReasons) private var redactionReasons

  private let progress: Double
  private let state: RingState
  private let fixedSize: CGFloat?
  @ScaledMetric private var scaledSize: CGFloat

  public init(
    progress: Double, state: RingState, size: CGFloat, relativeTo style: TTextStyle? = nil
  ) {
    self.progress = progress
    self.state = state
    fixedSize = style == nil ? size : nil
    _scaledSize = ScaledMetric(wrappedValue: size, relativeTo: style?.textStyle ?? .body)
  }

  private var size: CGFloat { fixedSize ?? scaledSize }

  static func fraction(progress: Double, state: RingState) -> Double {
    state == .under ? min(max(progress, 0), 1) : 1
  }

  static func lineWidth(for size: CGFloat) -> CGFloat {
    max(2, size * 3.5 / 32)
  }

  public var body: some View {
    if redactionReasons.contains(.placeholder) {
      Circle()
        .fill(theme.colors.surfaceInset)
        .frame(width: size, height: size)
    } else {
      ring
    }
  }

  private var ring: some View {
    let width = Self.lineWidth(for: size)
    let fraction = Self.fraction(progress: progress, state: state)
    let fill = state == .under ? theme.colors.accentDefault : theme.colors.successDefault
    return ZStack {
      Circle().stroke(theme.colors.surfaceInset, lineWidth: width)
      Circle()
        .trim(from: 0, to: fraction)
        .stroke(fill, style: StrokeStyle(lineWidth: width, lineCap: fraction > 0 ? .round : .butt))
        .rotationEffect(.degrees(-90))
    }
    .padding(width / 2)
    .frame(width: size, height: size)
    .animation(reduceMotion ? nil : .easeOut(duration: 0.3), value: fraction)
    .accessibilityHidden(true)
  }
}
