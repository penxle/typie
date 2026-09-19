import SwiftUI

public struct TIcon: View {
  @Environment(\.theme) private var theme
  private var colors: TColors { theme.colors }

  private let name: TIconName
  private let fixedSize: CGFloat?
  @ScaledMetric private var scaledSize: CGFloat
  private let tint: Color?
  private let label: String?

  public init(
    _ name: TIconName, size: CGFloat = 24, tint: Color? = nil, label: String? = nil,
    relativeTo style: TTextStyle? = nil
  ) {
    self.name = name
    fixedSize = style == nil ? size : nil
    _scaledSize = ScaledMetric(wrappedValue: size, relativeTo: style?.textStyle ?? .body)
    self.tint = tint
    self.label = label
  }

  private var side: CGFloat { fixedSize ?? scaledSize }

  private var image: Image {
    if let label {
      Image(name.assetName, bundle: .module, label: Text(label))
    } else {
      Image(decorative: name.assetName, bundle: .module)
    }
  }

  public var body: some View {
    image
      .renderingMode(.template)
      .resizable()
      .frame(width: side, height: side)
      .foregroundStyle(tint ?? colors.textDefault)
  }
}
