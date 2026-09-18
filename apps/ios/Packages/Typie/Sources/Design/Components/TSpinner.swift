import SwiftUI

public struct TSpinner: View {
  @State private var rotating = false

  private let color: Color
  private let fixedSize: CGFloat?
  @ScaledMetric private var scaledSize: CGFloat

  private static let tail: Double = 280
  private static let headAngle = Angle.degrees(tail)

  public init(color: Color, size: CGFloat = 16, relativeTo style: TTextStyle? = nil) {
    self.color = color
    fixedSize = style == nil ? size : nil
    _scaledSize = ScaledMetric(wrappedValue: size, relativeTo: style?.textStyle ?? .body)
  }

  private var size: CGFloat { fixedSize ?? scaledSize }

  private var lineWidth: CGFloat { (size * 0.1 * 2).rounded() / 2 }

  private var gradient: AngularGradient {
    let head = Self.tail / 360
    return AngularGradient(
      stops: [
        .init(color: color.opacity(0), location: 0),
        .init(color: color.opacity(0.08), location: head * 0.1),
        .init(color: color.opacity(0.30), location: head * 0.45),
        .init(color: color.opacity(0.62), location: head * 0.8),
        .init(color: color, location: head),
        .init(color: color.opacity(0), location: head),
        .init(color: color.opacity(0), location: 1),
      ],
      center: .center)
  }

  public var body: some View {
    let radius = (size - lineWidth) / 2
    ZStack {
      Circle()
        .trim(from: 0, to: Self.tail / 360)
        .stroke(gradient, style: StrokeStyle(lineWidth: lineWidth, lineCap: .butt))
        .padding(lineWidth / 2)
      Circle()
        .fill(color)
        .frame(width: lineWidth, height: lineWidth)
        .offset(
          x: radius * cos(Self.headAngle.radians), y: radius * sin(Self.headAngle.radians))
    }
    .frame(width: size, height: size)
    .rotationEffect(.degrees(270 - Self.tail))
    .rotationEffect(.degrees(rotating ? 360 : 0))
    .animation(.linear(duration: 0.9).repeatForever(autoreverses: false), value: rotating)
    .onAppear { rotating = true }
  }
}
