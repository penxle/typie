import Testing

@testable import Design

@MainActor @Suite struct TProgressRingTests {
  @Test func underClampsProgress() {
    #expect(TProgressRing.fraction(progress: 0.4, state: .under) == 0.4)
    #expect(TProgressRing.fraction(progress: -1, state: .under) == 0)
    #expect(TProgressRing.fraction(progress: 3, state: .under) == 1)
  }

  @Test func achievedIsAlwaysFull() {
    #expect(TProgressRing.fraction(progress: 0.4, state: .achieved) == 1)
    #expect(TProgressRing.fraction(progress: 0, state: .achieved) == 1)
  }

  @Test func noGoalHasNoFillAndEvenDots() {
    #expect(TProgressRing.fraction(progress: 0.7, state: .noGoal) == 0)
    let dash = TProgressRing.dash(for: 32)
    let circumference = Double.pi * (32 - TProgressRing.lineWidth(for: 32))
    let count = (circumference / (dash[0] + dash[1])).rounded()
    #expect(dash.count == 2)
    #expect(abs(count * (dash[0] + dash[1]) - circumference) < 0.001)
  }

  @Test func lineWidthScalesWithFloor() {
    #expect(TProgressRing.lineWidth(for: 32) == 3.5)
    #expect(TProgressRing.lineWidth(for: 16) == 2)
    #expect(TProgressRing.lineWidth(for: 72) == 7.875)
  }
}
