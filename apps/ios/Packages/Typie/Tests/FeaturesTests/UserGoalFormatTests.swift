import Core
import Testing

@testable import Features

@Suite struct UserGoalFormatTests {
  @Test func groupsThousandsWithComma() {
    #expect(Formatting.comma(0) == "0")
    #expect(Formatting.comma(1234) == "1,234")
    #expect(Formatting.comma(1_234_567) == "1,234,567")
    #expect(UserGoalFormat.characters(1234) == "1,234자")
  }

  @Test func labelsDayWithKoreanWeekday() {
    #expect(UserGoalFormat.dayLabel(KSTDay(year: 2026, month: 9, day: 20)) == "9월 20일 일")
    #expect(UserGoalFormat.dayLabel(KSTDay(year: 2026, month: 1, day: 3)) == "1월 3일 토")
  }
}
