import Testing

@testable import Core

@MainActor
struct AppLifecycleTests {
  @Test func startsInTheBackgroundAtGenerationZero() {
    let lifecycle = AppLifecycle()
    #expect(lifecycle.state == .background)
    #expect(lifecycle.foregroundGeneration == 0)
  }

  @Test func theFirstForegroundKeepsTheGeneration() {
    let lifecycle = AppLifecycle()
    lifecycle.update(foreground: true)
    #expect(lifecycle.state == .foreground)
    #expect(lifecycle.foregroundGeneration == 0)
  }

  @Test func returningToTheForegroundAdvancesTheGeneration() {
    let lifecycle = AppLifecycle()
    lifecycle.update(foreground: true)
    lifecycle.update(foreground: false)
    lifecycle.update(foreground: true)
    #expect(lifecycle.state == .foreground)
    #expect(lifecycle.foregroundGeneration == 1)
  }

  @Test func repeatedStatesAreIgnored() {
    let lifecycle = AppLifecycle()
    lifecycle.update(foreground: true)
    lifecycle.update(foreground: true)
    lifecycle.update(foreground: false)
    lifecycle.update(foreground: false)
    lifecycle.update(foreground: true)
    lifecycle.update(foreground: true)
    #expect(lifecycle.foregroundGeneration == 1)
  }
}
