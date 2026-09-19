import Testing

@testable import Design

@Suite struct TThemeModeTests {
  @Test func rawValuesAreLowercase() {
    #expect(TThemeMode.system.rawValue == "system")
    #expect(TThemeMode(rawValue: "dark") == .dark)
  }
}
