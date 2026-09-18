import Testing

@testable import Design

@Suite struct TypographyTests {
  @Test func fontsRegisterAndMeasure() {
    TFonts.registerAll()
    #expect(TTypography.body.resolvedFontName == "SUIT-Regular")
    #expect(TTypography.display.resolvedFontName == "SUIT-SemiBold")
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
