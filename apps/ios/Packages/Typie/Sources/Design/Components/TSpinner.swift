import SwiftUI

public struct TSpinner: View {
  @State private var rotating = false

  private let color: Color
  private let size: CGFloat
  private let strokeWidth: CGFloat
  private let sweepAngle: Double

  public init(
    color: Color, size: CGFloat = 16, strokeWidth: CGFloat = 1.5, sweepAngle: Double = 220
  ) {
    self.color = color
    self.size = size
    self.strokeWidth = strokeWidth
    self.sweepAngle = sweepAngle
  }

  public var body: some View {
    Circle()
      .trim(from: 0, to: sweepAngle / 360)
      .stroke(color, style: StrokeStyle(lineWidth: strokeWidth, lineCap: .round))
      .frame(width: size, height: size)
      .rotationEffect(.degrees(rotating ? 360 : 0))
      .animation(.linear(duration: 1).repeatForever(autoreverses: false), value: rotating)
      .onAppear { rotating = true }
  }
}
