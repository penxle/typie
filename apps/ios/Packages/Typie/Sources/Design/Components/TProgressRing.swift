import SwiftUI

public struct TProgressRing: View {
  public enum RingState: Sendable {
    case under
    case achieved
    case noGoal
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
    switch state {
    case .under: min(max(progress, 0), 1)
    case .achieved: 1
    case .noGoal: 0
    }
  }

  static func dotWidth(for size: CGFloat) -> CGFloat {
    min(lineWidth(for: size), Self.maxDotWidth)
  }

  static func dash(for size: CGFloat) -> [CGFloat] {
    let circumference = .pi * (size - lineWidth(for: size))
    let dot = dotWidth(for: size)
    let count = max(6, (circumference / (dot * 2)).rounded(.down))
    return [0.001, circumference / count - 0.001]
  }

  private static let maxDotWidth: CGFloat = 4

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
    let dashed = state == .noGoal
    return ZStack {
      Circle().stroke(
        theme.colors.borderEmphasis,
        style: StrokeStyle(
          lineWidth: dashed ? Self.dotWidth(for: size) : width, lineCap: dashed ? .round : .butt,
          dash: dashed ? Self.dash(for: size) : []
        ))
      if !dashed {
        Circle()
          .trim(from: 0, to: fraction)
          .stroke(
            fill, style: StrokeStyle(lineWidth: width, lineCap: fraction > 0 ? .round : .butt)
          )
          .rotationEffect(.degrees(-90))
      }
    }
    .padding(width / 2)
    .frame(width: size, height: size)
    .animation(reduceMotion ? nil : .easeOut(duration: 0.3), value: fraction)
    .accessibilityHidden(true)
  }
}
