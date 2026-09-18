import SwiftUI

public struct TIcon: View {
  @Environment(\.theme) private var theme
  private var colors: TColors { theme.colors }

  private let name: TIconName
  private let size: CGFloat
  private let tint: Color?
  private let label: String?

  public init(_ name: TIconName, size: CGFloat = 24, tint: Color? = nil, label: String? = nil) {
    self.name = name
    self.size = size
    self.tint = tint
    self.label = label
  }

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
      .frame(width: size, height: size)
      .foregroundStyle(tint ?? colors.textDefault)
  }
}
