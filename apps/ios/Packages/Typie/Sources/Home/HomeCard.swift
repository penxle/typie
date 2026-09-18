#if canImport(UIKit)

  import Design
  import SwiftUI

  public struct HomeCard<Content: View>: View {
    @Environment(\.theme) private var theme

    private let content: Content

    public init(@ViewBuilder content: () -> Content) {
      self.content = content()
    }

    static var tintOpacity: Double { 0.6 }

    public var body: some View {
      VStack(spacing: 0) { content }
        .padding(.horizontal, 10)
        .background(
          theme.colors.surfaceDefault.opacity(Self.tintOpacity), in: TShapes.rounded(TShapes.lg))
    }
  }

#endif
