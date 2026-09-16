import Testing

@testable import Design

@Suite struct ThemeModeTests {
  @Test func rawValuesAreLowercase() {
    #expect(ThemeMode.system.rawValue == "system")
    #expect(ThemeMode(rawValue: "dark") == .dark)
  }
}
