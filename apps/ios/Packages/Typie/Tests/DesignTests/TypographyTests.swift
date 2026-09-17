import CoreText
import Testing

@testable import Design

@Suite struct TypographyTests {
  private func resolvedFontName(_ style: TTextStyle) -> String {
    CTFontCopyPostScriptName(CTFontCreateWithName(style.fontName as CFString, style.size, nil))
      as String
  }

  @Test func fontsRegisterAndMeasure() {
    TFonts.registerAll()
    #expect(resolvedFontName(TTypography.body) == "SUIT-Regular")
    #expect(resolvedFontName(TTypography.display) == "SUIT-SemiBold")
    #expect(TTypography.body.naturalLineHeight > 0)
  }

  @Test func lineHeightsExceedNaturalMetrics() {
    TFonts.registerAll()
    let styles = [
      TTypography.display, TTypography.heading, TTypography.title, TTypography.label,
      TTypography.body, TTypography.action, TTypography.caption, TTypography.micro,
    ]
    for style in styles {
      #expect(style.naturalLineHeight < style.lineHeight)
    }
  }
}
