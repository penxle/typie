import Testing

@testable import Features

@Suite struct UserGoalLineTests {
  private func makeStatus(additions: Int, target: Int, streak: Int) -> UserGoalStatus {
    UserGoalStatus(
      additions: additions, target: target, achieved: false, remaining: target - additions,
      streak: streak, bestStreak: streak)
  }

  @Test func textWithStreak() {
    let status = makeStatus(additions: 186, target: 300, streak: 12)
    #expect(UserGoalLineCopy.text(status) == "오늘 186 / 300자 · 12일 연속")
    #expect(UserGoalLineCopy.accessibilityText(status) == "일일 목표, 오늘 300자 중 186자, 12일 연속")
  }

  @Test func textWithoutStreak() {
    let status = makeStatus(additions: 186, target: 300, streak: 0)
    #expect(UserGoalLineCopy.text(status) == "오늘 186 / 300자")
    #expect(UserGoalLineCopy.accessibilityText(status) == "일일 목표, 오늘 300자 중 186자")
    let grouped = makeStatus(additions: 1234, target: 2000, streak: 0)
    #expect(UserGoalLineCopy.text(grouped) == "오늘 1,234 / 2,000자")
  }

  @Test func textWithoutGoal() {
    #expect(UserGoalLineCopy.accessibilityText(nil) == "일일 목표, 설정된 목표가 없어요")
  }
}
