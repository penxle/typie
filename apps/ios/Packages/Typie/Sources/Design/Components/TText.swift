import SwiftUI

public struct TText: View {
  @Environment(\.theme) private var theme
  private var colors: TColors { theme.colors }

  private let text: String
  private let style: TTextStyle
  private let color: Color?
  private let alignment: TextAlignment
  private let maxLines: Int?

  public init(
    _ text: String,
    style: TTextStyle = TTypography.body,
    color: Color? = nil,
    alignment: TextAlignment = .leading,
    maxLines: Int? = nil
  ) {
    self.text = text
    self.style = style
    self.color = color
    self.alignment = alignment
    self.maxLines = maxLines
  }

  public var body: some View {
    let extra = style.extraLineSpacing
    Text(text)
      .font(style.font)
      .foregroundStyle(color ?? colors.textDefault)
      .multilineTextAlignment(alignment)
      .lineLimit(maxLines)
      .lineSpacing(extra)
      .padding(.vertical, extra / 2)
  }
}
