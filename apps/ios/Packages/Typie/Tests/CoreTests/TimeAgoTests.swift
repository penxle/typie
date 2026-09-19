import Foundation
import Testing

@testable import Core

@Suite struct TimeAgoTests {
  private let now = Date(timeIntervalSince1970: 1_800_000_000)

  private func ago(_ seconds: TimeInterval) -> String {
    timeAgo(now.addingTimeInterval(-seconds), now: now)
  }

  @Test func underAMinuteIsJustNow() {
    #expect(ago(0) == "방금")
    #expect(ago(59) == "방금")
    #expect(timeAgo(now.addingTimeInterval(30), now: now) == "방금")
  }

  @Test func minutesHoursDays() {
    #expect(ago(60) == "1분 전")
    #expect(ago(59 * 60) == "59분 전")
    #expect(ago(3600) == "1시간 전")
    #expect(ago(23 * 3600) == "23시간 전")
    #expect(ago(86400) == "1일 전")
    #expect(ago(29 * 86400) == "29일 전")
  }

  @Test func monthsAndYears() {
    #expect(ago(30 * 86400) == "1개월 전")
    #expect(ago(364 * 86400) == "12개월 전")
    #expect(ago(365 * 86400) == "1년 전")
    #expect(ago(800 * 86400) == "2년 전")
  }

  @Test func futureUsesAfter() {
    #expect(timeAgo(now.addingTimeInterval(2 * 3600), now: now) == "2시간 후")
  }
}
