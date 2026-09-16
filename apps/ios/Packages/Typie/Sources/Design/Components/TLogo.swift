import SwiftUI

public struct TLogo: View {
  @Environment(\.theme) private var theme
  private var colors: TColors { theme.colors }

  private let height: CGFloat
  private let color: Color?

  public init(height: CGFloat = 32, color: Color? = nil) {
    self.height = height
    self.color = color
  }

  public var body: some View {
    Image(decorative: "logo-full", bundle: .module)
      .renderingMode(.template)
      .resizable()
      .aspectRatio(208.0 / 148.0, contentMode: .fit)
      .frame(height: height)
      .foregroundStyle(color ?? colors.textDefault)
  }
}
