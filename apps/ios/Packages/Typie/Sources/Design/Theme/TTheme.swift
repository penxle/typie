import SwiftUI

public struct TTheme: Sendable {
  public let colors: TColors
  public let shadows: TShadows
  public let mode: ResolvedThemeMode

  public static let light = TTheme(colors: .light, shadows: .light, mode: .light)
  public static let dark = TTheme(colors: .dark, shadows: .dark, mode: .dark)
}

private struct TThemeKey: EnvironmentKey {
  static let defaultValue = TTheme.light
}

extension EnvironmentValues {
  public var theme: TTheme {
    get { self[TThemeKey.self] }
    set { self[TThemeKey.self] = newValue }
  }
}

private struct TThemeModifier: ViewModifier {
  @Environment(\.colorScheme) private var colorScheme

  func body(content: Content) -> some View {
    content.environment(\.theme, colorScheme == .dark ? TTheme.dark : TTheme.light)
  }
}

extension View {
  public func themed() -> some View {
    modifier(TThemeModifier())
  }
}
