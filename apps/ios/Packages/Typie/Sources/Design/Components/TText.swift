import SwiftUI

public struct TText: View {
  @Environment(\.theme) private var theme
  @Environment(\.redactionReasons) private var redactionReasons
  private var colors: TColors { theme.colors }

  private static let placeholderRadius: CGFloat = 4

  private let text: String
  private var metrics: TTextMetrics
  private let color: Color?
  private let alignment: TextAlignment
  private let maxLines: Int?
  private let monospacedDigit: Bool

  public init(
    _ text: String,
    style: TTextStyle = TTypography.text,
    color: Color? = nil,
    alignment: TextAlignment = .leading,
    maxLines: Int? = nil,
    monospacedDigit: Bool = false
  ) {
    self.text = text
    metrics = TTextMetrics(style)
    self.color = color
    self.alignment = alignment
    self.maxLines = maxLines
    self.monospacedDigit = monospacedDigit
  }

  public var body: some View {
    if redactionReasons.contains(.placeholder) {
      label
        .unredacted()
        .hidden()
        .overlay(RoundedRectangle(cornerRadius: Self.placeholderRadius).fill(colors.surfaceInset))
    } else {
      label
    }
  }

  private var label: some View {
    let extra = metrics.extraLineSpacing
    let base = Text(text)
    return (monospacedDigit ? base.monospacedDigit() : base)
      .font(metrics.style.font)
      .foregroundStyle(color ?? colors.textDefault)
      .multilineTextAlignment(alignment)
      .lineLimit(maxLines)
      .lineSpacing(extra)
      .padding(.vertical, extra / 2)
  }
}
