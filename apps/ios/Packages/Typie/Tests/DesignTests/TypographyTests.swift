import CoreText
import SwiftUI
import Testing

@testable import Design

@Suite struct TypographyTests {
  private static let tokens: [(TTextStyle, CGFloat, TFontWeight, CGFloat, Font.TextStyle)] = [
    (TTypography.hero, 28, .semibold, 36, .title),
    (TTypography.heading, 22, .semibold, 28, .title2),
    (TTypography.lead, 20, .semibold, 26, .title3),
    (TTypography.title, 17, .semibold, 22, .headline),
    (TTypography.text, 16, .regular, 24, .callout),
    (TTypography.label, 15, .semibold, 20, .subheadline),
    (TTypography.control, 15, .medium, 20, .subheadline),
    (TTypography.detail, 14, .regular, 20, .subheadline),
    (TTypography.caption, 13, .regular, 18, .footnote),
    (TTypography.section, 13, .bold, 18, .footnote),
    (TTypography.meta, 12, .regular, 16, .caption),
    (TTypography.fine, 11, .regular, 16, .caption2),
  ]

  private func resolvedFontName(_ style: TTextStyle) -> String {
    CTFontCopyPostScriptName(CTFontCreateWithName(style.fontName as CFString, style.size, nil))
      as String
  }

  @Test func tokenTableMatchesSpec() {
    for (style, size, weight, lineHeight, textStyle) in Self.tokens {
      #expect(style.size == size)
      #expect(style.weight == weight)
      #expect(style.lineHeight == lineHeight)
      #expect(style.textStyle == textStyle)
    }
  }

  @Test func fontsRegisterAndMeasure() {
    TFonts.registerAll()
    #expect(resolvedFontName(TTypography.text) == "SUIT-Regular")
    #expect(resolvedFontName(TTypography.control) == "SUIT-Medium")
    #expect(resolvedFontName(TTypography.hero) == "SUIT-SemiBold")
    #expect(resolvedFontName(TTypography.section) == "SUIT-Bold")
    #expect(TTypography.text.naturalLineHeight(atSize: 16) > 0)
  }

  @Test func lineHeightsExceedNaturalMetrics() {
    TFonts.registerAll()
    for (style, _, _, _, _) in Self.tokens {
      #expect(style.naturalLineHeight(atSize: style.size) < style.lineHeight)
    }
  }

  @Test func extraLineSpacingFollowsScaledInputs() {
    TFonts.registerAll()
    let style = TTypography.text
    let base = style.extraLineSpacing(atSize: 16, lineHeight: 24)
    let doubled = style.extraLineSpacing(atSize: 32, lineHeight: 48)
    #expect(base > 0)
    #expect(doubled > base)
    #expect(style.extraLineSpacing(atSize: 32, lineHeight: 30) == 0)
  }
}
