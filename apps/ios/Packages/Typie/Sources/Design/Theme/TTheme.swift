import SwiftUI

public struct TTheme: Sendable {
  public let colors: TColors
  public let shadows: TShadows
  public let mode: TResolvedThemeMode

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

private struct CanvasBackground: ViewModifier {
  @Environment(\.theme) private var theme

  func body(content: Content) -> some View {
    content.background(theme.colors.surfaceCanvas.ignoresSafeArea())
  }
}

private struct SheetBackground: ViewModifier {
  @Environment(\.theme) private var theme

  func body(content: Content) -> some View {
    content.background(
      theme.colors.surfaceDefault.opacity(TTheme.sheetTintOpacity).ignoresSafeArea())
  }
}

extension TTheme {
  public static let sheetTintOpacity: Double = 0.25
}

extension View {
  public func themed() -> some View {
    modifier(TThemeModifier())
  }

  public func sheetBackground() -> some View {
    modifier(SheetBackground())
  }

  public func canvasBackground() -> some View {
    modifier(CanvasBackground())
  }
}
